use anyhow::{bail, Result};

use image::{load_from_memory, GenericImageView};
use std::{
    cell::RefCell,
    ops::Deref,
    rc::Rc,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};
use winsafe::{self as w, co, gui, prelude::*};

// DWM imports from windows crate
use windows::Win32::Foundation::HWND as WinHWND;
use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
use windows::Win32::UI::Controls::MARGINS;

// GDI imports for CreateDIBSection fallback
use windows::Win32::Graphics::Gdi::{
    CreateDIBSection, GetDC, GetDeviceCaps, GetStockObject, ReleaseDC, RoundRect, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
    HDC as WinHDC, LOGPIXELSY, NULL_PEN,
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
    logo_hbmp: Rc<RefCell<Option<w::HBITMAP>>>,
    title_font: Rc<RefCell<Option<w::HFONT>>>,
    status_font: Rc<RefCell<Option<w::HFONT>>>,
}

impl SplashWindow {
    pub fn new(title: String, status_text: String, rx: Receiver<i16>) -> Result<Self> {
        let w = 300;
        let h = 340;

        let wnd = gui::WindowMain::new(gui::WindowMainOpts {
            class_icon: gui::Icon::Idi(co::IDI::APPLICATION),
            class_cursor: gui::Cursor::Idc(co::IDC::ARROW),
            class_style: co::CS::HREDRAW | co::CS::VREDRAW,
            class_name: "VelopackModernSplashWindow".to_owned(),
            title: title.clone(),
            size: (w as u32, h as u32),
            ex_style: co::WS_EX::APPWINDOW,
            style: co::WS::POPUP | co::WS::VISIBLE | co::WS::THICKFRAME,
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

        let mut new_self =
            Self { wnd, rx, target_progress, visual_progress, title, status_text, logo_data, logo_hbmp, title_font, status_font };
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
            // Center
            let raw_hwnd = self2.wnd.hwnd().ptr();
            let win_hwnd = WinHWND(raw_hwnd);
            let scale = get_dpi_scale(win_hwnd);

            let w_val = (300.0 * scale) as i32;
            let h_val = (340.0 * scale) as i32;

            let screen_cx = w::GetSystemMetrics(co::SM::CXSCREEN);
            let screen_cy = w::GetSystemMetrics(co::SM::CYSCREEN);
            let x = (screen_cx - w_val) / 2;
            let y = (screen_cy - h_val) / 2;

            self2.wnd.hwnd().SetWindowPos(w::HwndPlace::None, w::POINT { x, y }, w::SIZE { cx: w_val, cy: h_val }, co::SWP::NOZORDER)?;

            // DWM Extension
            let raw_hwnd = self2.wnd.hwnd().ptr();
            let win_hwnd = WinHWND(raw_hwnd);
            let margins = MARGINS { cxLeftWidth: 1, cxRightWidth: 1, cyTopHeight: 1, cyBottomHeight: 1 };
            unsafe {
                let _ = DwmExtendFrameIntoClientArea(win_hwnd, &margins);
            }

            self2.wnd.hwnd().SetTimer(TMR_PROGRESS, 16, None)?;
            Ok(0)
        });

        self.wnd.on().wm_nc_hit_test(|_m| Ok(co::HT::CAPTION));

        self.wnd.on().wm_nc_calc_size(|_p| Ok(co::WVR::REDRAW));

        let self2 = self.clone();
        self.wnd.on().wm_timer(TMR_PROGRESS, move || {
            let mut changed = false;
            loop {
                let msg = self2.rx.try_recv().unwrap_or(MSG_NOMESSAGE);
                if msg == MSG_NOMESSAGE {
                    break;
                } else if msg == MSG_CLOSE {
                    self2.wnd.hwnd().SendMessage(w::msg::wm::Close {});
                    return Ok(());
                } else if msg >= 0 {
                    let mut tp = self2.target_progress.borrow_mut();
                    *tp = msg;
                }
            }

            // Animation
            let target = *self2.target_progress.borrow() as f32;
            let mut visual = self2.visual_progress.borrow_mut();
            let diff = target - *visual;
            if diff.abs() > 0.1 {
                *visual += diff * 0.1;
                changed = true;
            } else if diff.abs() > 0.001 {
                *visual = target;
                changed = true;
            }

            if changed {
                self2.wnd.hwnd().InvalidateRect(None, false)?;
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

            let raw_hwnd = self2.wnd.hwnd().ptr();
            let win_hwnd = WinHWND(raw_hwnd);
            let scale = get_dpi_scale(win_hwnd);

            // 2. Logo (Centered, Top Padding)
            let logo_y = (50.0 * scale) as i32;
            let logo_display_size = (64.0 * scale) as i32; // Display at 64x64 * scale

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
                                std::ptr::copy_nonoverlapping(data.as_ptr(), pbits as *mut u8, data.len());
                                *self2.logo_hbmp.borrow_mut() = Some(w::HBITMAP::from_ptr(hbmp.0 as *mut _));
                            }
                        }
                    }
                }

                // Draw if ready
                if let Some(hbmp) = self2.logo_hbmp.borrow().as_ref() {
                    if let Ok(hdc_logo) = hdc.CreateCompatibleDC() {
                        if let Ok(_old) = hdc_logo.SelectObject(hbmp) {
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
                let create_font = |height: i32, weight: co::FW, face_name: &[u16]| -> Option<w::HFONT> {
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
                    .map(|mut g| g.leak())
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
                let _guard = hdc_mem.SelectObject(font)?;
                hdc_mem.DrawText(&self2.title, &title_rc, co::DT::CENTER | co::DT::SINGLELINE | co::DT::NOPREFIX)?;
            }

            // Status Text (Light Gray, Centered Below Title)
            let _ = hdc_mem.SetTextColor(w::COLORREF::new(180, 180, 180));
            let status_rc =
                w::RECT { left: 20, top: text_start_y + title_height, right: w - 20, bottom: text_start_y + title_height + status_height };

            if let Some(font) = self2.status_font.borrow().as_ref() {
                let _guard = hdc_mem.SelectObject(font)?;
                hdc_mem.DrawText(&self2.status_text.borrow(), &status_rc, co::DT::CENTER | co::DT::SINGLELINE | co::DT::NOPREFIX)?;
            }
            // 4. Progress Bar (Rounded, Padding from Bottom)
            let progress = (*self2.visual_progress.borrow() / 100.0).min(1.0).max(0.0);
            let ph = (6.0 * scale) as i32; // Height 6px
            let padding_x = (30.0 * scale) as i32;
            let padding_bottom = (30.0 * scale) as i32;

            let py = h - padding_bottom - ph;
            let bar_w = w - (padding_x * 2);

            let rounded_r = (3.0 * scale) as i32;

            // Use NULL_PEN to ensure no border
            let null_pen = unsafe {
                let ptr = GetStockObject(NULL_PEN);
                w::HPEN::from_ptr(ptr.0 as *mut _)
            };
            let _pen_guard = hdc_mem.SelectObject(&null_pen)?;

            // Track (Darker Gray)
            let track_brush = w::HBRUSH::CreateSolidBrush(w::COLORREF::new(50, 50, 50))?;
            let _brush_guard = hdc_mem.SelectObject(track_brush.deref())?;

            unsafe {
                let _ = RoundRect(WinHDC(hdc_mem.ptr()), padding_x, py, padding_x + bar_w, py + ph, rounded_r, rounded_r);
            }

            // Fill (White/Accent)
            if progress > 0.0 {
                let ind_w = (bar_w as f32 * progress) as i32;
                if ind_w > 0 {
                    let ind_brush = w::HBRUSH::CreateSolidBrush(w::COLORREF::new(255, 255, 255))?;
                    let _brush_guard2 = hdc_mem.SelectObject(ind_brush.deref())?;
                    unsafe {
                        // Overdraw the fill
                        let _ = RoundRect(WinHDC(hdc_mem.ptr()), padding_x, py, padding_x + ind_w, py + ph, rounded_r, rounded_r);
                    }
                }
            }

            hdc.BitBlt(w::POINT::default(), w::SIZE { cx: w, cy: h }, &hdc_mem, w::POINT::default(), co::ROP::SRCCOPY)?;
            Ok(())
        });
    }
}
