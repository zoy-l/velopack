use crate::shared::windows_ui;
use anyhow::{bail, Result};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};
use winsafe::{self as w, co, guard::DeleteObjectGuard, gui, prelude::*};

use windows::Win32::Foundation::HWND as WinHWND;
use windows::Win32::Graphics::GdiPlus::{
    FillModeAlternate, GdipCreateFromHDC, GdipCreatePath, GdipCreateSolidFill, GdipDeleteBrush, GdipDeleteGraphics, GdipDeletePath,
    GdipFillPath, GdipFillRectangle, GdipGraphicsClear, GdipSetSmoothingMode, GdiplusShutdown, GdiplusStartup, GdiplusStartupInput,
    GpGraphics as GdiplusGraphics, SmoothingModeAntiAlias,
};

const WINDOW_WIDTH: i32 = 300;
const WINDOW_HEIGHT: i32 = 340;
const LOGO_SOURCE_SIZE: i32 = 256;
const LOGO_DRAW_SIZE: f32 = 80.0;
const LOGO_TOP: f32 = 40.0;
const CLOSE_BTN_SIZE: f32 = 32.0;
const CLOSE_BTN_OFFSET: f32 = 16.0;
const PROGRESS_WIDTH: f32 = 240.0;
const PROGRESS_HEIGHT: f32 = 6.0;
const PROGRESS_BOTTOM_OFFSET: f32 = 50.0;
const TITLE_Y: f32 = 140.0;
const TITLE_H: f32 = 40.0;
const STATUS_Y: f32 = 180.0;
const STATUS_H: f32 = 60.0;
const STATUS_X_MARGIN: i32 = 20;

pub use windows_ui::{set_theme_override, set_theme_override_from_str, ThemeMode};

struct ThemeColors {
    background: u32,
    title_text: w::COLORREF,
    status_text: w::COLORREF,
    close_hover_bg: u32,
    close_icon: u32,
    close_icon_hover: u32,
    progress_bg: u32,
    progress_fill: u32,
}

fn theme_colors(theme: ThemeMode) -> ThemeColors {
    match theme {
        ThemeMode::Dark => ThemeColors {
            background: 0xFF202020,
            title_text: w::COLORREF::new(255, 255, 255),
            status_text: w::COLORREF::new(180, 180, 180),
            close_hover_bg: 0x33FFFFFF,
            close_icon: 0xFFA0A0A0,
            close_icon_hover: 0xFFFFFFFF,
            progress_bg: 0xFF404040,
            progress_fill: 0xFF0078D4,
        },
        ThemeMode::Light => ThemeColors {
            background: 0xFFF6F6F6,
            title_text: w::COLORREF::new(30, 30, 30),
            status_text: w::COLORREF::new(90, 90, 90),
            close_hover_bg: 0x14000000,
            close_icon: 0xFF666666,
            close_icon_hover: 0xFF000000,
            progress_bg: 0xFFDDDDDD,
            progress_fill: 0xFF0078D4,
        },
    }
}

// GDI imports
use windows::Win32::Graphics::Gdi::{CreateFontIndirectW, CLEARTYPE_QUALITY};
use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS};

const TMR_PROGRESS: usize = 1;
const MSG_NOMESSAGE: i16 = -99;
const IMAGE_DATA: &[u8] = include_bytes!("../assets/product_logo.png");

pub const MSG_CLOSE: i16 = -1;
pub const MSG_INDEFINITE: i16 = -2;

pub fn show_progress_dialog<T: AsRef<str>>(window_title: T) -> Sender<i16> {
    let window_title = window_title.as_ref().to_string();
    let content = "更新中...".to_string();
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
    let content = "安装中...".to_string();
    let (tx, rx) = mpsc::channel::<i16>();
    thread::spawn(move || {
        let _ = SplashWindow::new(app_name, content, rx).and_then(|w| {
            w.run()?;
            Ok(())
        });
    });
    tx
}

#[derive(Clone, Copy)]
struct SplashLayout {
    logo_x: i32,
    logo_y: i32,
    logo_size: i32,
    close_rect: w::RECT,
    progress_x: f32,
    progress_y: f32,
    progress_w: f32,
    progress_h: f32,
    title_rect: w::RECT,
    status_rect: w::RECT,
}

fn close_button_rect(width: i32, scale: f32) -> w::RECT {
    let btn_size = (CLOSE_BTN_SIZE * scale) as i32;
    let offset = (CLOSE_BTN_OFFSET * scale) as i32;
    w::RECT { left: width - btn_size - offset, top: offset, right: width - offset, bottom: btn_size + offset }
}

fn compute_layout(width: i32, height: i32, scale: f32) -> SplashLayout {
    let logo_size = (LOGO_DRAW_SIZE * scale) as i32;
    let logo_x = (width - logo_size) / 2;
    let logo_y = (LOGO_TOP * scale) as i32;
    let close_rect = close_button_rect(width, scale);
    let progress_w = PROGRESS_WIDTH * scale;
    let progress_h = PROGRESS_HEIGHT * scale;
    let progress_x = (width as f32 - progress_w) / 2.0;
    let progress_y = height as f32 - (PROGRESS_BOTTOM_OFFSET * scale);
    let title_y = (TITLE_Y * scale) as i32;
    let title_h = (TITLE_H * scale) as i32;
    let status_y = (STATUS_Y * scale) as i32;
    let status_h = (STATUS_H * scale) as i32;

    SplashLayout {
        logo_x,
        logo_y,
        logo_size,
        close_rect,
        progress_x,
        progress_y,
        progress_w,
        progress_h,
        title_rect: w::RECT { left: 0, top: title_y, right: width, bottom: title_y + title_h },
        status_rect: w::RECT { left: STATUS_X_MARGIN, top: status_y, right: width - STATUS_X_MARGIN, bottom: status_y + status_h },
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
    theme: Rc<RefCell<ThemeMode>>,
    is_close_hovered: Rc<RefCell<bool>>,
    should_close: Rc<RefCell<bool>>,
    close_delay_ticks: Rc<RefCell<i32>>,
}

impl SplashWindow {
    pub fn new(title: String, status_text: String, rx: Receiver<i16>) -> Result<Self> {
        let wnd = gui::WindowMain::new(gui::WindowMainOpts {
            class_icon: gui::Icon::Id(1),
            class_style: co::CS::HREDRAW | co::CS::VREDRAW | co::CS::DROPSHADOW,
            class_name: "VelopackModernSplashWindow".to_owned(),
            title: title.clone(),
            size: (WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32),
            // Show on taskbar
            ex_style: co::WS_EX::APPWINDOW | co::WS_EX::TOPMOST,
            style: co::WS::POPUP,
            ..Default::default()
        });

        let logo_data = windows_ui::load_logo_data(LOGO_SOURCE_SIZE, IMAGE_DATA);

        let rx = Rc::new(rx);
        let target_progress = Rc::new(RefCell::new(0));
        let visual_progress = Rc::new(RefCell::new(0.0));
        let status_text = Rc::new(RefCell::new(status_text));
        let logo_data = Rc::new(logo_data);
        let title_font = Rc::new(RefCell::new(None));
        let status_font = Rc::new(RefCell::new(None));
        let dpi_scale = Rc::new(RefCell::new(1.0));
        let theme = Rc::new(RefCell::new(windows_ui::current_theme()));
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
            theme,
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

            // 1. DWM Dark Mode and Corners
            let theme = windows_ui::current_theme();
            *self2.theme.borrow_mut() = theme;
            windows_ui::apply_window_style(win_hwnd, theme);

            // 2. Position and Size (Use NOREDRAW/HIDEWINDOW if needed, but here we just want it ready)
            let scale = windows_ui::get_dpi_scale(win_hwnd);
            *self2.dpi_scale.borrow_mut() = scale;

            let w_val = (WINDOW_WIDTH as f32 * scale) as i32;
            let h_val = (WINDOW_HEIGHT as f32 * scale) as i32;

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
            init_fonts(&self2, scale);

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
            let rect = self2.wnd.hwnd().GetClientRect()?;
            let w = rect.right - rect.left;
            let btn_rect = close_button_rect(w, scale);

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
            let dpi = *self2.dpi_scale.borrow();
            let layout = compute_layout(width, height, dpi);

            // Double buffering
            let hdc_mem = hdc.CreateCompatibleDC()?;
            let hbmp_mem = hdc.CreateCompatibleBitmap(width, height)?;
            let _hbmp_old = hdc_mem.SelectObject(&*hbmp_mem)?;

            let colors = theme_colors(*self2.theme.borrow());
            unsafe {
                let mut graphics = std::ptr::null_mut();
                let _ = GdipCreateFromHDC(windows::Win32::Graphics::Gdi::HDC(hdc_mem.ptr() as _), &mut graphics);
                let _ = GdipGraphicsClear(graphics, 0x00000000); // 清除背景，解决圆角外部“脏边角”问题
                let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);

                draw_background(graphics, width, height, &colors);
                if let Some(logo_data) = self2.logo_data.as_ref() {
                    let logo_rect = w::RECT {
                        left: layout.logo_x,
                        top: layout.logo_y,
                        right: layout.logo_x + layout.logo_size,
                        bottom: layout.logo_y + layout.logo_size,
                    };
                    windows_ui::draw_logo(graphics, logo_rect, logo_data, windows_ui::PIXEL_FORMAT32BPP_ARGB);
                }
                windows_ui::draw_close_button(
                    graphics,
                    layout.close_rect,
                    *self2.dpi_scale.borrow(),
                    *self2.is_close_hovered.borrow(),
                    colors.close_hover_bg,
                    colors.close_icon,
                    colors.close_icon_hover,
                    4.0,
                    0.35,
                );
                draw_progress_bar(graphics, &self2, layout, &colors);
                let _ = GdipDeleteGraphics(graphics);
            }

            draw_text(&self2, &hdc_mem, layout, &colors)?;

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
            let btn_rect = close_button_rect(w, scale);

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
            let btn_rect = close_button_rect(w, scale);

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
                let btn_rect = close_button_rect(width, scale);
                hwnd.InvalidateRect(Some(&btn_rect), false)?;
            }
            Ok(())
        });
    }
}

fn init_fonts(window: &SplashWindow, scale: f32) {
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
        lf_title.lfQuality = CLEARTYPE_QUALITY;
        let hfont_title = CreateFontIndirectW(&lf_title);
        if !hfont_title.0.is_null() {
            *window.title_font.borrow_mut() = Some(DeleteObjectGuard::new(w::HFONT::from_ptr(hfont_title.0)));
        }

        let mut lf_status = ncm.lfMessageFont;
        lf_status.lfHeight = -((12.0 * 96.0 / 72.0) * scale) as i32;
        lf_status.lfQuality = CLEARTYPE_QUALITY;
        let hfont_status = CreateFontIndirectW(&lf_status);
        if !hfont_status.0.is_null() {
            *window.status_font.borrow_mut() = Some(DeleteObjectGuard::new(w::HFONT::from_ptr(hfont_status.0)));
        }
    }
}

unsafe fn draw_background(graphics: *mut GdiplusGraphics, width: i32, height: i32, colors: &ThemeColors) {
    let mut bg_brush = std::ptr::null_mut();
    let _ = GdipCreateSolidFill(colors.background, &mut bg_brush);
    let _ = GdipFillRectangle(graphics, bg_brush as _, 0.0, 0.0, width as f32, height as f32);
    let _ = GdipDeleteBrush(bg_brush as _);
}

unsafe fn draw_progress_bar(graphics: *mut GdiplusGraphics, window: &SplashWindow, layout: SplashLayout, colors: &ThemeColors) {
    let radius_pb = layout.progress_h / 2.0;

    let mut pb_bg_brush = std::ptr::null_mut();
    let _ = GdipCreateSolidFill(colors.progress_bg, &mut pb_bg_brush);
    let mut path_pb_bg = std::ptr::null_mut();
    let _ = GdipCreatePath(FillModeAlternate, &mut path_pb_bg);
    windows_ui::add_round_rect(path_pb_bg, layout.progress_x, layout.progress_y, layout.progress_w, layout.progress_h, radius_pb);
    let _ = GdipFillPath(graphics, pb_bg_brush as _, path_pb_bg);
    let _ = GdipDeletePath(path_pb_bg);
    let _ = GdipDeleteBrush(pb_bg_brush as _);

    let visual_p = *window.visual_progress.borrow();
    if visual_p > 0.0 {
        let fill_w = (layout.progress_w * (visual_p / 100.0)).max(radius_pb * 2.0);
        let mut pb_fill_brush = std::ptr::null_mut();
        let _ = GdipCreateSolidFill(colors.progress_fill, &mut pb_fill_brush);
        let mut path_pb_fill = std::ptr::null_mut();
        let _ = GdipCreatePath(FillModeAlternate, &mut path_pb_fill);
        windows_ui::add_round_rect(path_pb_fill, layout.progress_x, layout.progress_y, fill_w, layout.progress_h, radius_pb);
        let _ = GdipFillPath(graphics, pb_fill_brush as _, path_pb_fill);
        let _ = GdipDeletePath(path_pb_fill);
        let _ = GdipDeleteBrush(pb_fill_brush as _);
    }
}

fn draw_text(window: &SplashWindow, hdc: &w::HDC, layout: SplashLayout, colors: &ThemeColors) -> Result<()> {
    if let Some(font) = window.title_font.borrow().as_ref() {
        windows_ui::draw_text(
            hdc,
            font,
            colors.title_text,
            &layout.title_rect,
            co::DT::CENTER | co::DT::SINGLELINE | co::DT::VCENTER,
            &window.title,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
    }

    if let Some(font) = window.status_font.borrow().as_ref() {
        let status = window.status_text.borrow();
        windows_ui::draw_text(
            hdc,
            font,
            colors.status_text,
            &layout.status_rect,
            co::DT::CENTER | co::DT::WORDBREAK,
            &status,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
    }
    Ok(())
}
