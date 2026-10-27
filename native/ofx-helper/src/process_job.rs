//! A Windows job contains the worker and its descendants, including inherited pipe handles.
use std::{os::windows::io::AsRawHandle, ptr};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::JobObjects::*,
};
pub struct WorkerJob(HANDLE);
impl WorkerJob {
    pub fn new() -> Result<Self, String> {
        let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() {
            return Err(format!("cannot create worker job: {}", std::io::Error::last_os_error()));
        }
        let job = Self(handle);
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&limits) as u32,
            )
        } == 0
        {
            return Err(format!("cannot configure worker job: {}", std::io::Error::last_os_error()));
        }
        Ok(job)
    }
    pub fn attach(&self, child: &std::process::Child) -> Result<(), String> {
        if unsafe { AssignProcessToJobObject(self.0, child.as_raw_handle()) } == 0 {
            return Err(format!("cannot contain worker process: {}", std::io::Error::last_os_error()));
        }
        Ok(())
    }
}
impl Drop for WorkerJob {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
