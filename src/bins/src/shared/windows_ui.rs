use image::load_from_memory;
use std::sync::{OnceLock, RwLock};
use windows::core::BOOL;
use windows::Win32::Foundation::HWND as WinHWND;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, ReleaseDC, LOGPIXELSY};
use windows::Win32::Graphics::GdiPlus::{
    FillModeAlternate, GdipAddPathArc, GdipAddPathRectangle, GdipClosePathFigure, GdipCreateBitmapFromScan0, GdipCreatePath, GdipCreatePen1,
    GdipCreateSolidFill, GdipDeleteBrush, GdipDeletePath, GdipDeletePen, GdipDisposeImage, GdipDrawImageRectRectI, GdipDrawLine,
    GdipFillPath, GpGraphics as GdiplusGraphics, GpPath as GdiplusPath, UnitPixel,
};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use winsafe::{self as w, co, guard::DeleteObjectGuard, prelude::*};

pub const PIXEL_FORMAT32BPP_ARGB: i32 = 0x26200a;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

static THEME_OVERRIDE: OnceLock<RwLock<Option<ThemeMode>>> = OnceLock::new();

pub fn set_theme_override(theme: Option<ThemeMode>) {
    let lock = THEME_OVERRIDE.get_or_init(|| RwLock::new(None));
    if let Ok(mut guard) = lock.write() {
        *guard = theme;
    }
}

pub fn set_theme_override_from_str(theme: Option<&str>) -> bool {
    match theme {
        None => {
            set_theme_override(None);
            true
        }
        Some(value) => match value.to_ascii_lowercase().as_str() {
            "dark" => {
                set_theme_override(Some(ThemeMode::Dark));
                true
            }
            "light" => {
                set_theme_override(Some(ThemeMode::Light));
                true
            }
            _ => false,
        },
    }
}

fn read_reg_u32(subkey: windows::core::PCWSTR, value: windows::core::PCWSTR) -> Option<u32> {
    let mut data: u32 = 0;
    let mut len = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, subkey, value, RRF_RT_REG_DWORD, None, Some(&mut data as *mut _ as *mut _), Some(&mut len))
    };
    if status.is_ok() {
        Some(data)
    } else {
        None
    }
}

fn read_system_theme() -> ThemeMode {
    let subkey = windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let value = windows::core::w!("AppsUseLightTheme");
    match read_reg_u32(subkey, value) {
        Some(0) => ThemeMode::Dark,
        Some(1) => ThemeMode::Light,
        _ => ThemeMode::Light,
    }
}

pub fn current_theme() -> ThemeMode {
    if let Some(lock) = THEME_OVERRIDE.get() {
        if let Ok(guard) = lock.read() {
            if let Some(theme) = *guard {
                return theme;
            }
        }
    }
    read_system_theme()
}

pub fn apply_window_style(hwnd: WinHWND, theme: ThemeMode) {
    if hwnd.0 == std::ptr::null_mut() {
        return;
    }
    unsafe {
        let use_dark_mode = BOOL::from(theme == ThemeMode::Dark);
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &use_dark_mode as *const _ as *const _,
            std::mem::size_of::<BOOL>() as u32,
        );

        // DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_ROUND = 2
        let corner_preference = 2i32;
        let _ = DwmSetWindowAttribute(
            hwnd,
            windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE(33),
            &corner_preference as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );
    }
}

pub fn get_dpi_scale(hwnd: WinHWND) -> f32 {
    unsafe {
        let hdc = GetDC(Some(hwnd));
        if hdc.is_invalid() {
            return 1.0;
        }
        let dpi = GetDeviceCaps(Some(hdc), LOGPIXELSY);
        ReleaseDC(Some(hwnd), hdc);
        if dpi > 0 {
            dpi as f32 / 96.0
        } else {
            1.0
        }
    }
}

pub fn load_logo_data(target_size: i32, image_data: &'static [u8]) -> Option<(i32, i32, Vec<u8>)> {
    match load_from_memory(image_data) {
        Ok(img) => {
            let target_size = target_size.max(1) as u32;
            let resized = img.resize_exact(target_size, target_size, image::imageops::FilterType::Lanczos3);
            let mut bgra = Vec::with_capacity((target_size * target_size * 4) as usize);
            for p in resized.to_rgba8().pixels() {
                let a = p[3] as u32;
                let r = (p[0] as u32 * a) / 255;
                let g = (p[1] as u32 * a) / 255;
                let b = (p[2] as u32 * a) / 255;
                bgra.extend_from_slice(&[b as u8, g as u8, r as u8, p[3]]);
            }
            Some((target_size as i32, target_size as i32, bgra))
        }
        Err(e) => {
            eprintln!("Failed to load image from memory: {:?}", e);
            None
        }
    }
}

pub unsafe fn add_round_rect(path: *mut GdiplusPath, x: f32, y: f32, w: f32, h: f32, r: f32) {
    let r = r.min(w / 2.0).min(h / 2.0);
    if r <= 0.0 {
        let _ = GdipAddPathRectangle(path, x, y, w, h);
        return;
    }
    let d = r * 2.0;
    let _ = GdipAddPathArc(path, x, y, d, d, 180.0, 90.0);
    let _ = GdipAddPathArc(path, x + w - d, y, d, d, 270.0, 90.0);
    let _ = GdipAddPathArc(path, x + w - d, y + h - d, d, d, 0.0, 90.0);
    let _ = GdipAddPathArc(path, x, y + h - d, d, d, 90.0, 90.0);
    let _ = GdipClosePathFigure(path);
}

pub fn draw_text(hdc: &w::HDC, font: &DeleteObjectGuard<w::HFONT>, color: w::COLORREF, rect: &w::RECT, flags: co::DT, text: &str) -> w::SysResult<i32> {
    let _old_font = hdc.SelectObject(&**font)?;
    let _ = hdc.SetTextColor(color);
    let _ = hdc.SetBkMode(co::BKMODE::TRANSPARENT);
    hdc.DrawText(text, rect, flags)
}

pub unsafe fn draw_logo(
    graphics: *mut GdiplusGraphics,
    rect: w::RECT,
    logo_data: &(i32, i32, Vec<u8>),
    pixel_format: i32,
) {
    let (w_img, h_img, pixels) = logo_data;
    let mut gdip_img = std::ptr::null_mut();
    let _ = GdipCreateBitmapFromScan0(*w_img, *h_img, (*w_img * 4) as i32, pixel_format, Some(pixels.as_ptr()), &mut gdip_img);
    if !gdip_img.is_null() {
        let _ = GdipDrawImageRectRectI(
            graphics,
            gdip_img as _,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            0,
            0,
            *w_img,
            *h_img,
            UnitPixel,
            std::ptr::null_mut(),
            std::mem::transmute(0isize),
            std::ptr::null_mut(),
        );
        let _ = GdipDisposeImage(gdip_img as _);
    }
}

pub unsafe fn draw_close_button(
    graphics: *mut GdiplusGraphics,
    rect: w::RECT,
    dpi: f32,
    is_hover: bool,
    hover_bg: u32,
    icon_color: u32,
    icon_hover: u32,
    corner_radius: f32,
    pad_ratio: f32,
) {
    if is_hover {
        let mut btn_brush = std::ptr::null_mut();
        let _ = GdipCreateSolidFill(hover_bg, &mut btn_brush);
        let mut path_btn = std::ptr::null_mut();
        let _ = GdipCreatePath(FillModeAlternate, &mut path_btn);
        add_round_rect(
            path_btn,
            rect.left as f32,
            rect.top as f32,
            (rect.right - rect.left) as f32,
            (rect.bottom - rect.top) as f32,
            corner_radius * dpi,
        );
        let _ = GdipFillPath(graphics, btn_brush as _, path_btn);
        let _ = GdipDeletePath(path_btn);
        let _ = GdipDeleteBrush(btn_brush as _);
    }

    let mut x_pen = std::ptr::null_mut();
    let close_color = if is_hover { icon_hover } else { icon_color };
    let _ = GdipCreatePen1(close_color, 1.5 * dpi, UnitPixel, &mut x_pen);
    let size = (rect.right - rect.left) as f32;
    let pad = size * pad_ratio;
    let left = rect.left as f32;
    let top = rect.top as f32;
    let _ = GdipDrawLine(graphics, x_pen as _, left + pad, top + pad, left + size - pad, top + size - pad);
    let _ = GdipDrawLine(graphics, x_pen as _, left + size - pad, top + pad, left + pad, top + size - pad);
    let _ = GdipDeletePen(x_pen as _);
}
