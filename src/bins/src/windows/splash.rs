use anyhow::{bail, Result};

use image::{load_from_memory, GenericImageView};
use std::{
    cell::RefCell,
    ops::Deref,
    rc::Rc,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};
use winsafe::{self as w, co, guard::DeleteObjectGuard, gui, prelude::*};

// DWM imports from windows crate
use windows::core::BOOL;
use windows::Win32::Foundation::HWND as WinHWND;
use windows::Win32::Graphics::Dwm::{DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows::Win32::UI::Controls::MARGINS;

// GDI imports for CreateDIBSection fallback
use windows::Win32::Foundation::POINT as WinPOINT;
use windows::Win32::Graphics::Gdi::{
    BeginPath, CreateDIBSection, EndPath, FillPath, GetDC, GetDeviceCaps, GetStockObject, LineTo, MoveToEx, PolyBezierTo, ReleaseDC,
    RoundRect, SetBrushOrgEx, SetStretchBltMode, StretchBlt, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HALFTONE, HDC as WinHDC,
    LOGPIXELSY, NULL_PEN, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS};

const TMR_PROGRESS: usize = 1;
const MSG_NOMESSAGE: i16 = -99;
const IMAGE_DATA: &[u8] = include_bytes!("../assets/product_logo.png");

pub const MSG_CLOSE: i16 = -1;
pub const MSG_INDEFINITE: i16 = -2;

pub fn show_progress_dialog<T1: AsRef<str>, T2: AsRef<str>>(window_title: T1, content: T2) -> Sender<i16> {
    let window_title = window_title.as_ref().to_string();
    let content = content.as_ref().to_string();
    let (tx, rx) = mpsc::channel::<i16>();
    thread::spawn(move || {
        let _ = SplashWindow::new(window_title, content, rx).and_then(|w| {
            w.run()?;
            Ok(())
        });
    });
    tx
}

pub fn show_splash_dialog(app_name: String, _imgstream: Option<Vec<u8>>) -> Sender<i16> {
    let content = format!("安装中 {}...", app_name);
    show_progress_dialog(app_name, content)
}

fn get_dpi_scale(hwnd: WinHWND) -> f32 {
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

#[derive(Clone)]
pub struct SplashWindow {
    wnd: gui::WindowMain,
    rx: Rc<Receiver<i16>>,
    target_progress: Rc<RefCell<i16>>,
    visual_progress: Rc<RefCell<f32>>,
    title: String,
    status_text: Rc<RefCell<String>>,
    // Cache for Logo BGRA data: (width, height, pixels)
    logo_data: Rc<Option<(i32, i32, Vec<u8>)>>,
    // Cache for GDI Object
    logo_hbmp: Rc<RefCell<Option<DeleteObjectGuard<w::HBITMAP>>>>,
    title_font: Rc<RefCell<Option<DeleteObjectGuard<w::HFONT>>>>,
    status_font: Rc<RefCell<Option<DeleteObjectGuard<w::HFONT>>>>,
    dpi_scale: Rc<RefCell<f32>>,
    is_close_hovered: Rc<RefCell<bool>>,
    should_close: Rc<RefCell<bool>>,
    close_delay_ticks: Rc<RefCell<i32>>,
}

impl SplashWindow {
    pub fn new(title: String, status_text: String, rx: Receiver<i16>) -> Result<Self> {
        let w = 300;
        let h = 340;

        let wnd = gui::WindowMain::new(gui::WindowMainOpts {
            class_icon: gui::Icon::Idi(co::IDI::APPLICATION),
            class_style: co::CS::HREDRAW | co::CS::VREDRAW,
            class_name: "VelopackModernSplashWindow".to_owned(),
            title: title.clone(),
            size: (w as u32, h as u32),
            // WS_EX_TOOLWINDOW hides it from the taskbar and Alt-Tab
            ex_style: co::WS_EX::TOOLWINDOW | co::WS_EX::TOPMOST,
            style: co::WS::POPUP | co::WS::THICKFRAME,
            ..Default::default()
        });

        // Load Logo from memory
        let logo_data = {
            match load_from_memory(IMAGE_DATA) {
                Ok(img) => {
                    let (_w, _h) = img.dimensions();
                    // Resize to a larger size (256x256) to allow high-quality downscaling
                    let target_size = 256;
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
        };

        let rx = Rc::new(rx);
        let target_progress = Rc::new(RefCell::new(0));
        let visual_progress = Rc::new(RefCell::new(0.0));
        let status_text = Rc::new(RefCell::new(status_text));
        let logo_data = Rc::new(logo_data);
        let logo_hbmp = Rc::new(RefCell::new(None));
        let title_font = Rc::new(RefCell::new(None));
        let status_font = Rc::new(RefCell::new(None));
        let dpi_scale = Rc::new(RefCell::new(1.0));
        let is_close_hovered = Rc::new(RefCell::new(false));

        let mut new_self = Self {
            wnd,
            rx,
            target_progress,
            visual_progress,
            title,
            status_text,
            logo_data,
            logo_hbmp,
            title_font,
            status_font,
            dpi_scale,
            is_close_hovered,
            should_close: Rc::new(RefCell::new(false)),
            close_delay_ticks: Rc::new(RefCell::new(20)), // 60fps 下 20 ticks 约为 320ms
        };
        new_self.events();
        Ok(new_self)
    }

    pub fn run(&self) -> Result<i32> {
        let res = self.wnd.run_main(None);
        if res.is_err() {
            bail!("Error Showing Window: {:?}", res);
        }
        Ok(res.unwrap())
    }

    fn events(&mut self) {
        let self2 = self.clone();
        self.wnd.on().wm_create(move |_m| {
            let raw_hwnd = self2.wnd.hwnd().ptr();
            let win_hwnd = WinHWND(raw_hwnd);

            // 1. DWM Extension and Dark Mode (Do this before moving/showing)
            unsafe {
                let margins = MARGINS { cxLeftWidth: 0, cxRightWidth: 0, cyTopHeight: 1, cyBottomHeight: 0 };
                let _ = DwmExtendFrameIntoClientArea(win_hwnd, &margins);
                let use_dark_mode = BOOL::from(true);
                let _ = DwmSetWindowAttribute(
                    win_hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    &use_dark_mode as *const _ as *const _,
                    std::mem::size_of::<BOOL>() as u32,
                );
            }

            // 2. Position and Size (Use NOREDRAW/HIDEWINDOW if needed, but here we just want it ready)
            let scale = get_dpi_scale(win_hwnd);
            *self2.dpi_scale.borrow_mut() = scale;

            let w_val = (300.0 * scale) as i32;
            let h_val = (340.0 * scale) as i32;

            let screen_cx = w::GetSystemMetrics(co::SM::CXSCREEN);
            let screen_cy = w::GetSystemMetrics(co::SM::CYSCREEN);
            let x = (screen_cx - w_val) / 2;
            let y = (screen_cy - h_val) / 2;

            self2.wnd.hwnd().SetWindowPos(
                w::HwndPlace::None,
                w::POINT { x, y },
                w::SIZE { cx: w_val, cy: h_val },
                co::SWP::NOZORDER | co::SWP::NOACTIVATE,
            )?;

            self2.wnd.hwnd().SetTimer(TMR_PROGRESS, 16, None)?; // ~60 FPS

            // 3. Finally show and focus
            self2.wnd.hwnd().ShowWindow(co::SW::SHOW);
            self2.wnd.hwnd().SetForegroundWindow();
            Ok(0)
        });

        let self2 = self.clone();
        self.wnd.on().wm_nc_hit_test(move |p| {
            let mut pt = p.cursor_pos;
            self2.wnd.hwnd().ScreenToClient(&mut pt)?;

            let scale = *self2.dpi_scale.borrow();

            let hwnd = self2.wnd.hwnd();
            let rect = hwnd.GetClientRect()?;
            let w = rect.right - rect.left;

            let close_size = (32.0 * scale) as i32;
            let close_rect = w::RECT { left: w - close_size, top: 0, right: w, bottom: close_size };

            if pt.x >= close_rect.left && pt.x <= close_rect.right && pt.y >= close_rect.top && pt.y <= close_rect.bottom {
                Ok(co::HT::CLIENT)
            } else {
                Ok(co::HT::CAPTION)
            }
        });

        self.wnd.on().wm_nc_calc_size(|_p| Ok(co::WVR::REDRAW));

        let self2 = self.clone();
        self.wnd.on().wm_erase_bkgnd(move |_| {
            Ok(1) // 返回非 0 表示背景已擦除，防止系统默认擦除（导致白闪）
        });

        self.wnd.on().wm_timer(TMR_PROGRESS, move || {
            let mut changed = false;
            loop {
                let msg = self2.rx.try_recv().unwrap_or(MSG_NOMESSAGE);
                if msg == MSG_NOMESSAGE {
                    break;
                } else if msg == MSG_CLOSE {
                    *self2.should_close.borrow_mut() = true;
                } else if msg >= 0 {
                    let mut tp = self2.target_progress.borrow_mut();
                    *tp = msg;
                }
            }

            {
                // Animation
                let target = *self2.target_progress.borrow() as f32;
                let mut visual = self2.visual_progress.borrow_mut();
                let diff = target - *visual;

                if diff.abs() > 0.01 {
                    // 更加平滑的插值：基于距离的比例 + 最小步长，确保不会在末尾停滞
                    let step = (diff * 0.15).abs().max(0.1);
                    if diff > 0.0 {
                        *visual = (*visual + step).min(target);
                    } else {
                        *visual = (*visual - step).max(target);
                    }
                    changed = true;
                }
            }

            if changed {
                self2.wnd.hwnd().InvalidateRect(None, false)?;
            }

            let should_close = self2.should_close.borrow();
            if *should_close {
                let visual = *self2.visual_progress.borrow();
                let target = *self2.target_progress.borrow() as f32;
                // 确保动画基本完成 (>= 99.9)
                if target >= 100.0 && visual >= 99.9 {
                    let mut delay = self2.close_delay_ticks.borrow_mut();
                    if *delay <= 0 {
                        self2.wnd.hwnd().SendMessage(w::msg::wm::Close {});
                    } else {
                        *delay -= 1;
                    }
                } else if target < 100.0 && (target - visual).abs() < 1.0 {
                    // 对于非 100% 的提前关闭（如果有），直接关闭
                    self2.wnd.hwnd().SendMessage(w::msg::wm::Close {});
                }
            }
            Ok(())
        });

        let self2 = self.clone();
        self.wnd.on().wm_paint(move || {
            let hwnd = self2.wnd.hwnd();
            let rect = hwnd.GetClientRect()?;
            let hdc = hwnd.BeginPaint()?;
            let w = rect.right - rect.left;
            let h = rect.bottom - rect.top;

            let hdc_mem = hdc.CreateCompatibleDC()?;
            let buffer_bmp = hdc.CreateCompatibleBitmap(w, h)?;
            let _buffer_old = hdc_mem.SelectObject(buffer_bmp.deref())?;

            // 1. Background (Dark Grey #1E1E1E -> 30, 30, 30)
            let bg_brush = w::HBRUSH::CreateSolidBrush(w::COLORREF::new(30, 30, 30))?;
            hdc_mem.FillRect(rect, bg_brush.deref())?;

            let scale = *self2.dpi_scale.borrow();
            // Recalculate if needed (optional, but keep it simple for now)
            // let scale = get_dpi_scale(win_hwnd);

            // 2. Logo (Centered, Top Padding)
            let logo_y = (50.0 * scale) as i32;
            let logo_display_size = (86.0 * scale) as i32; // Display at 64x64 * scale

            if let Some((lw, lh, data)) = self2.logo_data.as_ref() {
                // Initialize HBITMAP if needed
                if self2.logo_hbmp.borrow().is_none() {
                    let bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: *lw,
                            biHeight: -*lh,
                            biPlanes: 1,
                            biBitCount: 32,
                            biCompression: 0, // BI_RGB
                            biSizeImage: 0,
                            biXPelsPerMeter: 0,
                            biYPelsPerMeter: 0,
                            biClrUsed: 0,
                            biClrImportant: 0,
                        },
                        bmiColors: [Default::default()],
                    };

                    let mut pbits: *mut std::ffi::c_void = std::ptr::null_mut();
                    unsafe {
                        let win_hdc = WinHDC(hdc.ptr());
                        if let Ok(hbmp) = CreateDIBSection(Some(win_hdc), &bmi, DIB_RGB_COLORS, &mut pbits, None, 0) {
                            if !pbits.is_null() {
                                let expected_len = (*lw * *lh * 4) as usize;
                                if data.len() >= expected_len {
                                    std::ptr::copy_nonoverlapping(data.as_ptr(), pbits as *mut u8, expected_len);
                                    let hbmp = w::HBITMAP::from_ptr(hbmp.0 as *mut _);
                                    *self2.logo_hbmp.borrow_mut() = Some(DeleteObjectGuard::new(hbmp));
                                }
                            }
                        }
                    }
                }

                // Draw if ready
                if let Some(hbmp) = self2.logo_hbmp.borrow().as_ref() {
                    if let Ok(hdc_logo) = hdc.CreateCompatibleDC() {
                        if let Ok(_old) = hdc_logo.SelectObject(&**hbmp) {
                            use windows::Win32::Graphics::Gdi::{AlphaBlend as WinAlphaBlend, BLENDFUNCTION};
                            let blend_fn = BLENDFUNCTION { BlendOp: 0, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: 1 };
                            let logo_x = (w - logo_display_size) / 2;
                            unsafe {
                                let _ = WinAlphaBlend(
                                    WinHDC(hdc_mem.ptr()),
                                    logo_x,
                                    logo_y,
                                    logo_display_size,
                                    logo_display_size,
                                    WinHDC(hdc_logo.ptr()),
                                    0,
                                    0,
                                    *lw,
                                    *lh,
                                    blend_fn,
                                );
                            }
                        }
                    }
                }
            }

            // 3. Text (Centered, Below Logo)
            let _ = hdc_mem.SetBkMode(co::BKMODE::TRANSPARENT);

            // Fetch System Font (e.g. Segoe UI) using SystemParametersInfo
            // This ensures we use the correct modern UI font rather than legacy MS Sans Serif
            if self2.title_font.borrow().is_none() {
                let mut ncm = NONCLIENTMETRICSW::default();
                ncm.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
                unsafe {
                    let _ =
                        SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, ncm.cbSize, Some(&mut ncm as *mut _ as *mut _), Default::default());
                }

                // Helper to create font from system metric name but with custom size/weight
                let create_font = |height: i32, weight: co::FW, face_name: &[u16]| -> Option<DeleteObjectGuard<w::HFONT>> {
                    let len = face_name.iter().position(|&c| c == 0).unwrap_or(face_name.len());
                    let face_name_str = String::from_utf16_lossy(&face_name[..len]);

                    w::HFONT::CreateFont(
                        w::SIZE { cx: 0, cy: height },
                        0,
                        0, // Escapement, Orientation
                        weight,
                        false,
                        false,
                        false,
                        co::CHARSET::DEFAULT,
                        co::OUT_PRECIS::DEFAULT,
                        co::CLIP::DEFAULT_PRECIS,
                        co::QUALITY::CLEARTYPE,
                        co::PITCH::DEFAULT,
                        &face_name_str,
                    )
                    .ok()
                };

                // Create Title Font (Bold, Larger) - Using system font face from ncm.lfMessageFont
                if let Some(h) = create_font((-18.0 * scale) as i32, co::FW::BOLD, &ncm.lfMessageFont.lfFaceName) {
                    *self2.title_font.borrow_mut() = Some(h);
                }

                // Create Status Font (Normal, Standard)
                if let Some(h) = create_font((-13.0 * scale) as i32, co::FW::NORMAL, &ncm.lfMessageFont.lfFaceName) {
                    *self2.status_font.borrow_mut() = Some(h);
                }
            }

            let text_start_y = logo_y + logo_display_size + (30.0 * scale) as i32;
            let title_height = (30.0 * scale) as i32;
            let status_height = (30.0 * scale) as i32;

            // Title (White, Centered)
            let _ = hdc_mem.SetTextColor(w::COLORREF::new(255, 255, 255));
            let title_rc = w::RECT { left: 0, top: text_start_y, right: w, bottom: text_start_y + title_height };

            if let Some(font) = self2.title_font.borrow().as_ref() {
                let _guard = hdc_mem.SelectObject(&**font)?;
                hdc_mem.DrawText(&self2.title, &title_rc, co::DT::CENTER | co::DT::SINGLELINE | co::DT::NOPREFIX)?;
            }

            // Status Text (Light Gray, Centered Below Title)
            let _ = hdc_mem.SetTextColor(w::COLORREF::new(180, 180, 180));
            let status_rc =
                w::RECT { left: 20, top: text_start_y + title_height, right: w - 20, bottom: text_start_y + title_height + status_height };

            if let Some(font) = self2.status_font.borrow().as_ref() {
                let _guard = hdc_mem.SelectObject(&**font)?;
                hdc_mem.DrawText(&self2.status_text.borrow(), &status_rc, co::DT::CENTER | co::DT::SINGLELINE | co::DT::NOPREFIX)?;
            }
            // 4. Progress Bar (Rounded, Padding from Bottom) - Super-Sampled for Anti-Aliasing
            let progress = (*self2.visual_progress.borrow() / 100.0).min(1.0).max(0.0);
            let ph = (10.0 * scale) as i32; // Height 10px
            let padding_x = (30.0 * scale) as i32;
            let padding_bottom = (30.0 * scale) as i32;

            let py = h - padding_bottom - ph;
            let bar_w = w - (padding_x * 2);
            let rounded_r = (10.0 * scale) as i32;

            // Super-sampling factor (4x)
            let ss_factor = 4;
            let ss_w = bar_w * ss_factor;
            let ss_h = ph * ss_factor;
            let ss_r = rounded_r * ss_factor;

            // Create High-Res DC and Bitmap
            if let Ok(hdc_ss) = hdc.CreateCompatibleDC() {
                if let Ok(bmp_ss) = hdc.CreateCompatibleBitmap(ss_w, ss_h) {
                    if let Ok(_old_ss) = hdc_ss.SelectObject(bmp_ss.deref()) {
                        // Fill Background (same as main bg)
                        let bg_brush_ss = w::HBRUSH::CreateSolidBrush(w::COLORREF::new(30, 30, 30))?;
                        let _ = hdc_ss.FillRect(w::RECT { left: 0, top: 0, right: ss_w, bottom: ss_h }, bg_brush_ss.deref());

                        // Use NULL_PEN
                        let null_pen = unsafe {
                            let ptr = GetStockObject(NULL_PEN);
                            w::HPEN::from_ptr(ptr.0 as *mut _)
                        };
                        let _pen_guard = hdc_ss.SelectObject(&null_pen);

                        // Draw Track (Darker Gray)
                        let track_brush = w::HBRUSH::CreateSolidBrush(w::COLORREF::new(50, 50, 50))?;
                        let _brush_guard = hdc_ss.SelectObject(track_brush.deref());

                        unsafe {
                            let _ = RoundRect(WinHDC(hdc_ss.ptr()), 0, 0, ss_w, ss_h, ss_r, ss_r);
                        }

                        // Draw Fill (White/Accent)
                        if progress > 0.0 {
                            let ss_ind_w = (ss_w as f32 * progress) as i32;
                            if ss_ind_w > 0 {
                                let ind_brush = w::HBRUSH::CreateSolidBrush(w::COLORREF::new(255, 255, 255))?;
                                let _brush_guard2 = hdc_ss.SelectObject(ind_brush.deref());
                                unsafe {
                                    let _ = RoundRect(WinHDC(hdc_ss.ptr()), 0, 0, ss_ind_w, ss_h, ss_r, ss_r);
                                }
                            }
                        }

                        // Downscale with HALFTONE
                        unsafe {
                            let _ = SetStretchBltMode(WinHDC(hdc_mem.ptr()), HALFTONE);
                            let _ = SetBrushOrgEx(WinHDC(hdc_mem.ptr()), 0, 0, None); // Align pattern
                            let _ = StretchBlt(
                                WinHDC(hdc_mem.ptr()),
                                padding_x,
                                py,
                                bar_w,
                                ph,
                                Some(WinHDC(hdc_ss.ptr())),
                                0,
                                0,
                                ss_w,
                                ss_h,
                                SRCCOPY,
                            );
                        }
                    }
                }
            }

            // 5. Close Button (Top Right) - Graphical "X" from SVG
            let scale = *self2.dpi_scale.borrow();
            let close_size = (32.0 * scale) as i32;
            let close_rect = w::RECT { left: w - close_size, top: 0, right: w, bottom: close_size };
            let is_hovered = *self2.is_close_hovered.borrow();

            if is_hovered {
                let hover_brush = w::HBRUSH::CreateSolidBrush(w::COLORREF::new(232, 17, 35))?; // Windows close red
                hdc_mem.FillRect(close_rect, hover_brush.deref())?;
            }

            // Draw SVG-based "X" centered in close_rect
            let icon_display_size = (10.0 * scale) as i32;
            let icon_x = close_rect.left + (close_size - icon_display_size) / 2;
            let icon_y = close_rect.top + (close_size - icon_display_size) / 2;

            // Super-sampling factor (4x) for anti-aliasing
            let ss_factor = 4;
            let ss_size = icon_display_size * ss_factor;

            if let Ok(hdc_ss) = hdc.CreateCompatibleDC() {
                if let Ok(bmp_ss) = hdc.CreateCompatibleBitmap(ss_size, ss_size) {
                    if let Ok(_old_ss) = hdc_ss.SelectObject(bmp_ss.deref()) {
                        // Fill Background (same as button bg)
                        let bg_color = if is_hovered { w::COLORREF::new(232, 17, 35) } else { w::COLORREF::new(30, 30, 30) };
                        let bg_brush_ss = w::HBRUSH::CreateSolidBrush(bg_color).unwrap();
                        let _ = hdc_ss.FillRect(w::RECT { left: 0, top: 0, right: ss_size, bottom: ss_size }, bg_brush_ss.deref());

                        let win_hdc_ss = WinHDC(hdc_ss.ptr());
                        let draw_ss = |v: f32| (((v - 3.0) / 10.0) * ss_size as f32) as i32;
                        let p_ss = |x: f32, y: f32| WinPOINT { x: draw_ss(x), y: draw_ss(y) };

                        unsafe {
                            let _ = BeginPath(win_hdc_ss);
                            let _ = MoveToEx(win_hdc_ss, draw_ss(8.0), draw_ss(8.70801), None);
                            let _ = LineTo(win_hdc_ss, draw_ss(3.85449), draw_ss(12.8535));
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(3.75684, 12.9512), p_ss(3.63965, 13.0), p_ss(3.50293, 13.0)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(3.3597, 13.0), p_ss(3.23926, 12.9528), p_ss(3.1416, 12.8584)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(3.0472, 12.7607), p_ss(3.0, 12.6403), p_ss(3.0, 12.4971)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(3.0, 12.3604), p_ss(3.04883, 12.2432), p_ss(3.14648, 12.1455)]);
                            let _ = LineTo(win_hdc_ss, draw_ss(7.29199), draw_ss(8.0));
                            let _ = LineTo(win_hdc_ss, draw_ss(3.14648), draw_ss(3.85449));
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(3.04883, 3.75684), p_ss(3.0, 3.63802), p_ss(3.0, 3.49805)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(3.0, 3.42969), p_ss(3.01302, 3.36458), p_ss(3.03906, 3.30273)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(3.0651, 3.24089), p_ss(3.10091, 3.1888), p_ss(3.14648, 3.14648)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(3.19206, 3.10091), p_ss(3.24577, 3.0651), p_ss(3.30762, 3.03906)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(3.36947, 3.01302), p_ss(3.43457, 3.0), p_ss(3.50293, 3.0)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(3.63965, 3.0), p_ss(3.75684, 3.04883), p_ss(3.85449, 3.14648)]);
                            let _ = LineTo(win_hdc_ss, draw_ss(8.0), draw_ss(7.29199));
                            let _ = LineTo(win_hdc_ss, draw_ss(12.1455), draw_ss(3.14648));
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(12.2432, 3.04883), p_ss(12.362, 3.0), p_ss(12.502, 3.0)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(12.5703, 3.0), p_ss(12.6338, 3.01302), p_ss(12.6924, 3.03906)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(12.7542, 3.0651), p_ss(12.8079, 3.10091), p_ss(12.8535, 3.14648)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(12.8991, 3.19206), p_ss(12.9349, 3.24577), p_ss(12.9609, 3.30762)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(12.987, 3.36621), p_ss(13.0, 3.42969), p_ss(13.0, 3.49805)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(13.0, 3.63802), p_ss(12.9512, 3.75684), p_ss(12.8535, 3.85449)]);
                            let _ = LineTo(win_hdc_ss, draw_ss(8.70801), draw_ss(8.0));
                            let _ = LineTo(win_hdc_ss, draw_ss(12.8535), draw_ss(12.1455));
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(12.9512, 12.2432), p_ss(13.0, 12.3604), p_ss(13.0, 12.4971)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(13.0, 12.5654), p_ss(12.987, 12.6305), p_ss(12.9609, 12.6924)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(12.9349, 12.7542), p_ss(12.8991, 12.8079), p_ss(12.8535, 12.8535)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(12.8112, 12.8991), p_ss(12.7591, 12.9349), p_ss(12.6973, 12.9609)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(12.6354, 12.987), p_ss(12.5703, 13.0), p_ss(12.502, 13.0)]);
                            let _ = PolyBezierTo(win_hdc_ss, &[p_ss(12.362, 13.0), p_ss(12.2432, 12.9512), p_ss(12.1455, 12.8535)]);
                            let _ = LineTo(win_hdc_ss, draw_ss(8.0), draw_ss(8.70801));
                            let _ = EndPath(win_hdc_ss);

                            let brush = w::HBRUSH::CreateSolidBrush(w::COLORREF::new(255, 255, 255)).unwrap();
                            let _old_brush = hdc_ss.SelectObject(brush.deref()).unwrap();
                            let _ = FillPath(win_hdc_ss);

                            // Downscale with HALFTONE
                            let _ = SetStretchBltMode(WinHDC(hdc_mem.ptr()), HALFTONE);
                            let _ = SetBrushOrgEx(WinHDC(hdc_mem.ptr()), 0, 0, None);
                            let _ = StretchBlt(
                                WinHDC(hdc_mem.ptr()),
                                icon_x,
                                icon_y,
                                icon_display_size,
                                icon_display_size,
                                Some(win_hdc_ss),
                                0,
                                0,
                                ss_size,
                                ss_size,
                                SRCCOPY,
                            );
                        }
                    }
                }
            }

            hdc.BitBlt(w::POINT::default(), w::SIZE { cx: w, cy: h }, &hdc_mem, w::POINT::default(), co::ROP::SRCCOPY)?;
            Ok(())
        });

        // Handle Close Button Click
        let self2 = self.clone();
        self.wnd.on().wm_l_button_up(move |p| {
            let scale = *self2.dpi_scale.borrow();

            let hwnd = self2.wnd.hwnd();
            let rect = hwnd.GetClientRect()?;
            let w = rect.right - rect.left;

            let close_size = (32.0 * scale) as i32;
            let close_rect = w::RECT { left: w - close_size, top: 0, right: w, bottom: close_size };

            let pt = p.coords;
            if pt.x >= close_rect.left && pt.x <= close_rect.right && pt.y >= close_rect.top && pt.y <= close_rect.bottom {
                self2.wnd.hwnd().SendMessage(w::msg::wm::Close {});
            }
            Ok(())
        });

        let self2 = self.clone();
        self.wnd.on().wm_mouse_move(move |p| {
            let scale = *self2.dpi_scale.borrow();
            let rect = self2.wnd.hwnd().GetClientRect()?;
            let w = rect.right - rect.left;

            let close_size = (32.0 * scale) as i32;
            let close_rect = w::RECT { left: w - close_size, top: 0, right: w, bottom: close_size };

            let pt = p.coords;
            let is_in = pt.x >= close_rect.left && pt.x <= close_rect.right && pt.y >= close_rect.top && pt.y <= close_rect.bottom;

            let mut hovered = self2.is_close_hovered.borrow_mut();
            if *hovered != is_in {
                *hovered = is_in;
                self2.wnd.hwnd().InvalidateRect(Some(&close_rect), false)?;
            }

            // Track for WM_MOUSELEAVE
            let mut tme = w::TRACKMOUSEEVENT::default();
            tme.dwFlags = co::TME::LEAVE;
            tme.hwndTrack = unsafe { w::HWND::from_ptr(self2.wnd.hwnd().ptr()) };
            let _ = w::TrackMouseEvent(&mut tme);

            Ok(())
        });

        let self2 = self.clone();
        self.wnd.on().wm_mouse_leave(move || {
            let mut hovered = self2.is_close_hovered.borrow_mut();
            if *hovered {
                *hovered = false;
                let scale = *self2.dpi_scale.borrow();
                let hwnd = self2.wnd.hwnd();
                let rect = hwnd.GetClientRect()?;
                let w = rect.right - rect.left;
                let close_size = (32.0 * scale) as i32;
                let close_rect = w::RECT { left: w - close_size, top: 0, right: w, bottom: close_size };
                hwnd.InvalidateRect(Some(&close_rect), false)?;
            }
            Ok(())
        });
    }
}
