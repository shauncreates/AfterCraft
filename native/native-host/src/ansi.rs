//! The documented legacy ANSI callbacks, backed by the operating-system C runtime.
use after_effects_sys as ae;
use std::ffi::{c_char, c_int};
#[link(name = "msvcrt")]
unsafe extern "C" {
    fn atan(x: f64) -> f64;
    fn atan2(y: f64, x: f64) -> f64;
    fn ceil(x: f64) -> f64;
    fn cos(x: f64) -> f64;
    fn exp(x: f64) -> f64;
    fn fabs(x: f64) -> f64;
    fn floor(x: f64) -> f64;
    fn fmod(x: f64, y: f64) -> f64;
    fn hypot(x: f64, y: f64) -> f64;
    fn log(x: f64) -> f64;
    fn log10(x: f64) -> f64;
    fn pow(x: f64, y: f64) -> f64;
    fn sin(x: f64) -> f64;
    fn sqrt(x: f64) -> f64;
    fn tan(x: f64) -> f64;
    fn sprintf(dest: *mut c_char, format: *const c_char, ...) -> c_int;
    fn strcpy(dest: *mut c_char, source: *const c_char) -> *mut c_char;
    fn asin(x: f64) -> f64;
    fn acos(x: f64) -> f64;
}
pub fn block() -> ae::PF_ANSICallbacksBlock {
    ae::PF_ANSICallbacksBlock {
        atan: Some(atan),
        atan2: Some(atan2),
        ceil: Some(ceil),
        cos: Some(cos),
        exp: Some(exp),
        fabs: Some(fabs),
        floor: Some(floor),
        fmod: Some(fmod),
        hypot: Some(hypot),
        log: Some(log),
        log10: Some(log10),
        pow: Some(pow),
        sin: Some(sin),
        sqrt: Some(sqrt),
        tan: Some(tan),
        sprintf: Some(sprintf),
        strcpy: Some(strcpy),
        asin: Some(asin),
        acos: Some(acos),
        unused_longA: [0],
    }
}
pub static SUITE: ae::PF_ANSICallbacksSuite1 = ae::PF_ANSICallbacksSuite1 {
    atan: Some(atan),
    atan2: Some(atan2),
    ceil: Some(ceil),
    cos: Some(cos),
    exp: Some(exp),
    fabs: Some(fabs),
    floor: Some(floor),
    fmod: Some(fmod),
    hypot: Some(hypot),
    log: Some(log),
    log10: Some(log10),
    pow: Some(pow),
    sin: Some(sin),
    sqrt: Some(sqrt),
    tan: Some(tan),
    sprintf: Some(sprintf),
    strcpy: Some(strcpy),
    asin: Some(asin),
    acos: Some(acos),
};
