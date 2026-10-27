//! Capture public SDK parameter definitions while plugin-owned text is still valid.
use after_effects_sys as ae;
use serde_json::{Value, json};
use std::{cell::RefCell, ffi::CString};
thread_local! {static TEXT: RefCell<Vec<CString>>=const{RefCell::new(Vec::new())};}
pub fn capture(def: &mut ae::PF_ParamDef) -> Result<Value, i32> {
    unsafe {
        Ok(match def.param_type {
            ae::PF_Param_SLIDER => {
                json!({"type":"slider","default":def.u.sd.dephault,"min":def.u.sd.valid_min,"max":def.u.sd.valid_max,"sliderMin":def.u.sd.slider_min,"sliderMax":def.u.sd.slider_max})
            }
            ae::PF_Param_FIX_SLIDER => {
                json!({"type":"slider","default":f64::from(def.u.fd.dephault)/65536.0,"min":f64::from(def.u.fd.valid_min)/65536.0,"max":f64::from(def.u.fd.valid_max)/65536.0,"sliderMin":f64::from(def.u.fd.slider_min)/65536.0,"sliderMax":f64::from(def.u.fd.slider_max)/65536.0})
            }
            ae::PF_Param_FLOAT_SLIDER => {
                json!({"type":"slider","default":def.u.fs_d.dephault,"min":def.u.fs_d.valid_min,"max":def.u.fs_d.valid_max,"sliderMin":def.u.fs_d.slider_min,"sliderMax":def.u.fs_d.slider_max})
            }
            ae::PF_Param_CHECKBOX => json!({"type":"checkbox","default":def.u.bd.dephault!=0}),
            ae::PF_Param_ANGLE => {
                json!({"type":"angle","default":f64::from(def.u.ad.dephault)/65536.0})
            }
            ae::PF_Param_COLOR => {
                let p = def.u.cd.dephault;
                json!({"type":"color","default":[f64::from(p.red)/255.0,f64::from(p.green)/255.0,f64::from(p.blue)/255.0,f64::from(p.alpha)/255.0]})
            }
            ae::PF_Param_POINT => {
                json!({"type":"point","default":[f64::from(def.u.td.x_dephault)/65536.0/100.0,f64::from(def.u.td.y_dephault)/65536.0/100.0]})
            }
            ae::PF_Param_POPUP => {
                let popup = def.u.pd;
                if popup.u.namesptr.is_null() {
                    return Ok(Value::Null);
                }
                let mut bytes = Vec::new();
                for offset in 0..65536 {
                    let byte = *popup.u.namesptr.add(offset) as u8;
                    bytes.push(byte);
                    if byte == 0 {
                        break;
                    }
                }
                if bytes.last() != Some(&0) {
                    return Err(ae::PF_Err_BAD_CALLBACK_PARAM);
                }
                let text = CString::from_vec_with_nul(bytes).map_err(|_| ae::PF_Err_BAD_CALLBACK_PARAM)?;
                let options: Vec<String> = text.to_string_lossy().split('|').map(str::to_owned).collect();
                def.u.pd.u.namesptr = text.as_ptr();
                TEXT.with(|texts| -> Result<(), i32> {
                    let mut texts = texts.try_borrow_mut().map_err(|_| ae::PF_Err_BAD_CALLBACK_PARAM)?;
                    if texts.len() >= 1024 {
                        return Err(ae::PF_Err_OUT_OF_MEMORY);
                    }
                    texts.push(text);
                    Ok(())
                })?;
                if options.len() > 1024
                    || usize::try_from(popup.num_choices).ok() != Some(options.len())
                    || popup.dephault < 1
                    || popup.dephault > popup.num_choices
                {
                    Value::Null
                } else {
                    json!({"type":"popup","default":popup.dephault-1,"options":options})
                }
            }
            _ => Value::Null,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owns_popup_text_after_registration() {
        let mut def: ae::PF_ParamDef = unsafe { std::mem::zeroed() };
        let text = CString::new("First|Second").unwrap();
        def.param_type = ae::PF_Param_POPUP;
        def.u.pd.num_choices = 2;
        def.u.pd.dephault = 2;
        def.u.pd.u.namesptr = text.as_ptr();
        assert_eq!(capture(&mut def).unwrap(), json!({"type":"popup","default":1,"options":["First","Second"]}));
        drop(text);
        assert_eq!(unsafe { std::ffi::CStr::from_ptr(def.u.pd.u.namesptr) }.to_bytes(), b"First|Second");
    }
}
