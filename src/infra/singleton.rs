#![cfg(windows)]

use std::process;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{CloseHandle, GetLastError, HANDLE, WIN32_ERROR},
        System::Threading::{CreateMutexW, ReleaseMutex},
        UI::WindowsAndMessaging::{
            FindWindowW, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
        },
    },
};

const MUTEX_NAME: &str = "Local\\NexaMediaApp_C3B2A1F0_Singleton\0";

const WINDOW_TITLE: &str = "Nexa\0";

const ERROR_ALREADY_EXISTS: WIN32_ERROR = WIN32_ERROR(183);

pub struct SingletonGuard {
    handle: HANDLE,
}

impl Drop for SingletonGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseMutex(self.handle);
            let _ = CloseHandle(self.handle);
        }
    }
}

pub fn acquire() -> SingletonGuard {
    unsafe {
        let name = PCWSTR(MUTEX_NAME.encode_utf16().collect::<Vec<u16>>().as_ptr());

        let handle = CreateMutexW(None, true, name).unwrap_or(HANDLE::default());

        let last_error = GetLastError();

        if last_error == ERROR_ALREADY_EXISTS {
            focus_existing_window();
            process::exit(0);
        }

        if handle.is_invalid() {
            eprintln!(
                "[singleton] CreateMutexW failed (error {:?}); \
                 running without singleton protection.",
                last_error
            );

            return SingletonGuard { handle };
        }

        SingletonGuard { handle }
    }
}

fn focus_existing_window() {
    unsafe {
        let title = PCWSTR(WINDOW_TITLE.encode_utf16().collect::<Vec<u16>>().as_ptr());

        let hwnd = match FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())) {
            Ok(hwnd) => hwnd,
            Err(_) => return,
        };

        if hwnd.0.is_null() {
            return;
        }

        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        } else {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }

        let _ = SetForegroundWindow(hwnd);
    }
}
