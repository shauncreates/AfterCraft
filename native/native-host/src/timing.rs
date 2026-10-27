//! Rational frame clocks. Frame-index jobs never pass through floating-point time.
use serde::Deserialize;
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameRate {
    pub num: u32,
    pub den: u32,
}
impl Default for FrameRate {
    fn default() -> Self {
        Self { num: 25, den: 1 }
    }
}
pub struct Clock {
    pub scale: u32,
    pub step: i32,
    pub current: i32,
    pub total: i32,
    pub ticks_per_frame: i64,
}
pub fn clock(rate: FrameRate, seconds: f64, frame: Option<u64>) -> Result<Clock, String> {
    if rate.num == 0 || rate.den == 0 || rate.num > 240000 || rate.den > 100000 {
        return Err("Invalid rational frame rate".into());
    }
    let fps = f64::from(rate.num) / f64::from(rate.den);
    if !(1.0..=240.0).contains(&fps) || !seconds.is_finite() || !(0.0..=36000.0).contains(&seconds) {
        return Err("Frame rate/time outside supported range".into());
    }
    let mut a = rate.num;
    let mut b = rate.den;
    while b != 0 {
        let rem = a % b;
        a = b;
        b = rem;
    }
    let reduced_scale = rate.num / a;
    let factor = (10_000 / reduced_scale).max(1);
    let scale = reduced_scale * factor;
    let step = rate.den / a * factor;
    let current = if let Some(frame) = frame {
        if seconds != 0.0 {
            return Err("Choose frame_index or time_seconds, not both".into());
        }
        i32::try_from(frame.checked_mul(u64::from(step)).ok_or("Frame index overflow")?).map_err(|_| "Frame index exceeds SDK time range")?
    } else {
        let units = (seconds * f64::from(scale)).round();
        if units > f64::from(i32::MAX) {
            return Err("Time exceeds SDK time range".into());
        }
        units as i32
    };
    if f64::from(current) / f64::from(scale) > 36000.0 {
        return Err("Frame index exceeds 10 hours".into());
    }
    let total = current.checked_add(scale as i32).ok_or("Frame context exceeds SDK time range")?;
    let ticks = (254_016_000_000u64 * u64::from(step) + u64::from(scale) / 2) / u64::from(scale);
    Ok(Clock { scale, step: step as i32, current, total, ticks_per_frame: ticks as i64 })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ntsc_frame_index_is_exact() {
        let c = clock(FrameRate { num: 30000, den: 1001 }, 0.0, Some(10000)).unwrap();
        assert_eq!((c.scale, c.step, c.current, c.ticks_per_frame), (30000, 1001, 10010000, 8_475_667_200));
    }
    #[test]
    fn rejects_invalid_and_overflowing_clocks() {
        assert!(clock(FrameRate { num: 0, den: 1 }, 0.0, None).is_err());
        assert!(clock(FrameRate::default(), f64::NAN, None).is_err());
        assert!(clock(FrameRate::default(), 1.0, Some(1)).is_err());
        assert!(clock(FrameRate::default(), 0.0, Some(u64::MAX)).is_err());
    }
    #[test]
    fn preserves_subframe_time_at_integer_rates() {
        let c = clock(FrameRate::default(), 0.01, None).unwrap();
        assert_eq!((c.scale, c.step, c.current), (10000, 400, 100));
    }
}
