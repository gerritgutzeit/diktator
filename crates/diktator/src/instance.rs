use anyhow::Result;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, OpenEventW, SetEvent, WaitForSingleObject, EVENT_MODIFY_STATE,
};

const MUTEX_NAME: PCWSTR = w!("Local\\Diktator.Gutzeit");
const SHOW_EVENT_NAME: PCWSTR = w!("Local\\Diktator.Gutzeit.Show");

pub struct SingleInstance {
    _mutex: HANDLE,
    show_event: HANDLE,
}

impl SingleInstance {
    pub fn acquire() -> Result<Option<Self>> {
        unsafe {
            let mutex = CreateMutexW(None, true, MUTEX_NAME)?;
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = CloseHandle(mutex);
                if let Ok(event) = OpenEventW(EVENT_MODIFY_STATE, false, SHOW_EVENT_NAME) {
                    let _ = SetEvent(event);
                    let _ = CloseHandle(event);
                }
                return Ok(None);
            }
            let show_event = CreateEventW(None, false, false, SHOW_EVENT_NAME)?;
            Ok(Some(Self {
                _mutex: mutex,
                show_event,
            }))
        }
    }

    pub fn listen_for_show(&self, on_show: impl Fn() + Send + Sync + 'static) {
        let event_bits = self.show_event.0 as usize;
        std::thread::spawn(move || loop {
            unsafe {
                let event = HANDLE(event_bits as *mut core::ffi::c_void);
                let wait = WaitForSingleObject(event, 500);
                if wait == WAIT_OBJECT_0 {
                    on_show();
                }
            }
        });
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.show_event);
            let _ = CloseHandle(self._mutex);
        }
    }
}
