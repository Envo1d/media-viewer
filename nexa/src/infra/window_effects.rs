#![cfg(windows)]

use windows::Win32::Foundation::*;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::*;

#[inline]
fn get_x_lparam(lp: isize) -> i32 {
    (lp & 0xFFFF) as i16 as i32
}

#[inline]
fn get_y_lparam(lp: isize) -> i32 {
    ((lp >> 16) & 0xFFFF) as i16 as i32
}

pub unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    unsafe {
        match msg {
            WM_NCHITTEST => {
                let x = get_x_lparam(lparam.0);
                let y = get_y_lparam(lparam.0);

                let mut rect = RECT::default();
                let _ = GetWindowRect(hwnd, &mut rect);

                let border = 8;

                let left = x >= rect.left && x < rect.left + border;
                let right = x < rect.right && x >= rect.right - border;
                let top = y >= rect.top && y < rect.top + border;
                let bottom = y < rect.bottom && y >= rect.bottom - border;

                match (left, right, top, bottom) {
                    (true, _, true, _) => LRESULT(HTTOPLEFT as isize),
                    (_, true, true, _) => LRESULT(HTTOPRIGHT as isize),
                    (true, _, _, true) => LRESULT(HTBOTTOMLEFT as isize),
                    (_, true, _, true) => LRESULT(HTBOTTOMRIGHT as isize),
                    (true, _, _, _) => LRESULT(HTLEFT as isize),
                    (_, true, _, _) => LRESULT(HTRIGHT as isize),
                    (_, _, true, _) => LRESULT(HTTOP as isize),
                    (_, _, _, true) => LRESULT(HTBOTTOM as isize),
                    _ => DefSubclassProc(hwnd, msg, wparam, lparam),
                }
            }
            _ => DefSubclassProc(hwnd, msg, wparam, lparam),
        }
    }
}

pub struct WindowEffects {
    hwnd: Option<HWND>,
}

impl WindowEffects {
    pub fn new() -> Self {
        Self { hwnd: None }
    }

    pub fn apply(&mut self) {
        if self.hwnd.is_some() {
            return;
        }

        let hwnd = unsafe { GetActiveWindow() };

        if hwnd.is_invalid() {
            return;
        }

        self.hwnd = Some(hwnd);

        unsafe {
            make_borderless(hwnd);
            enable_rounding(hwnd);
            enable_shadow(hwnd);
            enable_backdrop(hwnd);

            let _ = SetWindowSubclass(hwnd, Some(subclass_proc), 1, 0);
        }
    }
}

unsafe fn make_borderless(hwnd: HWND) {
    unsafe {
        let style = GetWindowLongW(hwnd, GWL_STYLE);

        let new_style = (style as u32 & !WS_CAPTION.0) | WS_THICKFRAME.0;

        SetWindowLongW(hwnd, GWL_STYLE, new_style as i32);

        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);

        let new_ex_style = (ex_style as u32) | WS_EX_APPWINDOW.0;

        SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex_style as i32);

        let _ = SetWindowPos(
            hwnd,
            Option::from(HWND(std::ptr::null_mut())),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        );
    }
}

unsafe fn enable_rounding(hwnd: HWND) {
    let preference = DWMWCP_ROUND;

    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const _ as _,
            size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
    }
}

unsafe fn enable_shadow(hwnd: HWND) {
    let val: i32 = 2;

    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY,
            &val as *const _ as _,
            size_of::<i32>() as u32,
        );
    }
}

unsafe fn enable_backdrop(hwnd: HWND) {
    let backdrop = DWMSBT_MAINWINDOW;

    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &backdrop as *const _ as _,
            size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
        );
    }
}
