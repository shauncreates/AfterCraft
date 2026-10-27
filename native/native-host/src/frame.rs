//! Single-frame PNG exchange. AE receives premultiplied 8-bit ARGB; PNG uses straight RGBA.
use after_effects_sys as ae;
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    #[serde(default)]
    pub host_api: HostApi,
    pub plugin: PathBuf,
    #[serde(default)]
    pub match_name: Option<String>,
    #[serde(default)]
    pub dll_dirs: Vec<PathBuf>,
    pub input: PathBuf,
    pub output: PathBuf,
    #[serde(default)]
    pub time_seconds: f64,
    #[serde(default)]
    pub frame_rate: crate::timing::FrameRate,
    #[serde(default)]
    pub frame_index: Option<u64>,
    #[serde(default)]
    pub downsample: Downsample,
    #[serde(default)]
    pub layer_size: Option<[u32; 2]>,
    #[serde(default)]
    pub render_mode: RenderMode,
    /// Parameters by 1-based index in the plugin's native parameter order.
    #[serde(default)]
    pub parameters: BTreeMap<usize, Value>,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Downsample {
    pub num: u32,
    pub den: u32,
}
impl Default for Downsample {
    fn default() -> Self {
        Self { num: 1, den: 1 }
    }
}
impl Downsample {
    pub fn validate(self) -> Result<(), String> {
        if self.num == 0 || self.den == 0 || self.num > self.den || self.den > 65536 || u64::from(self.num) * 16384 < u64::from(self.den) {
            return Err("invalid preview downsample ratio".into());
        }
        Ok(())
    }
}
#[derive(Default, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum HostApi {
    #[default]
    ThirdParty,
    AfterEffects,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderMode {
    #[default]
    Auto,
    Legacy,
    Smart,
}
impl Job {
    pub fn read(path: &Path) -> Result<Self, String> {
        let mut bytes = Vec::new();
        std::fs::File::open(path).map_err(|e| e.to_string())?.take(1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        if bytes.len() > 1024 * 1024 {
            return Err("render job exceeds 1 MiB".into());
        }
        let job: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        for path in [&job.plugin, &job.input, &job.output] {
            if !path.is_absolute() {
                return Err("all render paths must be absolute".into());
            }
        }
        if job.dll_dirs.len() > 16 || job.dll_dirs.iter().any(|p| !p.is_absolute() || !p.is_dir()) {
            return Err("invalid DLL directories".into());
        }
        if job.input == job.output || job.output.exists() {
            return Err("output must be a new file, distinct from input".into());
        }
        if !job.time_seconds.is_finite() || !(0.0..=36000.0).contains(&job.time_seconds) {
            return Err("time must be within 0..36000 seconds".into());
        }
        if job.parameters.len() > 1024 {
            return Err("too many parameters".into());
        }
        job.clock()?;
        job.downsample.validate()?;
        if job.layer_size.is_some_and(|s| s.into_iter().any(|n| n == 0 || n > 16384)) {
            return Err("logical layer dimensions must be within 1..16384".into());
        }
        Ok(job)
    }
    pub fn clock(&self) -> Result<crate::timing::Clock, String> {
        crate::timing::clock(self.frame_rate, self.time_seconds, self.frame_index)
    }
    pub fn logical_size(&self, width: u32, height: u32) -> Result<[u32; 2], String> {
        self.downsample.validate()?;
        let scaled = |n| {
            u32::try_from((u64::from(n) * u64::from(self.downsample.den)).div_ceil(u64::from(self.downsample.num)))
                .map_err(|_| "logical layer size overflow".to_string())
        };
        let size = match self.layer_size {
            Some(size) => size,
            None => [scaled(width)?, scaled(height)?],
        };
        if size.into_iter().any(|n| n == 0 || n > 16384) {
            return Err("logical layer dimensions must be within 1..16384".into());
        }
        Ok(size)
    }
}
pub fn load(path: &Path) -> Result<(u32, u32, Vec<ae::PF_Pixel>), String> {
    let reader = image::ImageReader::open(path).map_err(|e| e.to_string())?.with_guessed_format().map_err(|e| e.to_string())?;
    if reader.format() != Some(image::ImageFormat::Png) {
        return Err("input must be PNG".into());
    }
    let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > 16 * 1024 * 1024 || w > 16384 || h > 16384 {
        return Err("input exceeds frame limits".into());
    }
    let mut reader = image::ImageReader::open(path).map_err(|e| e.to_string())?.with_guessed_format().map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|e| e.to_string())?.to_rgba8();
    let pixels = image
        .pixels()
        .map(|p| {
            let [r, g, b, a] = p.0;
            let premul = |c: u8| ((u16::from(c) * u16::from(a) + 127) / 255) as u8;
            ae::PF_Pixel { alpha: a, red: premul(r), green: premul(g), blue: premul(b) }
        })
        .collect();
    Ok((w, h, pixels))
}
pub fn save(path: &Path, w: u32, h: u32, pixels: &[ae::PF_Pixel]) -> Result<(), String> {
    let count = (w as usize).checked_mul(h as usize).ok_or("frame overflow")?;
    if count != pixels.len() {
        return Err("invalid output frame length".into());
    }
    let mut bytes = Vec::with_capacity(count.checked_mul(4).ok_or("frame overflow")?);
    for p in pixels {
        let straight = |c: u8| {
            if p.alpha == 0 { 0 } else { ((u32::from(c) * 255 + u32::from(p.alpha) / 2) / u32::from(p.alpha)).min(255) as u8 }
        };
        bytes.extend([straight(p.red), straight(p.green), straight(p.blue), p.alpha]);
    }
    let file = std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(|e| e.to_string())?;
    let encoder = image::codecs::png::PngEncoder::new(file);
    use image::ImageEncoder;
    encoder.write_image(&bytes, w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())
}
pub fn defaults(def: &mut ae::PF_ParamDef, w: i32, h: i32) {
    // Read only the union member selected by the SDK's param_type tag.
    unsafe {
        match def.param_type {
            ae::PF_Param_SLIDER => def.u.sd.value = def.u.sd.dephault,
            ae::PF_Param_FIX_SLIDER => def.u.fd.value = def.u.fd.dephault,
            ae::PF_Param_FLOAT_SLIDER => def.u.fs_d.value = f64::from(def.u.fs_d.dephault),
            ae::PF_Param_ANGLE => def.u.ad.value = def.u.ad.dephault,
            ae::PF_Param_CHECKBOX => def.u.bd.value = i32::from(def.u.bd.dephault),
            ae::PF_Param_POPUP => def.u.pd.value = i32::from(def.u.pd.dephault),
            ae::PF_Param_COLOR => def.u.cd.value = def.u.cd.dephault,
            ae::PF_Param_POINT => {
                def.u.td.x_value = (i64::from(def.u.td.x_dephault) * i64::from(w) / 100).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
                def.u.td.y_value = (i64::from(def.u.td.y_dephault) * i64::from(h) / 100).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
            }
            ae::PF_Param_POINT_3D => {
                def.u.point3d_d.x_value = def.u.point3d_d.x_dephault;
                def.u.point3d_d.y_value = def.u.point3d_d.y_dephault;
                def.u.point3d_d.z_value = def.u.point3d_d.z_dephault;
            }
            ae::PF_Param_ARBITRARY_DATA => def.u.arb_d.value = def.u.arb_d.dephault,
            ae::PF_Param_PATH => def.u.path_d.path_id = def.u.path_d.dephault.max(0) as u32,
            _ => {}
        }
    }
}
pub fn override_value(def: &mut ae::PF_ParamDef, value: &Value) -> Result<(), String> {
    if def.param_type == ae::PF_Param_CHECKBOX
        && let Some(value) = value.as_bool()
    {
        def.u.bd.value = i32::from(value);
        return Ok(());
    }
    if def.param_type == ae::PF_Param_COLOR {
        let values = components::<4>(value)?;
        if values.iter().any(|v| !(0.0..=1.0).contains(v)) {
            return Err("Color components must be within 0..1".into());
        }
        let [r, g, b, a] = values.map(|v| (v * 255.0).round() as u8);
        def.u.cd.value = ae::PF_Pixel { alpha: a, red: r, green: g, blue: b };
        return Ok(());
    }
    if def.param_type == ae::PF_Param_POINT {
        let [x, y] = components::<2>(value)?;
        let (x, y) = (fixed(x)?, fixed(y)?);
        def.u.td.x_value = x;
        def.u.td.y_value = y;
        return Ok(());
    }
    if def.param_type == ae::PF_Param_POINT_3D {
        let [x, y, z] = components::<3>(value)?;
        if [x, y, z].iter().any(|v| v.abs() > 1e9) {
            return Err("3D point exceeds supported range".into());
        }
        def.u.point3d_d.x_value = x;
        def.u.point3d_d.y_value = y;
        def.u.point3d_d.z_value = z;
        return Ok(());
    }
    let n = value.as_f64().filter(|n| n.is_finite()).ok_or("currently only finite numeric parameter overrides are supported")?;
    unsafe {
        match def.param_type {
            ae::PF_Param_FLOAT_SLIDER if n >= f64::from(def.u.fs_d.valid_min) && n <= f64::from(def.u.fs_d.valid_max) => def.u.fs_d.value = n,
            ae::PF_Param_FIX_SLIDER if n * 65536.0 >= f64::from(def.u.fd.valid_min) && n * 65536.0 <= f64::from(def.u.fd.valid_max) => {
                def.u.fd.value = (n * 65536.0).round() as i32
            }
            ae::PF_Param_SLIDER if n.fract() == 0.0 && n >= f64::from(def.u.sd.valid_min) && n <= f64::from(def.u.sd.valid_max) => def.u.sd.value = n as i32,
            ae::PF_Param_CHECKBOX if n == 0.0 || n == 1.0 => def.u.bd.value = n as i32,
            ae::PF_Param_POPUP if n.fract() == 0.0 && n >= 1.0 && n <= f64::from(def.u.pd.num_choices) => def.u.pd.value = n as i32,
            ae::PF_Param_ANGLE => def.u.ad.value = fixed(n)?,
            _ => return Err("unsupported parameter type or value outside declared bounds".into()),
        }
    }
    Ok(())
}
fn fixed(value: f64) -> Result<i32, String> {
    let raw = value * 65536.0;
    if !raw.is_finite() || raw < f64::from(i32::MIN) || raw > f64::from(i32::MAX) {
        return Err("Value exceeds SDK fixed-point range".into());
    }
    Ok(raw.round() as i32)
}
fn components<const N: usize>(value: &Value) -> Result<[f64; N], String> {
    let array = value.as_array().filter(|a| a.len() == N).ok_or_else(|| format!("Expected {N} components"))?;
    let mut values = [0.0; N];
    for (dst, src) in values.iter_mut().zip(array) {
        *dst = src.as_f64().filter(|v| v.is_finite()).ok_or("Components must be finite numbers")?;
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_geometry_rejects_zero_overflow_and_preserves_odd_layer_sizes() {
        let mut job: Job =
            serde_json::from_value(serde_json::json!({"plugin":"test.aex","input":"in.png","output":"out.png","downsample":{"num":1,"den":3}})).unwrap();
        assert_eq!(job.logical_size(16, 8).unwrap(), [48, 24]);
        job.layer_size = Some([47, 23]);
        assert_eq!(job.logical_size(16, 8).unwrap(), [47, 23]);
        job.layer_size = None;
        assert!(job.logical_size(u32::MAX, u32::MAX).is_err());
        for (num, den) in [(0, 1), (1, 0), (2, 1), (1, 65537)] {
            job.downsample = Downsample { num, den };
            assert!(job.logical_size(16, 8).is_err());
        }
    }
    #[test]
    fn defaults_and_overrides_follow_the_union_tag() {
        let mut def: ae::PF_ParamDef = unsafe { std::mem::zeroed() };
        def.param_type = ae::PF_Param_FLOAT_SLIDER;
        def.u.fs_d.dephault = 2.0;
        def.u.fs_d.valid_min = 0.0;
        def.u.fs_d.valid_max = 10.0;
        defaults(&mut def, 64, 64);
        assert_eq!(unsafe { def.u.fs_d.value }, 2.0);
        assert!(override_value(&mut def, &serde_json::json!(5)).is_ok());
        assert_eq!(unsafe { def.u.fs_d.value }, 5.0);
        assert!(override_value(&mut def, &serde_json::json!(11)).is_err());
        assert_eq!(unsafe { def.u.fs_d.value }, 5.0);
    }
    #[test]
    fn typed_overrides_are_bounded_and_atomic() {
        let mut def: ae::PF_ParamDef = unsafe { std::mem::zeroed() };
        def.param_type = ae::PF_Param_POINT;
        override_value(&mut def, &serde_json::json!([32.5, 16])).unwrap();
        assert_eq!(unsafe { def.u.td.x_value }, (32.5 * 65536.0) as i32);
        assert!(override_value(&mut def, &serde_json::json!([5, 1e20])).is_err());
        assert_eq!(unsafe { def.u.td.x_value }, (32.5 * 65536.0) as i32);
        def.param_type = ae::PF_Param_COLOR;
        override_value(&mut def, &serde_json::json!([1, 0.5, 0, 1])).unwrap();
        assert_eq!(unsafe { def.u.cd.value.green }, 128);
        assert!(override_value(&mut def, &serde_json::json!([1, 2, 0, 1])).is_err());
        def.param_type = ae::PF_Param_ANGLE;
        override_value(&mut def, &serde_json::json!(90.5)).unwrap();
        assert_eq!(unsafe { def.u.ad.value }, (90.5 * 65536.0) as i32);
    }
}
