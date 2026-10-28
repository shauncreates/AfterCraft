//! Public options-button label registration, independent of native custom drawing.
use after_effects_sys as ae;
use std::cell::RefCell;

thread_local! {static OPTIONS_NAME: RefCell<String> = const {RefCell::new(String::new())};}

unsafe extern "C" fn set_name(_effect: ae::PF_ProgPtr, name: *const ae::A_char) -> ae::PF_Err {
    if name.is_null() {
        return ae::PF_Err_BAD_CALLBACK_PARAM;
    }
    let mut bytes = Vec::new();
    for at in 0..32 {
        // The SDK owns this NUL-terminated string for the callback duration.
        let byte = unsafe { *name.add(at) } as u8;
        if byte == 0 {
            return OPTIONS_NAME.with(|slot| match slot.try_borrow_mut() {
                Ok(mut text) => {
                    *text = String::from_utf8_lossy(&bytes).into_owned();
                    0
                }
                Err(_) => ae::PF_Err_BAD_CALLBACK_PARAM,
            });
        }
        bytes.push(byte);
    }
    ae::PF_Err_BAD_CALLBACK_PARAM
}

pub const SUITE: ae::PF_EffectUISuite1 = ae::PF_EffectUISuite1 { PF_SetOptionsButtonName: Some(set_name) };

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn label_registration_owns_the_text_and_rejects_unbounded_input() {
        let label = std::ffi::CString::new("Original test options").unwrap();
        assert_eq!(unsafe { set_name(std::ptr::null_mut(), label.as_ptr()) }, 0);
        drop(label);
        assert_eq!(OPTIONS_NAME.with(|slot| slot.borrow().clone()), "Original test options");
        assert_ne!(unsafe { set_name(std::ptr::null_mut(), std::ptr::null()) }, 0);
        let oversized = [b'x'; 32];
        assert_ne!(unsafe { set_name(std::ptr::null_mut(), oversized.as_ptr().cast()) }, 0);
    }
}
