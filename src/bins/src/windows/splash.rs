use i_slint_backend_winit::WinitWindowAccessor;
use slint::ComponentHandle;
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

slint::include_modules!();

pub const MSG_CLOSE: i16 = -1;
pub const MSG_INDEFINITE: i16 = -2;
pub const MSG_START_INSTALL: i16 = -3;

/// 在 Windows 上隐藏标题栏但保留圆角和阴影
/// 使用 DWM API 扩展客户区到标题栏区域
#[cfg(target_os = "windows")]
fn apply_windows_decorations(w: &i_slint_backend_winit::winit::window::Window) {
    use i_slint_backend_winit::winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    if let Ok(handle) = w.window_handle() {
        if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
            use windows::core::BOOL;
            use windows::Win32::Foundation::HWND;
            use windows::Win32::Graphics::Dwm::{DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
            use windows::Win32::UI::Controls::MARGINS;

            let hwnd = HWND(win32_handle.hwnd.get() as *mut _);

            unsafe {
                // 使用 DWM 扩展客户区到整个窗口（包括标题栏区域）
                // 设置 top margin 为 -1 表示完全隐藏标题栏但保留边框效果
                let margins = MARGINS {
                    cxLeftWidth: 0,
                    cxRightWidth: 0,
                    cyTopHeight: -1, // -1 表示扩展到整个窗口
                    cyBottomHeight: 0,
                };
                let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

                // 启用深色模式边框（可选，用于深色主题）
                let use_dark_mode: BOOL = BOOL::from(true);
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    &use_dark_mode as *const _ as *const _,
                    std::mem::size_of::<BOOL>() as u32,
                );
            }
        }
    }
}

/// 执行一次性的窗口装饰设置
fn setup_native_decorations<W: ComponentHandle + 'static>(window_handle: &W) {
    #[cfg(target_os = "macos")]
    {
        window_handle.window().with_winit_window(|w| {
            use i_slint_backend_winit::winit::platform::macos::WindowExtMacOS;
            w.set_titlebar_transparent(true);
            w.set_title_hidden(true);
            w.set_fullsize_content_view(true);
        });
    }

    #[cfg(target_os = "windows")]
    {
        let weak = window_handle.as_weak();
        // 使用异步 spawn，在窗口句柄准备好后只执行一次
        slint::spawn_local(async move {
            if let Some(w_handle) = weak.upgrade() {
                if let Ok(winit_window) = w_handle.window().winit_window().await {
                    apply_windows_decorations(&winit_window);
                }
            }
        })
        .unwrap();
    }
}

pub fn show_progress_dialog<T1: AsRef<str>, T2: AsRef<str>>(window_title: T1, content: T2) -> Sender<i16> {
    let title_str = window_title.as_ref().to_string();
    let _status = content.as_ref().to_string();
    let (app_clean_name, sub_text) = if let Some(stripped) = title_str.strip_suffix(" Update") {
        (stripped.to_string(), "正在更新")
    } else if let Some(stripped) = title_str.strip_suffix(" Setup") {
        (stripped.to_string(), "正在安装")
    } else {
        (title_str, "正在安装")
    };

    let (tx, rx) = mpsc::channel::<i16>();

    thread::spawn(move || {
        let window = SetupWindow::new().unwrap();

        // 执行一次性装饰初始化
        setup_native_decorations(&window);
        let _ = window.show();

        window.set_app_name(app_clean_name.into());
        window.set_sub_text(sub_text.into());
        window.set_current_step(0); // Welcome 页面

        let weak = window.as_weak();
        window.on_close(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
        });

        let weak = window.as_weak();
        window.on_window_move(move |dx, dy| {
            if let Some(w) = weak.upgrade() {
                let window = w.window();
                let scale_factor = window.scale_factor();
                let pos = window.position();
                let new_pos = slint::PhysicalPosition::new(pos.x + (dx * scale_factor) as i32, pos.y + (dy * scale_factor) as i32);
                window.set_position(new_pos);
            }
        });

        // 点击开始安装按钮后切换到安装页面
        let weak = window.as_weak();
        window.on_start_install(move || {
            if let Some(w) = weak.upgrade() {
                w.set_current_step(1); // Installing 页面
            }
        });

        // 点击启动应用按钮后关闭窗口
        let weak = window.as_weak();
        window.on_launch_app(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
        });

        let weak_progress = window.as_weak();
        spawn_progress_loop(weak_progress, rx);
        let _ = window.run();
    });
    tx
}

pub fn show_splash_dialog(app_name: String) -> Sender<i16> {
    let (tx, rx) = mpsc::channel::<i16>();

    thread::spawn(move || {
        let window = SetupWindow::new().unwrap();

        // 执行一次性装饰初始化
        setup_native_decorations(&window);
        let _ = window.show();

        window.set_app_name(app_name.clone().into());
        window.set_sub_text("正在安装".into());
        window.set_current_step(0); // Welcome 页面

        let weak = window.as_weak();
        window.on_close(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
        });

        let weak = window.as_weak();
        window.on_window_move(move |dx, dy| {
            if let Some(w) = weak.upgrade() {
                let window = w.window();
                let scale_factor = window.scale_factor();
                let pos = window.position();
                let new_pos = slint::PhysicalPosition::new(pos.x + (dx * scale_factor) as i32, pos.y + (dy * scale_factor) as i32);
                window.set_position(new_pos);
            }
        });

        // 点击开始安装按钮后切换到安装页面
        let weak = window.as_weak();
        window.on_start_install(move || {
            if let Some(w) = weak.upgrade() {
                w.set_current_step(1); // Installing 页面
            }
        });

        // 点击启动应用按钮后关闭窗口
        let weak = window.as_weak();
        window.on_launch_app(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
        });

        let weak_progress = window.as_weak();
        spawn_progress_loop(weak_progress, rx);
        let _ = window.run();
    });
    tx
}

pub fn show_overwrite_dialog(
    app_name: String,
    dialog_type: String,
    old_version: String,
    new_version: String,
    install_path: String,
) -> bool {
    let (tx, rx) = mpsc::channel::<bool>();

    thread::spawn(move || {
        let window = OverwriteDialog::new().unwrap();

        // 执行一次性装饰初始化
        setup_native_decorations(&window);
        let _ = window.show();

        window.set_app_name(app_name.into());
        window.set_dialog_type(dialog_type.into());
        window.set_old_version(old_version.into());
        window.set_new_version(new_version.into());
        window.set_install_path(install_path.into());

        let weak = window.as_weak();
        let tx_confirm = tx.clone();
        window.on_confirm(move || {
            let _ = tx_confirm.send(true);
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
        });

        let weak = window.as_weak();
        let tx_cancel = tx.clone();
        window.on_cancel(move || {
            let _ = tx_cancel.send(false);
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
        });

        let weak = window.as_weak();
        window.on_window_move(move |dx, dy| {
            if let Some(w) = weak.upgrade() {
                let window = w.window();
                let scale_factor = window.scale_factor();
                let pos = window.position();
                let new_pos = slint::PhysicalPosition::new(pos.x + (dx * scale_factor) as i32, pos.y + (dy * scale_factor) as i32);
                window.set_position(new_pos);
            }
        });

        let _ = window.run();
    });

    rx.recv().unwrap_or(false)
}

pub fn show_msg_dialog(title: String, header: String, body: String, dialog_type: String, buttons: Vec<String>) -> usize {
    let (tx, rx) = mpsc::channel::<usize>();

    thread::spawn(move || {
        let window = MessageDialog::new().unwrap();

        // 执行一次性装饰初始化
        setup_native_decorations(&window);
        let _ = window.show();

        window.set_dialog_title(title.into());
        window.set_heading(header.into());
        window.set_text(body.into());
        window.set_type(dialog_type.into());

        let model =
            std::rc::Rc::new(slint::VecModel::from(buttons.iter().map(|s| s.as_str().into()).collect::<Vec<slint::SharedString>>()));
        window.set_buttons(model.into());

        let weak = window.as_weak();
        let tx = tx.clone();
        window.on_close_dialog(move |index| {
            let _ = tx.send(index as usize);
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
        });

        let weak = window.as_weak();
        window.on_window_move(move |dx, dy| {
            if let Some(w) = weak.upgrade() {
                let window = w.window();
                let scale_factor = window.scale_factor();
                let pos = window.position();
                let new_pos = slint::PhysicalPosition::new(pos.x + (dx * scale_factor) as i32, pos.y + (dy * scale_factor) as i32);
                window.set_position(new_pos);
            }
        });

        let _ = window.run();
    });

    rx.recv().unwrap_or(0)
}

fn spawn_progress_loop<W: ComponentHandle + 'static>(weak: slint::Weak<W>, rx: Receiver<i16>)
where
    W: ProgressSetter,
{
    thread::spawn(move || {
        while let Ok(msg) = rx.recv() {
            let w = weak.clone();
            if msg == MSG_CLOSE {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = w.upgrade() {
                        let _ = w.hide();
                    }
                });
                break;
            } else if msg == MSG_START_INSTALL {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = w.upgrade() {
                        w.set_current_step(1); // 切换到 Installing 页面
                    }
                });
            } else if msg >= 0 {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = w.upgrade() {
                        w.set_progress(msg as f32);
                        // 进度达到 100% 时自动切换到 Complete 页面
                        if msg >= 100 {
                            w.set_current_step(2);
                        }
                    }
                });
            }
        }
    });
}

trait ProgressSetter {
    fn set_progress(&self, value: f32);
    fn set_current_step(&self, step: i32);
}

impl ProgressSetter for SetupWindow {
    fn set_progress(&self, value: f32) {
        self.set_progress(value);
    }

    fn set_current_step(&self, step: i32) {
        self.set_current_step(step);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_show_splash() {
        let tx = show_splash_dialog("Velopack 应用".to_string());
        let _ = tx.send(20);
        std::thread::sleep(std::time::Duration::from_secs(1));
        let _ = tx.send(60);
        std::thread::sleep(std::time::Duration::from_secs(1));
        let _ = tx.send(100);
        // 延长观察时间，防止过快关闭
        println!("Window will stay open for 60 seconds for inspection...");
        std::thread::sleep(std::time::Duration::from_secs(60));
        let _ = tx.send(MSG_CLOSE);
    }

    #[test]
    #[ignore]
    fn test_show_setup() {
        let tx = show_progress_dialog("Velopack 安装", "正在准备环境...");
        // 等待用户手动点击"开始安装"按钮
        println!("请点击'开始安装'按钮...");
        std::thread::sleep(std::time::Duration::from_secs(5));
        // 模拟进度更新
        for i in 0..=100 {
            let _ = tx.send(i as i16);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        // 延长观察时间，查看完成页面
        println!("安装完成，窗口将保持 60 秒...");
        std::thread::sleep(std::time::Duration::from_secs(60));
        let _ = tx.send(MSG_CLOSE);
    }

    #[test]
    #[ignore]
    fn test_show_overwrite() {
        let result = show_overwrite_dialog(
            "Visual Studio Code".to_string(),
            "update".to_string(),
            "1.85.0".to_string(),
            "1.86.0".to_string(),
            "%LocalAppData%\\Programs\\Microsoft VS Code".to_string(),
        );
        println!("Dialog result: {}", result);
    }

    #[test]
    #[ignore]
    fn test_show_message() {
        let buttons = vec!["Retry".to_string(), "Continue".to_string(), "Cancel".to_string()];
        let result = show_msg_dialog(
            "Velopack Error".to_string(),
            "Application Error".to_string(),
            "The application failed to start because of a missing dependency.\n\nPlease install .NET Runtime expecting version >= 6.0."
                .to_string(),
            "error".to_string(),
            buttons,
        );
        println!("Message Dialog Result Index: {}", result);
    }
}
