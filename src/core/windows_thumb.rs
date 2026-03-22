use image::RgbaImage;
use std::ffi::c_void;
use std::mem::zeroed;
use windows::{
    core::*,
    Win32::{Foundation::*, Graphics::Gdi::*, UI::Shell::*},
};

fn hbitmap_to_rgba(hbitmap: HBITMAP) -> Option<RgbaImage> {
    unsafe {
        let mut bmp: BITMAP = zeroed();
        let result = GetObjectW(
            HGDIOBJ(hbitmap.0),
            size_of::<BITMAP>() as i32,
            Some(&mut bmp as *mut _ as *mut c_void),
        );

        if result == 0 {
            return None;
        }

        let width = bmp.bmWidth as u32;
        let height = bmp.bmHeight.abs() as u32;

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: bmp.bmWidth,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let hdc = GetDC(None);
        let mut buffer = vec![0u8; (width * height * 4) as usize];

        let lines_copied = GetDIBits(
            hdc,
            hbitmap,
            0,
            height,
            Some(buffer.as_mut_ptr() as *mut c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        ReleaseDC(None, hdc);

        if lines_copied == 0 {
            return None;
        }

        for rgba in buffer.chunks_exact_mut(4) {
            rgba.swap(0, 2);
        }

        RgbaImage::from_raw(width, height, buffer)
    }
}

pub fn get_thumbnail(path: &str, size: u32) -> Option<RgbaImage> {
    unsafe {
        let item: IShellItem = SHCreateItemFromParsingName(&HSTRING::from(path), None).ok()?;

        let factory: IShellItemImageFactory = item.cast().ok()?;

        let size_struct = SIZE {
            cx: size as i32,
            cy: size as i32,
        };

        let hbitmap = factory.GetImage(size_struct, SIIGBF_BIGGERSIZEOK).ok()?;

        let img = hbitmap_to_rgba(hbitmap);

        let _ = DeleteObject(HGDIOBJ::from(hbitmap));

        img
    }
}
