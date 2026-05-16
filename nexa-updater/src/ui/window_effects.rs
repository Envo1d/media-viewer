#[cfg(windows)]
pub struct WindowEffects {
    applied: bool,
}

#[cfg(windows)]
impl WindowEffects {
    pub fn new() -> Self {
        Self { applied: false }
    }

    pub fn apply(&mut self) {
        if self.applied {
            return;
        }

        use windows::Win32::{
            Graphics::Dwm::*, UI::Input::KeyboardAndMouse::GetActiveWindow,
            UI::WindowsAndMessaging::*,
        };

        let hwnd = unsafe { GetActiveWindow() };
        if hwnd.is_invalid() {
            return;
        }

        unsafe {
            let pref = DWMWCP_ROUND;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &pref as *const _ as _,
                std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
            );

            let val: i32 = 2;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_NCRENDERING_POLICY,
                &val as *const _ as _,
                std::mem::size_of::<i32>() as u32,
            );

            let backdrop = DWMSBT_MAINWINDOW;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &backdrop as *const _ as _,
                std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
            );

            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            let new_style = (style & !WS_CAPTION.0) | WS_THICKFRAME.0;
            SetWindowLongW(hwnd, GWL_STYLE, new_style as i32);

            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
            );
        }

        self.applied = true;
    }
}

#[cfg(not(windows))]
pub struct WindowEffects;

#[cfg(not(windows))]
impl WindowEffects {
    pub fn new() -> Self {
        Self
    }
    pub fn apply(&mut self) {}
}
