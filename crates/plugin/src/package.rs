//! Downloaded ZIP packages keep their complete resource tree in a private, versioned folder.
use std::{
    collections::HashSet,
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
};
const MAX_FILES: usize = 50000;
const MAX_FILE: u64 = 1024 * 1024 * 1024;
const MAX_TOTAL: u64 = 4 * 1024 * 1024 * 1024;
fn number(bytes: &[u8], at: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(bytes.get(at..at + 4).ok_or("truncated ZIP directory")?.try_into().map_err(|_| "invalid ZIP directory")?))
}
/// Bound the directory before handing the archive to the library. ZIP64/spanned archives
/// require manual unpacking; their potentially unbounded directory counts are not accepted.
fn preflight(file: &mut fs::File) -> Result<(), String> {
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    if !(22..=MAX_FILE).contains(&size) {
        return Err("ZIP downloads must be between 22 bytes and 1 GiB; unpack larger packages with the vendor's tools".into());
    }
    let length = size.min(65557) as usize;
    file.seek(SeekFrom::End(-(length as i64))).map_err(|e| e.to_string())?;
    let mut tail = vec![0; length];
    file.read_exact(&mut tail).map_err(|e| e.to_string())?;
    let at = (0..=length.saturating_sub(22))
        .rev()
        .find(|at| {
            tail.get(*at..*at + 4) == Some(&[0x50, 0x4b, 5, 6])
                && tail.get(*at + 20..*at + 22).is_some_and(|s| usize::from(u16::from_le_bytes([s[0], s[1]])) + *at + 22 == length)
        })
        .ok_or("ZIP end directory not found")?;
    let end = tail.get(at..).ok_or("invalid ZIP end directory")?;
    let count = u16::from_le_bytes(end.get(10..12).ok_or("missing ZIP count")?.try_into().map_err(|_| "invalid ZIP count")?) as usize;
    if end.get(4..8) != Some(&[0, 0, 0, 0])
        || end.get(8..10) != end.get(10..12)
        || count > MAX_FILES
        || count == 65535
        || at.checked_sub(20).is_some_and(|p| tail.get(p..p + 4) == Some(&[0x50, 0x4b, 6, 7]))
    {
        return Err("ZIP64, split archives or packages over 50,000 files require manual unpacking".into());
    }
    let directory = u64::from(number(end, 12)?);
    let offset = u64::from(number(end, 16)?);
    if directory > 64 * 1024 * 1024 || offset.checked_add(directory).is_none_or(|n| n > size) {
        return Err("ZIP directory is too large or invalid".into());
    }
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    Ok(())
}
fn relative(name: &str) -> Result<PathBuf, String> {
    if name.len() > 1024 || name.contains(['\0', ':', '<', '>', '"', '|', '?', '*']) {
        return Err("ZIP contains an unsupported Windows path".into());
    }
    let name = name.replace('\\', "/");
    let path = PathBuf::from(name);
    if path.as_os_str().is_empty() || path.components().any(|p| !matches!(p, Component::Normal(_))) {
        return Err("ZIP contains an absolute or parent-relative path".into());
    }
    for part in path.components() {
        let text = part.as_os_str().to_string_lossy();
        if text.ends_with(['.', ' ']) {
            return Err("ZIP contains a path with a trailing dot/space".into());
        }
        let stem = text.split('.').next().unwrap_or_default().to_ascii_uppercase();
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
            || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && stem.as_bytes().get(3).is_some_and(|c| (b'1'..=b'9').contains(c)))
        {
            return Err("ZIP contains a reserved Windows device path".into());
        }
    }
    Ok(path)
}
pub(crate) fn unpack(source: &Path, packages: &Path, progress: &dyn Fn(u64, u64, &str) -> bool) -> Result<PathBuf, String> {
    let mut file = fs::File::open(source).map_err(|e| e.to_string())?;
    preflight(&mut file)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    if archive.len() > MAX_FILES {
        return Err("ZIP package contains too many files".into());
    }
    let mut total = 0u64;
    let mut names = HashSet::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let path = relative(entry.name())?;
        if entry.enclosed_name().is_none() || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("ZIP paths or symbolic links are unsupported".into());
        }
        if path.to_string_lossy().replace('\\', "/").to_ascii_lowercase().ends_with("csxs/manifest.xml") {
            return Err("This download is a CEP extension; a CEP runtime is not available in this build".into());
        }
        if !entry.is_dir() && !names.insert(path.to_string_lossy().to_lowercase()) {
            return Err("ZIP contains duplicate file paths".into());
        }
        total = total.checked_add(entry.size()).ok_or("ZIP size overflow")?;
        if entry.size() > MAX_FILE || total > MAX_TOTAL {
            return Err("ZIP package exceeds the 1 GiB per-file / 4 GiB unpacked limit".into());
        }
    }
    fs::create_dir_all(packages).map_err(|e| e.to_string())?;
    let parent = packages.parent().ok_or("package destination has no parent")?.canonicalize().map_err(|e| e.to_string())?;
    let packages = packages.canonicalize().map_err(|e| e.to_string())?;
    if !packages.starts_with(parent) {
        return Err("package destination resolves outside its settings folder".into());
    }
    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    let stem: String = stem.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_')).take(48).collect();
    let staging =
        tempfile::Builder::new().prefix(&format!("{}-", if stem.is_empty() { "package" } else { &stem })).tempdir_in(&packages).map_err(|e| e.to_string())?;
    let target = staging.path().canonicalize().map_err(|e| e.to_string())?;
    if !target.starts_with(&packages) {
        return Err("package staging directory is outside its destination".into());
    }
    let mut done = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let path = target.join(relative(entry.name())?);
        if entry.is_dir() {
            fs::create_dir_all(&path).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut output = fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|e| e.to_string())?;
        let expected = entry.size();
        let mut copied = 0u64;
        let mut buffer = [0u8; 65536];
        loop {
            if !progress(done, total, "Unpacking downloaded plugin package") {
                return Err("cancelled".into());
            }
            let n = entry.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            copied = copied.checked_add(n as u64).ok_or("ZIP size overflow")?;
            if copied > expected {
                return Err("ZIP entry exceeds its declared size".into());
            }
            output.write_all(buffer.get(..n).ok_or("invalid ZIP read length")?).map_err(|e| e.to_string())?;
            done += n as u64;
        }
        if copied != expected {
            return Err("ZIP entry is truncated".into());
        }
    }
    Ok(staging.keep())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn archive(path: &Path, files: &[(&str, &[u8])]) {
        let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
        for (name, bytes) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored)).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }
    #[test]
    fn preserves_resources_and_rejects_traversal_and_cep_packages() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("plugin.zip");
        archive(&source, &[("Vendor/plugin.aex", b"synthetic DLL"), ("Vendor/resources/data.txt", b"keep resources")]);
        let target = unpack(&source, &dir.path().join("packages"), &|_, _, _| true).unwrap();
        assert_eq!(fs::read(target.join("Vendor/resources/data.txt")).unwrap(), b"keep resources");
        archive(&source, &[("../outside.aex", b"bad")]);
        assert!(unpack(&source, &dir.path().join("packages"), &|_, _, _| true).is_err());
        archive(&source, &[("CSXS/manifest.xml", b"CEP")]);
        assert!(unpack(&source, &dir.path().join("packages"), &|_, _, _| true).unwrap_err().contains("CEP"));
        assert!(relative("C:/outside.dll").is_err());
        assert!(relative("Vendor/CON.dll").is_err());
    }
}
