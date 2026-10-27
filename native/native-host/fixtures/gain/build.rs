//! Emit an original PiPL as a minimal COFF resource object. No resource compiler is needed.
//! Format: Microsoft PE/COFF resource directory and IMAGE_REL_AMD64_ADDR32NB relocation.
use std::{env, fs, path::PathBuf};
fn word(b: &mut Vec<u8>, n: u16) {
    b.extend(n.to_le_bytes());
}
fn long(b: &mut Vec<u8>, n: u32) {
    b.extend(n.to_le_bytes());
}
fn property(b: &mut Vec<u8>, key: &[u8; 4], value: &[u8]) {
    b.extend(b"MIB8");
    b.extend(key.iter().rev());
    long(b, 0);
    long(b, value.len() as u32);
    b.extend(value);
    while !(b.len() - 10).is_multiple_of(4) {
        b.push(0);
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pipl = vec![1, 0];
    long(&mut pipl, 0);
    long(&mut pipl, 5);
    property(&mut pipl, b"kind", b"TKFe");
    property(&mut pipl, b"name", b"\x09Test Gain");
    property(&mut pipl, b"catg", b"\x05Tests");
    property(&mut pipl, b"eMNA", b"\x19org.effectcraft.test.gain");
    property(&mut pipl, b"8664", b"EffectMain\0");
    let mut data = Vec::new();
    // Type directory: one named entry, named PiPL.
    for n in [0u32, 0, 0, 1] {
        long(&mut data, n);
    } // last DWORD packs named=1, ids=0
    // Correct the packed counts: NumberOfNamedEntries is the low WORD.
    data[12..16].copy_from_slice(&1u32.to_le_bytes());
    long(&mut data, 0x80000000 | 88);
    long(&mut data, 0x80000000 | 24);
    // Resource ID directory.
    for n in [0u32, 0, 0, 0x00010000] {
        long(&mut data, n);
    }
    long(&mut data, 128);
    long(&mut data, 0x80000000 | 48);
    // Language directory.
    for n in [0u32, 0, 0, 0x00010000] {
        long(&mut data, n);
    }
    long(&mut data, 1033);
    long(&mut data, 72);
    // Data RVA gets a COFF relocation to piplblob at section offset 100.
    long(&mut data, 0);
    long(&mut data, pipl.len() as u32);
    long(&mut data, 0);
    long(&mut data, 0);
    word(&mut data, 4);
    for ch in "PIPL".encode_utf16() {
        word(&mut data, ch);
    }
    while data.len() < 100 {
        data.push(0);
    }
    data.extend(pipl);
    let raw_size = data.len() as u32;
    let reloc = 60 + raw_size;
    let symbols = reloc + 10;
    let mut obj = Vec::new();
    word(&mut obj, 0x8664);
    word(&mut obj, 1);
    long(&mut obj, 0);
    long(&mut obj, symbols);
    long(&mut obj, 1);
    word(&mut obj, 0);
    word(&mut obj, 0);
    obj.extend(b".rsrc\0\0\0");
    long(&mut obj, 0);
    long(&mut obj, 0);
    long(&mut obj, raw_size);
    long(&mut obj, 60);
    long(&mut obj, reloc);
    long(&mut obj, 0);
    word(&mut obj, 1);
    word(&mut obj, 0);
    long(&mut obj, 0x40300040);
    obj.extend(data);
    long(&mut obj, 72);
    long(&mut obj, 0);
    word(&mut obj, 3);
    obj.extend(b"piplblob");
    long(&mut obj, 100);
    word(&mut obj, 1);
    word(&mut obj, 0);
    obj.push(3);
    obj.push(0);
    long(&mut obj, 4);
    let path = PathBuf::from(env::var("OUT_DIR")?).join("fixture-resource.o");
    fs::write(&path, obj)?;
    println!("cargo:rustc-link-arg={}", path.display());
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
