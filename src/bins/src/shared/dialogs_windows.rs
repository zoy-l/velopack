use super::{dialogs_common::*, dialogs_const::*, windows_ui};
use anyhow::Result;
use std::{cell::RefCell, path::PathBuf, rc::Rc};
use velopack::bundle::Manifest;
use velopack::locator::{auto_locate_app_manifest, LocationContext};
use windows::Win32::Foundation::HWND as WinHWND;
use windows::Win32::Graphics::Gdi::{CreateFontIndirectW, CLEARTYPE_QUALITY};
use windows::Win32::Graphics::GdiPlus::{
    FillModeAlternate, GdipCreateFromHDC, GdipCreatePath, GdipCreateSolidFill, GdipDeleteBrush, GdipDeleteGraphics, GdipDeletePath,
    GdipFillPath, GdipFillRectangle, GdipGraphicsClear, GdipSetSmoothingMode, GdiplusShutdown, GdiplusStartup, GdiplusStartupInput,
    SmoothingModeAntiAlias,
};
use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS};
use winsafe::{self as w, co, guard::DeleteObjectGuard, gui, prelude::*, WString};

pub use windows_ui::{set_theme_override, set_theme_override_from_str, ThemeMode};

const IMAGE_DATA: &[u8] = include_bytes!("../assets/product_logo.png");

struct ThemeColors {
    background: u32,
    title_text: w::COLORREF,
    header_text: w::COLORREF,
    body_text: w::COLORREF,
    footer_text: w::COLORREF,
    footer_link: w::COLORREF,
    close_hover_bg: u32,
    close_icon: u32,
    close_icon_hover: u32,
    button_primary: u32,
    button_primary_hover: u32,
    button_secondary: u32,
    button_secondary_hover: u32,
}

fn theme_colors(theme: ThemeMode) -> ThemeColors {
    match theme {
        ThemeMode::Dark => ThemeColors {
            background: 0xFF202020,
            title_text: w::COLORREF::new(200, 200, 200),
            header_text: w::COLORREF::new(255, 255, 255),
            body_text: w::COLORREF::new(180, 180, 180),
            footer_text: w::COLORREF::new(160, 160, 160),
            footer_link: w::COLORREF::new(0, 120, 212),
            close_hover_bg: 0x33FFFFFF,
            close_icon: 0xFFA0A0A0,
            close_icon_hover: 0xFFFFFFFF,
            button_primary: 0xFF0078D4,
            button_primary_hover: 0xFF1E88E5,
            button_secondary: 0xFF303030,
            button_secondary_hover: 0xFF3A3A3A,
        },
        ThemeMode::Light => ThemeColors {
            background: 0xFFF6F6F6,
            title_text: w::COLORREF::new(80, 80, 80),
            header_text: w::COLORREF::new(20, 20, 20),
            body_text: w::COLORREF::new(70, 70, 70),
            footer_text: w::COLORREF::new(120, 120, 120),
            footer_link: w::COLORREF::new(0, 120, 212),
            close_hover_bg: 0x14000000,
            close_icon: 0xFF666666,
            close_icon_hover: 0xFF000000,
            button_primary: 0xFF0078D4,
            button_primary_hover: 0xFF1E88E5,
            button_secondary: 0xFFE6E6E6,
            button_secondary_hover: 0xFFDCDCDC,
        },
    }
}

fn button_text_color(theme: ThemeMode, is_primary: bool, is_sub: bool) -> w::COLORREF {
    match theme {
        ThemeMode::Dark => {
            if is_sub {
                w::COLORREF::new(200, 200, 200)
            } else {
                w::COLORREF::new(255, 255, 255)
            }
        }
        ThemeMode::Light => {
            if is_primary {
                w::COLORREF::new(255, 255, 255)
            } else if is_sub {
                w::COLORREF::new(110, 110, 110)
            } else {
                w::COLORREF::new(30, 30, 30)
            }
        }
    }
}

struct DialogButtonSpec {
    result: DialogResult,
    label: String,
    is_primary: bool,
}

impl DialogButtonSpec {
    fn primary(result: DialogResult, label: impl Into<String>) -> Self {
        Self { result, label: label.into(), is_primary: true }
    }

    fn secondary(result: DialogResult, label: impl Into<String>) -> Self {
        Self { result, label: label.into(), is_primary: false }
    }
}

struct DialogOptions {
    title: String,
    header: Option<String>,
    body: String,
    buttons: Vec<DialogButtonSpec>,
    footer_text: Option<String>,
    footer_link: Option<String>,
}

fn show_modern_dialog(opts: DialogOptions) -> DialogResult {
    let window = ModernDialogWindow::new(opts);
    window.run()
}

pub fn show_restart_required(app: &Manifest) {
    let _ = show_modern_dialog(DialogOptions {
        title: format!("{} 安装程序 {}", app.title, app.version),
        header: Some("需要重启".to_string()),
        body: "安装继续之前需要重启计算机。请重启后再试。".to_string(),
        buttons: vec![DialogButtonSpec::primary(DialogResult::Ok, "确定")],
        footer_text: None,
        footer_link: None,
    });
}

pub fn show_update_missing_dependencies_dialog(
    app: &Manifest,
    depedency_string: &str,
    from: &semver::Version,
    to: &semver::Version,
) -> bool {
    if get_silent() {
        // this has different behavior to show_setup_missing_dependencies_dialog,
        // if silent is true then we will bail because the app is probably exiting
        // and installing dependencies may result in a UAC prompt.
        warn!("Cancelling pre-requisite installation because silent flag is true.");
        return false;
    }

    let result = show_modern_dialog(DialogOptions {
        title: format!("{} 更新", app.title),
        header: Some(format!("{} 想从 {} 更新到 {}", app.title, from, to)),
        body: format!("{} {to} 缺少依赖组件，需要安装：{}。是否继续？", app.title, depedency_string),
        buttons: vec![DialogButtonSpec::primary(DialogResult::Ok, "安装并更新"), DialogButtonSpec::secondary(DialogResult::Cancel, "取消")],
        footer_text: None,
        footer_link: None,
    });
    result == DialogResult::Ok
}

pub fn show_setup_missing_dependencies_dialog(app: &Manifest, depedency_string: &str) -> bool {
    if get_silent() {
        return true;
    }

    let result = show_modern_dialog(DialogOptions {
        title: format!("{} 安装程序 {}", app.title, app.version),
        header: Some(format!("{} 缺少系统依赖。", app.title)),
        body: format!("{} 需要安装以下组件：{}。是否继续？", app.title, depedency_string),
        buttons: vec![DialogButtonSpec::primary(DialogResult::Ok, "安装"), DialogButtonSpec::secondary(DialogResult::Cancel, "取消")],
        footer_text: None,
        footer_link: None,
    });
    result == DialogResult::Ok
}

pub fn show_uninstall_complete_with_errors_dialog(app_title: &str, log_path: Option<&PathBuf>) {
    if get_silent() {
        return;
    }

    let mut footer_text = None;
    let mut footer_link = None;
    if let Some(log_path) = log_path {
        if log_path.exists() {
            footer_text = Some(format!("日志文件：{}", log_path.display()));
            footer_link = Some(log_path.display().to_string());
        }
    }

    let _ = show_modern_dialog(DialogOptions {
        title: format!("{} 卸载", app_title),
        header: Some(format!("{} 卸载已完成，但出现错误。", app_title)),
        body: "系统中可能残留文件或目录。你可以手动删除，或重新安装后再试。".to_string(),
        buttons: vec![DialogButtonSpec::primary(DialogResult::Ok, "确定")],
        footer_text,
        footer_link,
    });
}

pub fn show_processes_locking_folder_dialog(app_title: &str, app_version: &str, process_names: &str) -> DialogResult {
    if get_silent() {
        return DialogResult::Cancel;
    }

    show_modern_dialog(DialogOptions {
        title: format!("{} 更新 {}", app_title, app_version),
        header: Some(format!("{} 更新", app_title)),
        body: format!(
            "有程序（{}）正在阻止 {} 更新继续。\n\n\
你可以点击“继续”让更新器尝试自动关闭这些程序，或者在你手动关闭后点击“重试”再次检查。",
            process_names, app_title
        ),
        buttons: vec![
            DialogButtonSpec::primary(DialogResult::Retry, "重试\n已关闭后重试"),
            DialogButtonSpec::secondary(DialogResult::Continue, "继续\n尝试自动关闭程序"),
            DialogButtonSpec::secondary(DialogResult::Cancel, "取消\n更新不会继续"),
        ],
        footer_text: None,
        footer_link: None,
    })
}

pub fn show_overwrite_repair_dialog(app: &Manifest, root_path: &PathBuf) -> bool {
    if get_silent() {
        return true;
    }

    // these are the defaults, if we can't detect the current app version - we call it "Repair"
    let setup_name = format!("{} 安装程序 {}", app.title, app.version);
    let mut instruction = format!("{} 已经安装。", app.title);
    let mut content = "该应用已安装在电脑上。如果运行异常，你可以尝试修复。".to_string();
    let mut btn_yes_txt = format!("修复\n删除应用并重新安装版本 {}", app.version);
    let btn_cancel_txt = "取消\n请先备份或保存你的工作".to_string();

    // if we can detect the current app version, we call it "Update" or "Downgrade"
    let old_app = auto_locate_app_manifest(LocationContext::FromSpecifiedRootDir(root_path.to_owned()));
    if let Ok(old) = old_app {
        let old_version = old.get_manifest_version();
        if old_version < app.version {
            instruction = format!("已安装旧版本 {}", app.title);
            content = format!("是否从 {} 更新到 {}？", old_version, app.version);
            btn_yes_txt = format!("更新\n更新到版本 {}", app.version);
        } else if old_version > app.version {
            instruction = format!("已安装更新版本 {}", app.title);
            content = format!("当前已安装 {}。是否将应用降级到旧版本？", old_version);
            btn_yes_txt = format!("降级\n降级到版本 {}", app.version);
        }
    }

    let result = show_modern_dialog(DialogOptions {
        title: setup_name,
        header: Some(instruction),
        body: content,
        buttons: vec![
            DialogButtonSpec::primary(DialogResult::Yes, &btn_yes_txt),
            DialogButtonSpec::secondary(DialogResult::Cancel, &btn_cancel_txt),
        ],
        footer_text: None,
        footer_link: None,
    });
    result == DialogResult::Yes
}

struct DialogLayout {
    close_rect: w::RECT,
    logo_rect: Option<w::RECT>,
    title_rect: w::RECT,
    header_rect: Option<w::RECT>,
    body_rect: w::RECT,
    button_rects: Vec<w::RECT>,
    footer_rect: Option<w::RECT>,
    has_command_links: bool,
}

struct ModernDialogWindow {
    wnd: gui::WindowMain,
    options: DialogOptions,
    result: Rc<RefCell<Option<DialogResult>>>,
    dpi_scale: Rc<RefCell<f32>>,
    theme: Rc<RefCell<ThemeMode>>,
    logo_data: Rc<Option<(i32, i32, Vec<u8>)>>,
    title_font: Rc<RefCell<Option<DeleteObjectGuard<w::HFONT>>>>,
    header_font: Rc<RefCell<Option<DeleteObjectGuard<w::HFONT>>>>,
    body_font: Rc<RefCell<Option<DeleteObjectGuard<w::HFONT>>>>,
    button_font: Rc<RefCell<Option<DeleteObjectGuard<w::HFONT>>>>,
    button_sub_font: Rc<RefCell<Option<DeleteObjectGuard<w::HFONT>>>>,
    footer_font: Rc<RefCell<Option<DeleteObjectGuard<w::HFONT>>>>,
    is_close_hovered: Rc<RefCell<bool>>,
    hovered_button: Rc<RefCell<Option<usize>>>,
}

impl ModernDialogWindow {
    pub fn new(options: DialogOptions) -> Self {
        let (base_w, base_h) = calc_window_size(&options, 1.0);
        let wnd = gui::WindowMain::new(gui::WindowMainOpts {
            class_icon: gui::Icon::Id(1),
            class_style: co::CS::HREDRAW | co::CS::VREDRAW | co::CS::DROPSHADOW,
            class_name: "VelopackModernDialogWindow".to_owned(),
            title: options.title.clone(),
            size: (base_w as u32, base_h as u32),
            // Show on taskbar
            ex_style: co::WS_EX::APPWINDOW,
            style: co::WS::POPUP,
            ..Default::default()
        });

        let logo_data = Rc::new(windows_ui::load_logo_data(96, IMAGE_DATA));
        let mut new_self = Self {
            wnd,
            options,
            result: Rc::new(RefCell::new(None)),
            dpi_scale: Rc::new(RefCell::new(1.0)),
            theme: Rc::new(RefCell::new(windows_ui::current_theme())),
            logo_data,
            title_font: Rc::new(RefCell::new(None)),
            header_font: Rc::new(RefCell::new(None)),
            body_font: Rc::new(RefCell::new(None)),
            button_font: Rc::new(RefCell::new(None)),
            button_sub_font: Rc::new(RefCell::new(None)),
            footer_font: Rc::new(RefCell::new(None)),
            is_close_hovered: Rc::new(RefCell::new(false)),
            hovered_button: Rc::new(RefCell::new(None)),
        };

        new_self.events();
        new_self
    }

    pub fn run(&self) -> DialogResult {
        let mut token: usize = 0;
        let input = GdiplusStartupInput { GdiplusVersion: 1, ..Default::default() };
        unsafe {
            let _ = GdiplusStartup(&mut token, &input, std::ptr::null_mut());
        }

        let _ = self.wnd.run_main(None);

        unsafe {
            GdiplusShutdown(token);
        }

        let result = self.result.borrow().clone();
        result.unwrap_or_else(|| default_dialog_result(&self.options))
    }

    fn events(&mut self) {
        let self2 = self.clone();
        self.wnd.on().wm_create(move |_m| {
            let raw_hwnd = self2.wnd.hwnd().ptr();
            let win_hwnd = WinHWND(raw_hwnd);

            let theme = windows_ui::current_theme();
            *self2.theme.borrow_mut() = theme;
            let hwnd = unsafe { self2.wnd.hwnd().raw_copy() };
            let raw_hwnd = hwnd.ptr();
            if !raw_hwnd.is_null() {
                windows_ui::apply_window_style(WinHWND(raw_hwnd), theme);
            }

            let scale = windows_ui::get_dpi_scale(win_hwnd);
            *self2.dpi_scale.borrow_mut() = scale;
            let (w_val, h_val) = calc_window_size(&self2.options, scale);

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

            init_fonts(&self2, scale);

            self2.wnd.hwnd().ShowWindow(co::SW::SHOW);
            self2.wnd.hwnd().SetForegroundWindow();
            Ok(0)
        });

        let self2 = self.clone();
        self.wnd.on().wm_nc_hit_test(move |p| {
            let mut pt = p.cursor_pos;
            self2.wnd.hwnd().ScreenToClient(&mut pt)?;

            let hwnd = self2.wnd.hwnd();
            let rect = hwnd.GetClientRect()?;
            let layout = build_layout(&self2, rect.right - rect.left, rect.bottom - rect.top);

            if rect_contains(&layout.close_rect, pt) || layout.button_rects.iter().any(|r| rect_contains(r, pt)) {
                Ok(co::HT::CLIENT)
            } else if let Some(footer_rect) = layout.footer_rect {
                if rect_contains(&footer_rect, pt) && self2.options.footer_link.is_some() {
                    Ok(co::HT::CLIENT)
                } else {
                    Ok(co::HT::CAPTION)
                }
            } else {
                Ok(co::HT::CAPTION)
            }
        });

        let self2 = self.clone();
        self.wnd.on().wm_erase_bkgnd(move |_| Ok(1));

        self.wnd.on().wm_paint(move || {
            let hwnd = self2.wnd.hwnd();
            let hdc = hwnd.BeginPaint()?;

            let rc = hwnd.GetClientRect()?;
            let width = rc.right - rc.left;
            let height = rc.bottom - rc.top;

            let hdc_mem = hdc.CreateCompatibleDC()?;
            let hbmp_mem = hdc.CreateCompatibleBitmap(width, height)?;
            let _hbmp_old = hdc_mem.SelectObject(&*hbmp_mem)?;

            let layout = build_layout(&self2, width, height);
            let dpi = *self2.dpi_scale.borrow();
            let colors = theme_colors(*self2.theme.borrow());

            unsafe {
                let mut graphics = std::ptr::null_mut();
                let _ = GdipCreateFromHDC(windows::Win32::Graphics::Gdi::HDC(hdc_mem.ptr() as _), &mut graphics);
                let _ = GdipGraphicsClear(graphics, 0x00000000);
                let _ = GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);

                // Background fill
                let mut bg_rect_brush = std::ptr::null_mut();
                let _ = GdipCreateSolidFill(colors.background, &mut bg_rect_brush);
                let _ = GdipFillRectangle(graphics, bg_rect_brush as _, 0.0, 0.0, width as f32, height as f32);
                let _ = GdipDeleteBrush(bg_rect_brush as _);

                // Logo
                if let (Some(logo_rect), Some(logo_data)) = (layout.logo_rect, self2.logo_data.as_ref()) {
                    windows_ui::draw_logo(graphics, logo_rect, logo_data, windows_ui::PIXEL_FORMAT32BPP_ARGB);
                }

                // Close button
                windows_ui::draw_close_button(
                    graphics,
                    layout.close_rect,
                    dpi,
                    *self2.is_close_hovered.borrow(),
                    colors.close_hover_bg,
                    colors.close_icon,
                    colors.close_icon_hover,
                    4.0,
                    0.32,
                );

                // Buttons
                for (idx, rect) in layout.button_rects.iter().enumerate() {
                    let is_hover = self2.hovered_button.borrow().map(|i| i == idx).unwrap_or(false);
                    let is_primary = self2.options.buttons[idx].is_primary;
                    let base_color = if is_primary { colors.button_primary } else { colors.button_secondary };
                    let hover_color = if is_primary { colors.button_primary_hover } else { colors.button_secondary_hover };
                    let color = if is_hover { hover_color } else { base_color };

                    let mut btn_brush = std::ptr::null_mut();
                    let _ = GdipCreateSolidFill(color, &mut btn_brush);
                    let mut path_btn = std::ptr::null_mut();
                    let _ = GdipCreatePath(FillModeAlternate, &mut path_btn);
                    windows_ui::add_round_rect(
                        path_btn,
                        rect.left as f32,
                        rect.top as f32,
                        (rect.right - rect.left) as f32,
                        (rect.bottom - rect.top) as f32,
                        6.0 * dpi,
                    );
                    let _ = GdipFillPath(graphics, btn_brush as _, path_btn);
                    let _ = GdipDeletePath(path_btn);
                    let _ = GdipDeleteBrush(btn_brush as _);
                }

                let _ = GdipDeleteGraphics(graphics);
            }

            draw_dialog_text(&self2, &hdc_mem, &layout);

            hdc.BitBlt(w::POINT::new(0, 0), w::SIZE::new(width, height), &hdc_mem, w::POINT::new(0, 0), co::ROP::SRCCOPY)?;
            Ok(())
        });

        let self2 = self.clone();
        self.wnd.on().wm_l_button_up(move |p| {
            let hwnd = self2.wnd.hwnd();
            let rect = hwnd.GetClientRect()?;
            let layout = build_layout(&self2, rect.right - rect.left, rect.bottom - rect.top);

            let pt = p.coords;
            if rect_contains(&layout.close_rect, pt) {
                *self2.result.borrow_mut() = Some(default_dialog_result(&self2.options));
                self2.wnd.hwnd().SendMessage(w::msg::wm::Close {});
                return Ok(());
            }

            for (idx, rect) in layout.button_rects.iter().enumerate() {
                if rect_contains(rect, pt) {
                    let result = self2.options.buttons[idx].result;
                    *self2.result.borrow_mut() = Some(result);
                    self2.wnd.hwnd().SendMessage(w::msg::wm::Close {});
                    return Ok(());
                }
            }

            if let Some(footer_rect) = layout.footer_rect {
                if rect_contains(&footer_rect, pt) {
                    if let Some(link) = self2.options.footer_link.as_ref() {
                        w::HWND::GetDesktopWindow().ShellExecute("open", &link, None, None, co::SW::SHOWDEFAULT).ok();
                    }
                }
            }
            Ok(())
        });

        let self2 = self.clone();
        self.wnd.on().wm_mouse_move(move |p| {
            let rect = self2.wnd.hwnd().GetClientRect()?;
            let layout = build_layout(&self2, rect.right - rect.left, rect.bottom - rect.top);

            let pt = p.coords;
            let mut hovered_btn = None;
            for (idx, rect) in layout.button_rects.iter().enumerate() {
                if rect_contains(rect, pt) {
                    hovered_btn = Some(idx);
                    break;
                }
            }

            let mut hovered_btn_state = self2.hovered_button.borrow_mut();
            if *hovered_btn_state != hovered_btn {
                *hovered_btn_state = hovered_btn;
                self2.wnd.hwnd().InvalidateRect(None, false)?;
            }

            let is_in_close = rect_contains(&layout.close_rect, pt);
            let mut hovered_close = self2.is_close_hovered.borrow_mut();
            if *hovered_close != is_in_close {
                *hovered_close = is_in_close;
                self2.wnd.hwnd().InvalidateRect(Some(&layout.close_rect), false)?;
            }

            let mut tme = w::TRACKMOUSEEVENT::default();
            tme.dwFlags = co::TME::LEAVE;
            tme.hwndTrack = unsafe { w::HWND::from_ptr(self2.wnd.hwnd().ptr()) };
            let _ = w::TrackMouseEvent(&mut tme);

            Ok(())
        });

        let self2 = self.clone();
        self.wnd.on().wm_mouse_leave(move || {
            let mut hovered_btn = self2.hovered_button.borrow_mut();
            let mut hovered_close = self2.is_close_hovered.borrow_mut();
            if hovered_btn.is_some() || *hovered_close {
                *hovered_btn = None;
                *hovered_close = false;
                self2.wnd.hwnd().InvalidateRect(None, false)?;
            }
            Ok(())
        });

        let self2 = self.clone();
        self.wnd.on().wm_close(move || {
            if self2.result.borrow().is_none() {
                *self2.result.borrow_mut() = Some(default_dialog_result(&self2.options));
            }
            Ok(())
        });
    }
}

impl Clone for ModernDialogWindow {
    fn clone(&self) -> Self {
        Self {
            wnd: self.wnd.clone(),
            options: DialogOptions {
                title: self.options.title.clone(),
                header: self.options.header.clone(),
                body: self.options.body.clone(),
                buttons: self
                    .options
                    .buttons
                    .iter()
                    .map(|b| DialogButtonSpec { result: b.result, label: b.label.clone(), is_primary: b.is_primary })
                    .collect(),
                footer_text: self.options.footer_text.clone(),
                footer_link: self.options.footer_link.clone(),
            },
            result: self.result.clone(),
            dpi_scale: self.dpi_scale.clone(),
            theme: self.theme.clone(),
            logo_data: self.logo_data.clone(),
            title_font: self.title_font.clone(),
            header_font: self.header_font.clone(),
            body_font: self.body_font.clone(),
            button_font: self.button_font.clone(),
            button_sub_font: self.button_sub_font.clone(),
            footer_font: self.footer_font.clone(),
            is_close_hovered: self.is_close_hovered.clone(),
            hovered_button: self.hovered_button.clone(),
        }
    }
}

fn init_fonts(dialog: &ModernDialogWindow, scale: f32) {
    unsafe {
        let mut ncm = NONCLIENTMETRICSW { cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32, ..Default::default() };
        let _ = SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            ncm.cbSize,
            Some(&mut ncm as *mut _ as *mut _),
            windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );

        let mut lf_title = ncm.lfMessageFont;
        lf_title.lfHeight = -((12.0 * 96.0 / 72.0) * scale) as i32;
        lf_title.lfQuality = CLEARTYPE_QUALITY;
        let hfont_title = CreateFontIndirectW(&lf_title);
        if !hfont_title.0.is_null() {
            *dialog.title_font.borrow_mut() = Some(DeleteObjectGuard::new(w::HFONT::from_ptr(hfont_title.0)));
        }

        let mut lf_header = ncm.lfMessageFont;
        lf_header.lfHeight = -((18.0 * 96.0 / 72.0) * scale) as i32;
        lf_header.lfWeight = 700;
        lf_header.lfQuality = CLEARTYPE_QUALITY;
        let hfont_header = CreateFontIndirectW(&lf_header);
        if !hfont_header.0.is_null() {
            *dialog.header_font.borrow_mut() = Some(DeleteObjectGuard::new(w::HFONT::from_ptr(hfont_header.0)));
        }

        let mut lf_body = ncm.lfMessageFont;
        lf_body.lfHeight = -((10.0 * 96.0 / 72.0) * scale) as i32;
        lf_body.lfQuality = CLEARTYPE_QUALITY;
        let hfont_body = CreateFontIndirectW(&lf_body);
        if !hfont_body.0.is_null() {
            *dialog.body_font.borrow_mut() = Some(DeleteObjectGuard::new(w::HFONT::from_ptr(hfont_body.0)));
        }

        let mut lf_button = ncm.lfMessageFont;
        lf_button.lfHeight = -((10.0 * 96.0 / 72.0) * scale) as i32;
        lf_button.lfWeight = 600;
        lf_button.lfQuality = CLEARTYPE_QUALITY;
        let hfont_button = CreateFontIndirectW(&lf_button);
        if !hfont_button.0.is_null() {
            *dialog.button_font.borrow_mut() = Some(DeleteObjectGuard::new(w::HFONT::from_ptr(hfont_button.0)));
        }

        let mut lf_button_sub = ncm.lfMessageFont;
        lf_button_sub.lfHeight = -((8.0 * 96.0 / 72.0) * scale) as i32;
        lf_button_sub.lfQuality = CLEARTYPE_QUALITY;
        let hfont_button_sub = CreateFontIndirectW(&lf_button_sub);
        if !hfont_button_sub.0.is_null() {
            *dialog.button_sub_font.borrow_mut() = Some(DeleteObjectGuard::new(w::HFONT::from_ptr(hfont_button_sub.0)));
        }

        let mut lf_footer = ncm.lfMessageFont;
        lf_footer.lfHeight = -((8.0 * 96.0 / 72.0) * scale) as i32;
        lf_footer.lfQuality = CLEARTYPE_QUALITY;
        let hfont_footer = CreateFontIndirectW(&lf_footer);
        if !hfont_footer.0.is_null() {
            *dialog.footer_font.borrow_mut() = Some(DeleteObjectGuard::new(w::HFONT::from_ptr(hfont_footer.0)));
        }
    }
}

fn draw_dialog_text(dialog: &ModernDialogWindow, hdc: &w::HDC, layout: &DialogLayout) {
    let theme = *dialog.theme.borrow();
    let colors = theme_colors(theme);
    if let Some(font) = dialog.title_font.borrow().as_ref() {
        windows_ui::draw_text(
            hdc,
            font,
            colors.title_text,
            &layout.title_rect,
            co::DT::LEFT | co::DT::SINGLELINE | co::DT::VCENTER,
            &dialog.options.title,
        )
        .ok();
    }

    if let (Some(header), Some(rect), Some(font)) =
        (dialog.options.header.as_ref(), layout.header_rect, dialog.header_font.borrow().as_ref())
    {
        let is_single_line = !header.contains('\n');
        let flags = if is_single_line { co::DT::LEFT | co::DT::SINGLELINE | co::DT::VCENTER } else { co::DT::LEFT | co::DT::WORDBREAK };
        windows_ui::draw_text(hdc, font, colors.header_text, &rect, flags, header).ok();
    }

    if let Some(font) = dialog.body_font.borrow().as_ref() {
        windows_ui::draw_text(
            hdc,
            font,
            colors.body_text,
            &layout.body_rect,
            co::DT::LEFT | co::DT::WORDBREAK,
            &dialog.options.body,
        )
        .ok();
    }

    for (idx, rect) in layout.button_rects.iter().enumerate() {
        let label = &dialog.options.buttons[idx].label;
        let (main_text, sub_text) = split_button_label(label);
        if layout.has_command_links && sub_text.is_some() {
            let text_pad = (12.0 * *dialog.dpi_scale.borrow()) as i32;
            let text_rect = w::RECT {
                left: rect.left + text_pad,
                top: rect.top + (6.0 * *dialog.dpi_scale.borrow()) as i32,
                right: rect.right - text_pad,
                bottom: rect.bottom - (6.0 * *dialog.dpi_scale.borrow()) as i32,
            };
            let dpi = *dialog.dpi_scale.borrow();
            let main_h = (14.0 * dpi) as i32;
            let sub_h = (12.0 * dpi) as i32;
            let total_h = main_h + sub_h;
            let start_y = text_rect.top + ((text_rect.bottom - text_rect.top - total_h).max(0) / 2);
            if let Some(font) = dialog.button_font.borrow().as_ref() {
                let main_rect = w::RECT { left: text_rect.left, top: start_y, right: text_rect.right, bottom: start_y + main_h };
                windows_ui::draw_text(
                    hdc,
                    font,
                    button_text_color(theme, dialog.options.buttons[idx].is_primary, false),
                    &main_rect,
                    co::DT::LEFT | co::DT::SINGLELINE | co::DT::VCENTER,
                    &main_text,
                )
                .ok();
            }
            if let (Some(sub), Some(font)) = (sub_text, dialog.button_sub_font.borrow().as_ref()) {
                let sub_rect = w::RECT { left: text_rect.left, top: start_y + main_h, right: text_rect.right, bottom: start_y + main_h + sub_h };
                windows_ui::draw_text(
                    hdc,
                    font,
                    button_text_color(theme, dialog.options.buttons[idx].is_primary, true),
                    &sub_rect,
                    co::DT::LEFT | co::DT::WORDBREAK,
                    &sub,
                )
                .ok();
            }
        } else if let Some(font) = dialog.button_font.borrow().as_ref() {
            windows_ui::draw_text(
                hdc,
                font,
                button_text_color(theme, dialog.options.buttons[idx].is_primary, false),
                rect,
                co::DT::CENTER | co::DT::SINGLELINE | co::DT::VCENTER,
                &main_text,
            )
            .ok();
        }
    }

    if let (Some(footer_text), Some(footer_rect), Some(font)) =
        (dialog.options.footer_text.as_ref(), layout.footer_rect, dialog.footer_font.borrow().as_ref())
    {
        let color = if dialog.options.footer_link.is_some() { colors.footer_link } else { colors.footer_text };
        windows_ui::draw_text(hdc, font, color, &footer_rect, co::DT::LEFT | co::DT::SINGLELINE | co::DT::VCENTER, footer_text).ok();
    }
}

fn build_layout(dialog: &ModernDialogWindow, width: i32, height: i32) -> DialogLayout {
    let dpi = *dialog.dpi_scale.borrow();
    let margin = (16.0 * dpi) as i32;
    let gap = (8.0 * dpi) as i32;
    let title_h = (20.0 * dpi) as i32;
    let header_h = if dialog.options.header.is_some() { (32.0 * dpi) as i32 } else { 0 };
    let footer_h = if dialog.options.footer_text.is_some() { (14.0 * dpi) as i32 } else { 0 };
    let logo_size = (48.0 * dpi) as i32;
    let has_logo = dialog.logo_data.as_ref().is_some();
    let top_row_h = logo_size.max(title_h);

    let has_command_links = true;
    let buttons_area_h = if has_command_links {
        let btn_h = (52.0 * dpi) as i32;
        let spacing = gap;
        (dialog.options.buttons.len() as i32 * btn_h) + ((dialog.options.buttons.len() as i32 - 1).max(0) * spacing)
    } else {
        (32.0 * dpi) as i32
    };

    let close_size = (32.0 * dpi) as i32;
    let close_offset = margin;
    let close_rect = w::RECT {
        left: width - close_offset - close_size,
        top: close_offset,
        right: width - close_offset,
        bottom: close_offset + close_size,
    };

    let logo_rect = if has_logo {
        Some(w::RECT { left: margin, top: close_offset, right: margin + logo_size, bottom: close_offset + logo_size })
    } else {
        None
    };
    let title_left = logo_rect.map(|r| r.right + gap).unwrap_or(margin);
    let title_rect = w::RECT { left: title_left, top: close_offset, right: close_rect.left - gap, bottom: close_offset + top_row_h };

    let header_top = close_offset + top_row_h + gap;
    let header_rect = if header_h > 0 {
        Some(w::RECT { left: margin, top: header_top, right: width - margin, bottom: header_top + header_h })
    } else {
        None
    };

    let body_top = header_rect.map(|r| r.bottom + gap).unwrap_or(title_rect.bottom + gap);
    let footer_bottom = height - margin;
    let footer_rect = if footer_h > 0 {
        Some(w::RECT { left: margin, top: footer_bottom - footer_h, right: width - margin, bottom: footer_bottom })
    } else {
        None
    };

    let buttons_bottom = if let Some(fr) = footer_rect { fr.top - gap } else { footer_bottom };
    let buttons_top = buttons_bottom - buttons_area_h;
    let body_bottom = buttons_top - gap;
    let body_rect = w::RECT { left: margin, top: body_top, right: width - margin, bottom: body_bottom.max(body_top + (72.0 * dpi) as i32) };

    let mut button_rects = Vec::with_capacity(dialog.options.buttons.len());
    if has_command_links {
        let btn_h = (52.0 * dpi) as i32;
        let spacing = (6.0 * dpi) as i32;
        let btn_w = width - margin * 2;
        let mut y = buttons_top;
        for _ in 0..dialog.options.buttons.len() {
            button_rects.push(w::RECT { left: margin, top: y, right: margin + btn_w, bottom: y + btn_h });
            y += btn_h + spacing;
        }
    } else {
        let btn_w = (110.0 * dpi) as i32;
        let btn_h = (32.0 * dpi) as i32;
        let spacing = gap;
        let total_w = (dialog.options.buttons.len() as i32 * btn_w) + ((dialog.options.buttons.len() as i32 - 1).max(0) * spacing);
        let start_x = width - margin - total_w;
        let y = buttons_top + ((buttons_area_h - btn_h) / 2);
        for i in 0..dialog.options.buttons.len() {
            let x = start_x + i as i32 * (btn_w + spacing);
            button_rects.push(w::RECT { left: x, top: y, right: x + btn_w, bottom: y + btn_h });
        }
    }

    DialogLayout { close_rect, logo_rect, title_rect, header_rect, body_rect, button_rects, footer_rect, has_command_links }
}

fn split_button_label(label: &str) -> (String, Option<String>) {
    if let Some((head, tail)) = label.split_once('\n') {
        (head.to_string(), Some(tail.to_string()))
    } else {
        (label.to_string(), None)
    }
}

fn default_dialog_result(opts: &DialogOptions) -> DialogResult {
    if opts.buttons.iter().any(|b| b.result == DialogResult::Cancel) {
        DialogResult::Cancel
    } else {
        DialogResult::Ok
    }
}

fn rect_contains(rect: &w::RECT, pt: w::POINT) -> bool {
    pt.x >= rect.left && pt.x <= rect.right && pt.y >= rect.top && pt.y <= rect.bottom
}

fn calc_window_size(opts: &DialogOptions, scale: f32) -> (i32, i32) {
    let width = (460.0 * scale) as i32;
    let padding = 16.0 * scale;
    let gap = 8.0 * scale;
    let logo_size = 48.0 * scale;
    let title_h = 20.0 * scale;
    let top_row_h = logo_size.max(title_h);
    let has_command_links = true;
    let buttons_area_h = if has_command_links {
        let btn_h = 52.0 * scale;
        let spacing = gap;
        (opts.buttons.len() as f32 * btn_h) + ((opts.buttons.len().saturating_sub(1)) as f32 * spacing)
    } else {
        32.0 * scale
    };
    let header_h = if opts.header.is_some() { 32.0 * scale } else { 0.0 };
    let footer_h = if opts.footer_text.is_some() { 14.0 * scale } else { 0.0 };
    let body_h = 102.0 * scale;
    let height = (padding
        + top_row_h
        + gap
        + header_h
        + if header_h > 0.0 { gap } else { 0.0 }
        + body_h
        + gap
        + buttons_area_h
        + if footer_h > 0.0 { gap } else { 0.0 }
        + footer_h
        + padding) as i32;
    (width, height)
}

extern "system" fn task_dialog_callback(hwnd: w::HWND, msg: co::TDN, _: usize, _: isize, lp_ref_data: usize) -> co::HRESULT {
    if msg == co::TDN::CREATED {
        let raw_hwnd = hwnd.ptr();
        if !raw_hwnd.is_null() {
            windows_ui::apply_window_style(WinHWND(raw_hwnd), windows_ui::current_theme());
        }
        return co::HRESULT::S_OK;
    }
    if msg == co::TDN::HYPERLINK_CLICKED {
        if lp_ref_data != 0 {
            let raw = lp_ref_data as *const PathBuf;
            let path: &PathBuf = unsafe { &*raw };
            let dir = path.to_str().unwrap();
            w::HWND::GetDesktopWindow().ShellExecute("open", &dir, None, None, co::SW::SHOWDEFAULT).ok();
            return co::HRESULT::S_FALSE; // do not close dialog
        }
    }
    return co::HRESULT::S_OK; // close dialog on button press
}

pub fn generate_confirm(
    title: &str,
    header: Option<&str>,
    body: &str,
    ok_text: Option<&str>,
    btns: DialogButton,
    ico: DialogIcon,
) -> Result<DialogResult> {
    let hparent = w::HWND::GetDesktopWindow();
    let mut ok_text_buf = WString::from_opt_str(ok_text);
    let mut custom_btns = if ok_text.is_some() {
        let mut td_btn = w::TASKDIALOG_BUTTON::default();
        td_btn.set_nButtonID(co::DLGID::OK.into());
        td_btn.set_pszButtonText(Some(&mut ok_text_buf));
        let mut custom_btns = Vec::with_capacity(1);
        custom_btns.push(td_btn);
        custom_btns
    } else {
        Vec::<w::TASKDIALOG_BUTTON>::default()
    };

    let mut tdc = w::TASKDIALOGCONFIG::default();
    tdc.hwndParent = unsafe { hparent.raw_copy() };
    tdc.dwFlags = co::TDF::ALLOW_DIALOG_CANCELLATION | co::TDF::POSITION_RELATIVE_TO_WINDOW;
    tdc.dwCommonButtons = btns.to_win();
    tdc.set_pszMainIcon(w::IconIdTdicon::Tdicon(ico.to_win()));

    if ok_text.is_some() {
        tdc.set_pButtons(Some(&mut custom_btns));
    }
    tdc.pfCallback = Some(task_dialog_callback);

    let mut title_buf = WString::from_str(title);
    tdc.set_pszWindowTitle(Some(&mut title_buf));

    let mut header_buf = WString::from_opt_str(header);
    if header.is_some() {
        tdc.set_pszMainInstruction(Some(&mut header_buf));
    }

    let mut body_buf = WString::from_str(body);
    tdc.set_pszContent(Some(&mut body_buf));

    let result = w::TaskDialogIndirect(&tdc, None).map(|(dlg_id, _)| dlg_id)?;
    Ok(DialogResult::from_win(result))
}

pub fn generate_alert(
    title: &str,
    header: Option<&str>,
    body: &str,
    ok_text: Option<&str>,
    btns: DialogButton,
    ico: DialogIcon,
) -> Result<()> {
    let _ = generate_confirm(title, header, body, ok_text, btns, ico).map(|_| ())?;
    Ok(())
}
