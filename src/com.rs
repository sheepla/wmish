use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};

pub struct CoInitializer {}

impl CoInitializer {
    pub fn new() -> windows_core::Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };
        Ok(Self {})
    }
}

impl Drop for CoInitializer {
    fn drop(&mut self) {
        unsafe { CoUninitialize() }
    }
}
