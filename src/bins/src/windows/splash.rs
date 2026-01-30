use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
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
use windows::Win32::Graphics::Gdi::{CreateDIBSection, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HDC as WinHDC};

const TMR_PROGRESS: usize = 1;
const MSG_NOMESSAGE: i16 = -99;
const IMAGE_DATA: &str = "data:image/png;base64,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";

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
}

impl SplashWindow {
    pub fn new(title: String, status_text: String, rx: Receiver<i16>) -> Result<Self> {
        let w = 320;
        let h = 120;

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

        // Load Logo from memory (decode base64 first)
        let logo_data = {
            // Remove Data URL prefix if present (e.g., "data:image/png;base64,")
            let base64_str = if let Some(comma_pos) = IMAGE_DATA.find(',') { &IMAGE_DATA[comma_pos + 1..] } else { IMAGE_DATA };

            match BASE64.decode(base64_str) {
                Ok(image_bytes) => match load_from_memory(&image_bytes) {
                    Ok(img) => {
                        let (_w, _h) = img.dimensions();
                        let resized = img.resize_exact(96, 96, image::imageops::FilterType::Lanczos3);
                        let mut bgra = Vec::with_capacity((96 * 96 * 4) as usize);
                        for p in resized.to_rgba8().pixels() {
                            let a = p[3] as u32;
                            let r = (p[0] as u32 * a) / 255;
                            let g = (p[1] as u32 * a) / 255;
                            let b = (p[2] as u32 * a) / 255;
                            bgra.extend_from_slice(&[b as u8, g as u8, r as u8, p[3]]);
                        }
                        Some((96, 96, bgra))
                    }
                    Err(e) => {
                        eprintln!("Failed to load image from memory: {:?}", e);
                        None
                    }
                },
                Err(e) => {
                    eprintln!("Failed to decode base64 image data: {:?}", e);
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

        let mut new_self = Self { wnd, rx, target_progress, visual_progress, title, status_text, logo_data, logo_hbmp };
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
            let screen_cx = w::GetSystemMetrics(co::SM::CXSCREEN);
            let screen_cy = w::GetSystemMetrics(co::SM::CYSCREEN);
            let w_val = 480;
            let h_val = 240;
            let x = (screen_cx - w_val) / 2;
            let y = (screen_cy - h_val) / 2;

            self2.wnd.hwnd().SetWindowPos(
                w::HwndPlace::None,
                w::POINT { x, y },
                w::SIZE { cx: 0, cy: 0 },
                co::SWP::NOSIZE | co::SWP::NOZORDER,
            )?;

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

            // 1. Background (Neutral 950)
            let bg_brush = w::HBRUSH::CreateSolidBrush(w::COLORREF::new(10, 10, 10))?;
            hdc_mem.FillRect(rect, bg_brush.deref())?;

            // 2. Logo (at 30, 30)
            let start_text_x = if let Some((lw, lh, data)) = self2.logo_data.as_ref() {
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
                    // Use windows crate CreateDIBSection with Some(hdc)
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
                            unsafe {
                                let _ = WinAlphaBlend(
                                    WinHDC(hdc_mem.ptr()),
                                    30,
                                    30,
                                    *lw,
                                    *lh,
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
                // Text starts after logo (30 + 96 + 20 gap)
                146
            } else {
                40
            };

            // 3. Text
            let _ = hdc_mem.SetBkMode(co::BKMODE::TRANSPARENT);
            let sys_font = w::HFONT::GetStockObject(co::STOCK_FONT::DEFAULT_GUI)?;
            let _old_font = hdc_mem.SelectObject(&sys_font)?;

            // 应用标题(白色)
            let _ = hdc_mem.SetTextColor(w::COLORREF::new(255, 255, 255));
            hdc_mem.DrawText(
                &self2.title,
                &w::RECT { left: start_text_x, top: 40, right: w - 40, bottom: 80 },
                co::DT::LEFT | co::DT::SINGLELINE | co::DT::NOPREFIX,
            )?;

            // Status Text (Neutral 400)
            let _ = hdc_mem.SetTextColor(w::COLORREF::new(163, 163, 163));
            hdc_mem.DrawText(
                &self2.status_text.borrow(),
                &w::RECT { left: start_text_x, top: 80, right: w - 40, bottom: 110 },
                co::DT::LEFT | co::DT::SINGLELINE | co::DT::NOPREFIX,
            )?;

            // 4. Progress Bar (底部占满，无圆角)
            let progress = (*self2.visual_progress.borrow() / 100.0).min(1.0).max(0.0);
            let ph = 40; // 进度条高度
            let py = h - ph; // 位于底部

            // 绘制进度条轨道（背景）
            let track_brush = w::HBRUSH::CreateSolidBrush(w::COLORREF::new(38, 38, 38))?;
            hdc_mem.FillRect(w::RECT { left: 0, top: py, right: w, bottom: h }, track_brush.deref())?;

            // 绘制进度条填充
            if progress > 0.0 {
                let ind_w = (w as f32 * progress) as i32;
                if ind_w > 0 {
                    let ind_brush = w::HBRUSH::CreateSolidBrush(w::COLORREF::new(255, 255, 255))?;
                    hdc_mem.FillRect(w::RECT { left: 0, top: py, right: ind_w, bottom: h }, ind_brush.deref())?;
                }
            }

            hdc.BitBlt(w::POINT::default(), w::SIZE { cx: w, cy: h }, &hdc_mem, w::POINT::default(), co::ROP::SRCCOPY)?;
            Ok(())
        });
    }
}
