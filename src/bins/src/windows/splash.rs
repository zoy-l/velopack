use anyhow::{bail, Result};

use image::{load_from_memory, GenericImageView};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};
use winsafe::{self as w, co, guard::DeleteObjectGuard, gui, prelude::*};

// DWM imports from windows crate
use windows::core::BOOL;
use windows::Win32::Foundation::HWND as WinHWND;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows::Win32::Graphics::GdiPlus::{
    FillModeAlternate, GdipAddPathArc, GdipAddPathRectangle, GdipClosePathFigure, GdipCreateBitmapFromScan0, GdipCreateFromHDC,
    GdipCreatePath, GdipCreatePen1, GdipCreateSolidFill, GdipDeleteBrush, GdipDeleteGraphics, GdipDeletePath, GdipDeletePen,
    GdipDisposeImage, GdipDrawImageRectRectI, GdipDrawLine, GdipFillPath, GdipFillRectangle, GdipGraphicsClear, GdipSetSmoothingMode,
    GdiplusShutdown, GdiplusStartup, GdiplusStartupInput, GpPath as GdiplusPath, SmoothingModeAntiAlias, UnitPixel,
};

const PIXEL_FORMAT32BPP_ARGB: i32 = 0x26200a;

// GDI imports
use windows::Win32::Graphics::Gdi::{CreateFontIndirectW, GetDC, GetDeviceCaps, ReleaseDC, LOGPIXELSY};
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
            class_style: co::CS::HREDRAW | co::CS::VREDRAW | co::CS::DROPSHADOW,
            class_name: "VelopackModernSplashWindow".to_owned(),
            title: title.clone(),
            size: (w as u32, h as u32),
            // WS_EX_TOOLWINDOW hides it from the taskbar and Alt-Tab
            ex_style: co::WS_EX::TOOLWINDOW | co::WS_EX::TOPMOST,
            style: co::WS::POPUP,
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
        let mut token: usize = 0;
        let input = GdiplusStartupInput { GdiplusVersion: 1, ..Default::default() };
        unsafe {
            let _ = GdiplusStartup(&mut token, &input, std::ptr::null_mut());
        }

        let res = self.wnd.run_main(None);

        unsafe {
            GdiplusShutdown(token);
        }

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

            // 1. DWM Dark Mode and Corners (Do this before moving/showing)
            unsafe {
                let use_dark_mode = BOOL::from(true);
                let _ = DwmSetWindowAttribute(
                    win_hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    &use_dark_mode as *const _ as *const _,
                    std::mem::size_of::<BOOL>() as u32,
                );

                // DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_ROUND = 2
                let corner_preference = 2i32;
                let _ = DwmSetWindowAttribute(
                    win_hwnd,
                    windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE(33),
                    &corner_preference as *const _ as *const _,
                    std::mem::size_of::<i32>() as u32,
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

            // 3. Initialize Fonts from System Metrics
            unsafe {
                let mut ncm = NONCLIENTMETRICSW { cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32, ..Default::default() };
                let _ = SystemParametersInfoW(
                    SPI_GETNONCLIENTMETRICS,
                    ncm.cbSize,
                    Some(&mut ncm as *mut _ as *mut _),
                    windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
                );

                let mut lf_title = ncm.lfMessageFont;
                // lfHeight = -((font_size_pt * 96.0 / 72.0) * scale)
                lf_title.lfHeight = -((20.0 * 96.0 / 72.0) * scale) as i32;
                lf_title.lfWeight = 700; // Bold
                let hfont_title = CreateFontIndirectW(&lf_title);
                if !hfont_title.0.is_null() {
                    *self2.title_font.borrow_mut() = Some(DeleteObjectGuard::new(w::HFONT::from_ptr(hfont_title.0)));
                }

                let mut lf_status = ncm.lfMessageFont;
                lf_status.lfHeight = -((12.0 * 96.0 / 72.0) * scale) as i32;
                let hfont_status = CreateFontIndirectW(&lf_status);
                if !hfont_status.0.is_null() {
                    *self2.status_font.borrow_mut() = Some(DeleteObjectGuard::new(w::HFONT::from_ptr(hfont_status.0)));
                }
            }

            // 4. Finally show and focus
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

            let btn_size = (32.0 * scale) as i32;
            let offset = (8.0 * scale) as i32;
            let btn_rect = w::RECT { left: w - btn_size - offset, top: offset, right: w - offset, bottom: btn_size + offset };

            if pt.x >= btn_rect.left && pt.x <= btn_rect.right && pt.y >= btn_rect.top && pt.y <= btn_rect.bottom {
                Ok(co::HT::CLIENT)
            } else {
                Ok(co::HT::CAPTION)
            }
        });

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
            let hdc = hwnd.BeginPaint()?;

            let rc = hwnd.GetClientRect()?;
            let width = rc.right - rc.left;
            let height = rc.bottom - rc.top;

            // Double buffering
            let hdc_mem = hdc.CreateCompatibleDC()?;
            let hbmp_mem = hdc.CreateCompatibleBitmap(width, height)?;
            let _hbmp_old = hdc_mem.SelectObject(&*hbmp_mem)?;

            unsafe {
                let mut graphics = std::ptr::null_mut();
                let _ = GdipCreateFromHDC(windows::Win32::Graphics::Gdi::HDC(hdc_mem.ptr() as _), &mut graphics);
                let _ = GdipGraphicsClear(graphics, 0x00000000); // 清除背景，解决圆角外部“脏边角”问题
                let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);

                // 1. Draw Background (Native Simple Rect)
                let mut bg_brush = std::ptr::null_mut();
                let _ = GdipCreateSolidFill(0xFF202020, &mut bg_brush);
                let _ = GdipFillRectangle(graphics, bg_brush as _, 0.0, 0.0, width as f32, height as f32);
                let _ = GdipDeleteBrush(bg_brush as _);

                let dpi = *self2.dpi_scale.borrow();

                // 2. Draw Logo
                if let Some((w_img, h_img, pixels)) = self2.logo_data.as_ref() {
                    let mut gdip_img = std::ptr::null_mut();
                    let _ = GdipCreateBitmapFromScan0(
                        *w_img,
                        *h_img,
                        (*w_img * 4) as i32,
                        PIXEL_FORMAT32BPP_ARGB,
                        Some(pixels.as_ptr()),
                        &mut gdip_img,
                    );

                    if !gdip_img.is_null() {
                        let logo_size = (80.0 * dpi) as i32;
                        let logo_x = (width - logo_size) / 2;
                        let logo_y = (40.0 * dpi) as i32;

                        let _ = GdipDrawImageRectRectI(
                            graphics,
                            gdip_img as _,
                            logo_x,
                            logo_y,
                            logo_size,
                            logo_size,
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

                // 3. Close Button
                let is_hover = *self2.is_close_hovered.borrow();
                let btn_size = (32.0 * dpi) as f32;
                let btn_x = width as f32 - btn_size - (8.0 * dpi);
                let btn_y = 8.0 * dpi;

                if is_hover {
                    let mut btn_brush = std::ptr::null_mut();
                    let _ = GdipCreateSolidFill(0x33FFFFFF, &mut btn_brush);
                    let mut path_btn = std::ptr::null_mut();
                    let _ = GdipCreatePath(FillModeAlternate, &mut path_btn);
                    add_round_rect(path_btn, btn_x, btn_y, btn_size, btn_size, 4.0 * dpi);
                    let _ = GdipFillPath(graphics, btn_brush as _, path_btn);
                    let _ = GdipDeletePath(path_btn);
                    let _ = GdipDeleteBrush(btn_brush as _);
                }

                let mut x_pen = std::ptr::null_mut();
                let _ = GdipCreatePen1(if is_hover { 0xFFFFFFFF } else { 0xFFA0A0A0 }, 1.5 * dpi, UnitPixel, &mut x_pen);
                let pad = btn_size * 0.35;
                let _ = GdipDrawLine(graphics, x_pen as _, btn_x + pad, btn_y + pad, btn_x + btn_size - pad, btn_y + btn_size - pad);
                let _ = GdipDrawLine(graphics, x_pen as _, btn_x + btn_size - pad, btn_y + pad, btn_x + pad, btn_y + btn_size - pad);
                let _ = GdipDeletePen(x_pen as _);

                // 4. Progress Bar (Rounded)
                let pb_w = (240.0 * dpi) as f32;
                let pb_h = (6.0 * dpi) as f32;
                let pb_x = (width as f32 - pb_w) / 2.0;
                let pb_y = height as f32 - (50.0 * dpi);
                let radius_pb = pb_h / 2.0;

                let mut pb_bg_brush = std::ptr::null_mut();
                let _ = GdipCreateSolidFill(0xFF404040, &mut pb_bg_brush);
                let mut path_pb_bg = std::ptr::null_mut();
                let _ = GdipCreatePath(FillModeAlternate, &mut path_pb_bg);
                add_round_rect(path_pb_bg, pb_x, pb_y, pb_w, pb_h, radius_pb);
                let _ = GdipFillPath(graphics, pb_bg_brush as _, path_pb_bg);
                let _ = GdipDeletePath(path_pb_bg);
                let _ = GdipDeleteBrush(pb_bg_brush as _);

                let visual_p = *self2.visual_progress.borrow();
                if visual_p > 0.0 {
                    let fill_w = (pb_w * (visual_p / 100.0)).max(radius_pb * 2.0);
                    let mut pb_fill_brush = std::ptr::null_mut();
                    let _ = GdipCreateSolidFill(0xFF0078D4, &mut pb_fill_brush);
                    let mut path_pb_fill = std::ptr::null_mut();
                    let _ = GdipCreatePath(FillModeAlternate, &mut path_pb_fill);
                    add_round_rect(path_pb_fill, pb_x, pb_y, fill_w, pb_h, radius_pb);
                    let _ = GdipFillPath(graphics, pb_fill_brush as _, path_pb_fill);
                    let _ = GdipDeletePath(path_pb_fill);
                    let _ = GdipDeleteBrush(pb_fill_brush as _);
                }

                let _ = GdipDeleteGraphics(graphics);

                // 4. Draw Text (GDI with System Font)
                let title_y = (140.0 * dpi) as i32;
                let title_h = (40.0 * dpi) as i32;
                if let Some(font) = self2.title_font.borrow().as_ref() {
                    let _old_font = hdc_mem.SelectObject(&**font)?;
                    let _ = hdc_mem.SetTextColor(w::COLORREF::new(255, 255, 255));
                    let _ = hdc_mem.SetBkMode(co::BKMODE::TRANSPARENT);
                    hdc_mem.DrawText(
                        &self2.title,
                        &w::RECT { left: 0, top: title_y, right: width, bottom: title_y + title_h },
                        co::DT::CENTER | co::DT::SINGLELINE | co::DT::VCENTER,
                    )?;
                }

                let status_y = (180.0 * dpi) as i32;
                let status_h = (60.0 * dpi) as i32;
                if let Some(font) = self2.status_font.borrow().as_ref() {
                    let _old_font = hdc_mem.SelectObject(&**font)?;
                    let _ = hdc_mem.SetTextColor(w::COLORREF::new(180, 180, 180));
                    let status = self2.status_text.borrow();
                    hdc_mem.DrawText(
                        &status,
                        &w::RECT { left: 20, top: status_y, right: width - 20, bottom: status_y + status_h },
                        co::DT::CENTER | co::DT::WORDBREAK,
                    )?;
                }
            }

            hdc.BitBlt(w::POINT::new(0, 0), w::SIZE::new(width, height), &hdc_mem, w::POINT::new(0, 0), co::ROP::SRCCOPY)?;
            Ok(())
        });

        // Handle Close Button Click
        let self2 = self.clone();
        self.wnd.on().wm_l_button_up(move |p| {
            let scale = *self2.dpi_scale.borrow();

            let hwnd = self2.wnd.hwnd();
            let rect = hwnd.GetClientRect()?;
            let w = rect.right - rect.left;

            let btn_size = (32.0 * scale) as i32;
            let offset = (8.0 * scale) as i32;
            let btn_rect = w::RECT { left: w - btn_size - offset, top: offset, right: w - offset, bottom: btn_size + offset };

            let pt = p.coords;
            if pt.x >= btn_rect.left && pt.x <= btn_rect.right && pt.y >= btn_rect.top && pt.y <= btn_rect.bottom {
                self2.wnd.hwnd().SendMessage(w::msg::wm::Close {});
            }
            Ok(())
        });

        let self2 = self.clone();
        self.wnd.on().wm_mouse_move(move |p| {
            let scale = *self2.dpi_scale.borrow();
            let rect = self2.wnd.hwnd().GetClientRect()?;
            let w = rect.right - rect.left;

            let btn_size = (32.0 * scale) as i32;
            let offset = (8.0 * scale) as i32;
            let btn_rect = w::RECT { left: w - btn_size - offset, top: offset, right: w - offset, bottom: btn_size + offset };

            let pt = p.coords;
            let is_in = pt.x >= btn_rect.left && pt.x <= btn_rect.right && pt.y >= btn_rect.top && pt.y <= btn_rect.bottom;

            let mut hovered = self2.is_close_hovered.borrow_mut();
            if *hovered != is_in {
                *hovered = is_in;
                // 为了保险，使整个右上角失效
                self2.wnd.hwnd().InvalidateRect(
                    Some(&w::RECT { left: btn_rect.left - 2, top: btn_rect.top - 2, right: w, bottom: btn_rect.bottom + 2 }),
                    false,
                )?;
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
                let width = rect.right - rect.left;

                let btn_size = (32.0 * scale) as i32;
                let offset = (8.0 * scale) as i32;
                let btn_rect = w::RECT { left: width - btn_size - offset, top: offset, right: width - offset, bottom: btn_size + offset };
                hwnd.InvalidateRect(Some(&btn_rect), false)?;
            }
            Ok(())
        });
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
