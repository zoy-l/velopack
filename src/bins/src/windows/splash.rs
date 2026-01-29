use slint::ComponentHandle;
use std::{
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

slint::include_modules!();

pub const MSG_CLOSE: i16 = -1;
pub const MSG_INDEFINITE: i16 = -2;

pub fn show_progress_dialog<T1: AsRef<str>, T2: AsRef<str>>(window_title: T1, content: T2) -> Sender<i16> {
    let title_str = window_title.as_ref().to_string();
    let status = content.as_ref().to_string();
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
        window.set_app_name(app_clean_name.into());
        window.set_sub_text(sub_text.into());
        window.set_status_text(status.into());

        let weak = window.as_weak();
        window.on_close(move || {
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
        window.set_app_name(app_name.clone().into());
        window.set_sub_text("正在安装".into());
        window.set_status_text(format!("正在安装 {}...", app_name).into());

        let weak = window.as_weak();
        window.on_close(move || {
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

        let _ = window.run();
    });

    rx.recv().unwrap_or(false)
}

pub fn show_msg_dialog(title: String, header: String, body: String, dialog_type: String, buttons: Vec<String>) -> usize {
    let (tx, rx) = mpsc::channel::<usize>();

    thread::spawn(move || {
        let window = MessageDialog::new().unwrap();
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
            } else if msg >= 0 {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(w) = w.upgrade() {
                        w.set_progress(msg as f32);
                    }
                });
            }
        }
    });
}

trait ProgressSetter {
    fn set_progress(&self, value: f32);
}

impl ProgressSetter for SetupWindow {
    fn set_progress(&self, value: f32) {
        self.set_progress(value);
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
        for i in 0..=100 {
            let _ = tx.send(i as i16);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        // 延长观察时间
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
