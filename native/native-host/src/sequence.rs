//! Read-only access to the actual sequence handle, scoped to this frame's render lifecycle.
use after_effects_sys as ae;
use std::{cell::Cell, ptr};
thread_local! {static ACTIVE:Cell<Option<(usize,usize)>>=const{Cell::new(None)};}
pub struct Guard;
impl Guard {
    pub fn enter(effect: ae::PF_ProgPtr, handle: ae::PF_Handle) -> Self {
        ACTIVE.with(|s| s.set(Some((effect as usize, handle as usize))));
        Self
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|s| s.set(None));
    }
}
unsafe extern "C" fn get(effect: ae::PF_ProgPtr, out: *mut ae::PF_ConstHandle) -> i32 {
    if out.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    unsafe {
        *out = ptr::null();
    }
    ACTIVE.with(|s| {
        let Some((owner, handle)) = s.get().filter(|(owner, _)| *owner == effect as usize) else {
            return ae::PF_Err_INVALID_CALLBACK;
        };
        let _ = owner;
        unsafe {
            *out = handle as ae::PF_ConstHandle;
        }
        crate::event(serde_json::json!({"stage":"sequence_callback","name":"PF_GetConstSequenceData","present":handle!=0}));
        0
    })
}
pub static SUITE: ae::PF_EffectSequenceDataSuite1 = ae::PF_EffectSequenceDataSuite1 { PF_GetConstSequenceData: Some(get) };
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_foreign_and_expired_contexts() {
        let mut token = 0usize;
        let effect = ptr::from_mut(&mut token).cast();
        let mut data = ptr::null_mut();
        let handle = ptr::from_mut(&mut data);
        let mut out = ptr::null();
        let guard = Guard::enter(effect, handle);
        unsafe {
            assert_eq!(get(effect, &mut out), 0);
            assert_eq!(out as usize, handle as usize);
            assert_ne!(get(ptr::null_mut(), &mut out), 0);
            assert!(out.is_null());
            drop(guard);
            assert_ne!(get(effect, &mut out), 0);
        }
    }
}
