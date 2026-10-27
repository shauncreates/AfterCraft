//! Bounded reader for Windows PiPL resource metadata; never reads executable instructions.
//! Windows representation: reserved u16, version/count u32 LE, then property records.
use serde::Serialize;

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct Descriptor {
    pub name: String,
    pub category: String,
    pub match_name: String,
    pub entry_point: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(len).ok_or("PiPL length overflow")?;
        let value = self.data.get(self.pos..end).ok_or("truncated PiPL resource")?;
        self.pos = end;
        Ok(value)
    }
    fn number(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().map_err(|_| "invalid u32")?))
    }
}
fn fourcc(bytes: &[u8]) -> String {
    bytes.iter().rev().map(|b| char::from(*b)).collect()
}
fn pstring(bytes: &[u8], key: &str, warnings: &mut Vec<String>) -> Result<String, String> {
    let len = usize::from(*bytes.first().ok_or("missing PiPL string length")?);
    let body = bytes.get(1..).ok_or("missing PiPL string body")?;
    let (text, repaired) = if let Some(declared) = body.get(..len) {
        if let Some(end) = declared.iter().position(|byte| *byte == 0) {
            let tail = declared.get(end..).ok_or("invalid PiPL string padding")?;
            if tail.iter().any(|byte| *byte != 0) {
                return Err("nonzero data after PiPL string terminator".into());
            }
            (declared.get(..end).ok_or("invalid PiPL string")?, true)
        } else {
            (declared, false)
        }
    } else {
        // Some resource generators count the prefix/alignment in the Pascal length.
        // Recover only a terminated string wholly contained in this bounded property.
        let end = body.iter().position(|byte| *byte == 0).ok_or("truncated PiPL string")?;
        if body.get(end..).ok_or("invalid PiPL string padding")?.iter().any(|byte| *byte != 0) {
            return Err("nonzero data after PiPL string terminator".into());
        }
        (body.get(..end).ok_or("invalid PiPL string")?, true)
    };
    if repaired {
        warnings.push(format!("{key}: repaired padded Pascal string length"));
    }
    // SDK legacy names may be local-codepage text. Entry-point symbols below must be ASCII.
    Ok(String::from_utf8_lossy(text).into_owned())
}
pub fn parse(data: &[u8]) -> Result<Descriptor, String> {
    if data.len() > 1024 * 1024 {
        return Err("PiPL exceeds 1 MiB".into());
    }
    let mut r = Reader { data, pos: 0 };
    if r.take(2)? != [1, 0] {
        return Err("unsupported Windows PiPL header".into());
    }
    if r.number()? != 0 {
        return Err("unsupported PiPL version".into());
    }
    let count = r.number()?;
    if count > 4096 {
        return Err("too many PiPL properties".into());
    }
    let mut d = Descriptor::default();
    for _ in 0..count {
        let vendor = fourcc(r.take(4)?);
        let key = fourcc(r.take(4)?);
        let _id = r.number()?;
        let len = r.number()? as usize;
        let value = r.take(len)?;
        // Adobe's resource compiler can exclude padding from the declared data length;
        // other generators include it. Both representations start the next record at 4-byte alignment.
        r.take((4 - len % 4) % 4)?;
        if vendor != "8BIM" {
            continue;
        }
        match key.as_str() {
            "kind" if value.len() == 4 => d.kind = fourcc(value),
            "name" => d.name = pstring(value, &key, &mut d.warnings)?,
            "catg" => d.category = pstring(value, &key, &mut d.warnings)?,
            "eMNA" => d.match_name = pstring(value, &key, &mut d.warnings)?,
            "8664" => {
                let end = value.iter().position(|b| *b == 0).ok_or("unterminated entry point")?;
                let symbol = value.get(..end).ok_or("invalid entry point")?;
                if symbol.is_empty() || !symbol.is_ascii() {
                    return Err("invalid x64 entry point".into());
                }
                d.entry_point = String::from_utf8(symbol.to_vec()).map_err(|e| e.to_string())?;
            }
            _ => {}
        }
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut b = vec![1, 0, 0, 0, 0, 0];
        let props: [(&str, &[u8]); 4] = [("kind", b"TKFe"), ("name", b"\x04Test\0\0\0"), ("8664", b"EffectMain\0\0"), ("eMNA", b"\x04demo\0\0\0")];
        b.extend((props.len() as u32).to_le_bytes());
        for (key, data) in props {
            b.extend(b"MIB8");
            b.extend(key.bytes().rev());
            b.extend(0u32.to_le_bytes());
            b.extend((data.len() as u32).to_le_bytes());
            b.extend(data);
        }
        b
    }
    #[test]
    fn extracts_windows_effect_metadata() {
        let d = parse(&fixture()).unwrap();
        assert_eq!((d.kind.as_str(), d.name.as_str(), d.entry_point.as_str(), d.match_name.as_str()), ("eFKT", "Test", "EffectMain", "demo"));
    }
    #[test]
    fn every_truncation_is_rejected() {
        let b = fixture();
        for len in 0..b.len() {
            assert!(parse(&b[..len]).is_err(), "length {len}");
        }
    }
    #[test]
    fn rejects_unbounded_counts_and_lengths() {
        let mut b = fixture();
        b[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&b).is_err());
        let mut b = fixture();
        b[22..26].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&b).is_err());
    }
    #[test]
    fn accepts_padding_not_included_in_declared_length() {
        let mut b = fixture();
        // Name is five bytes of Pascal-string data, followed by three alignment bytes.
        b[42..46].copy_from_slice(&5u32.to_le_bytes());
        assert_eq!(parse(&b).unwrap().entry_point, "EffectMain");
    }
    #[test]
    fn repairs_vendor_pascal_lengths_without_embedded_nuls() {
        let mut warnings = Vec::new();
        for (bytes, expected) in [
            (&b"\x1bBCC Beat Reactor\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"[..], "BCC Beat Reactor"),
            (&b"\x10BCC6ThreeWayCC\0"[..], "BCC6ThreeWayCC"),
            (&b"\x12BCC Trails\0"[..], "BCC Trails"),
        ] {
            assert_eq!(pstring(bytes, "name", &mut warnings).unwrap(), expected);
        }
        assert_eq!(warnings.len(), 3);
        assert!(pstring(b"\x12BCC Trails", "name", &mut warnings).is_err());
        assert!(pstring(b"\x04A\0BC", "name", &mut warnings).is_err());
        assert!(pstring(b"\x12A\0BC", "name", &mut warnings).is_err());
    }
}
