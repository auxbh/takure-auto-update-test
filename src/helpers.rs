use anyhow::Result;
use crate::CONFIGURATION;
use crate::sys::{property_node_refer, NodeType};
use crate::takure::CURRENT_CARD_ID;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::ptr;
use snafu::Snafu;
use widestring::U16CString;
use winapi::shared::minwindef::{FALSE, HINSTANCE, HMODULE};
use winapi::shared::ntdef::HANDLE;
use winapi::shared::winerror::ERROR_INSUFFICIENT_BUFFER;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::handleapi::{CloseHandle, DuplicateHandle};
use winapi::um::libloaderapi::{FreeLibraryAndExitThread, GetModuleFileNameW};
use winapi::um::processthreadsapi::{GetCurrentProcess, GetCurrentThread};
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winnt::SYNCHRONIZE;

// Used so the background thread that applies a staged update can wait for DllMain to
// actually return (and release the loader lock) before calling FreeLibrary on ourselves —
// doing that from within DllMain itself risks deadlock.
#[cfg_attr(not(feature = "autoupdate"), allow(dead_code))]
pub struct ThreadHandle(HANDLE);

unsafe impl Send for ThreadHandle {}
unsafe impl Sync for ThreadHandle {}
impl ThreadHandle {
    #[cfg_attr(not(feature = "autoupdate"), allow(dead_code))]
    pub fn duplicate_current_thread_handle() -> Result<Self, u32> {
        unsafe {
            let mut cur_thread = ptr::null_mut();
            let result = DuplicateHandle(
                GetCurrentProcess(),
                GetCurrentThread(),
                GetCurrentProcess(),
                &mut cur_thread,
                SYNCHRONIZE,
                FALSE,
                0,
            );

            if result == 0 {
                return Err(GetLastError());
            }

            Ok(ThreadHandle(cur_thread as HANDLE))
        }
    }

    #[cfg_attr(not(feature = "autoupdate"), allow(dead_code))]
    pub fn wait_and_close(self, ms: u32) {
        unsafe {
            WaitForSingleObject(self.0, ms);
            CloseHandle(self.0);
        }
    }
}

#[cfg_attr(not(feature = "autoupdate"), allow(dead_code))]
pub struct LibraryHandle(HINSTANCE);

unsafe impl Send for LibraryHandle {}
unsafe impl Sync for LibraryHandle {}
impl LibraryHandle {
    #[cfg_attr(not(feature = "autoupdate"), allow(dead_code))]
    pub unsafe fn new(handle: HINSTANCE) -> Self {
        Self(handle)
    }

    #[cfg_attr(not(feature = "autoupdate"), allow(dead_code))]
    pub fn handle(&self) -> HINSTANCE {
        self.0
    }

    #[cfg_attr(not(feature = "autoupdate"), allow(dead_code))]
    pub fn free_and_exit_thread(self, code: u32) -> ! {
        unsafe {
            FreeLibraryAndExitThread(self.0, code);
        }
        unreachable!()
    }
}

#[derive(Debug, Snafu)]
pub enum ReadStringFnError {
    InvalidData,
    Other { errno: u32 },
}

#[cfg_attr(not(feature = "autoupdate"), allow(dead_code))]
pub fn get_module_file_name(handle: HMODULE) -> Result<String, ReadStringFnError> {
    let mut buffer = vec![0u16; 255];
    let mut buffer_length = 255u32;

    loop {
        let result =
            unsafe { GetModuleFileNameW(handle, buffer.as_mut_ptr(), buffer_length) as i32 };
        let errno = unsafe { GetLastError() };

        if result != 0 && errno != ERROR_INSUFFICIENT_BUFFER {
            let out = U16CString::from_vec_truncate(&buffer[..result as usize]);

            return out.to_string().map_err(|_| ReadStringFnError::InvalidData);
        }

        if errno != ERROR_INSUFFICIENT_BUFFER {
            return Err(ReadStringFnError::Other { errno });
        }

        buffer.resize(buffer.len() * 2, 0);
        buffer_length = buffer.len() as u32;
    }
}

pub fn request_agent() -> ureq::Agent {
    let timeout = CONFIGURATION.general.timeout;
    let timeout = if timeout > 10000 { 10000 } else { timeout };

    ureq::builder()
        .timeout(std::time::Duration::from_millis(timeout))
        .build()
}

fn request<T>(
    method: impl AsRef<str>,
    url: impl AsRef<str>,
    body: Option<T>,
) -> Result<ureq::Response>
where
    T: Serialize + Debug,
{
    let agent = request_agent();

    let method = method.as_ref();
    let url = url.as_ref();
    debug!("{} request to {} with body: {:#?}", method, url, body);

    let authorization = format!("Bearer {}", CONFIGURATION.tachi.api_key);
    let request = agent
        .request(method, url)
        .set("Authorization", authorization.as_str());
    let response = match body {
        Some(body) => request.send_json(body),
        None => request.call(),
    }
    .map_err(|err| anyhow::anyhow!("Could not reach Tachi API: {:#}", err))?;

    Ok(response)
}

pub fn call_tachi<T>(method: impl AsRef<str>, url: impl AsRef<str>, body: Option<T>) -> Result<()>
where
    T: Serialize + Debug,
{
    let response = request(method, url, body)?;
    let response: serde_json::Value = response.into_json()?;
    debug!("Tachi API response: {:#?}", response);

    Ok(())
}

pub fn request_tachi<T, R>(
    method: impl AsRef<str>,
    url: impl AsRef<str>,
    body: Option<T>,
) -> Result<R>
where
    T: Serialize + Debug,
    R: for<'de> Deserialize<'de> + Debug,
{
    let response = request(method, url, body)?;
    let response = response.into_json()?;
    debug!("Tachi API response: {:#?}", response);

    Ok(response)
}

pub fn get_current_card_id() -> Option<String> {
    let guard = CURRENT_CARD_ID.read().unwrap_or_else(|err| {
        error!("Current card ID RwLock is poisoned: {:#}", err);
        err.into_inner()
    });

    guard.clone()
}

pub fn read_node_str(node: *const (), path: *const u8, length: usize) -> Option<String> {
    let mut buffer = [0u8; 32];
    let result = unsafe {
        property_node_refer(
            node,
            node,
            path,
            NodeType::NodeStr,
            buffer.as_mut_ptr() as *mut (),
            32,
        )
    };

    if result < 0 {
        return None;
    }

    Some(String::from_utf8_lossy(&buffer[..length]).to_string())
}
