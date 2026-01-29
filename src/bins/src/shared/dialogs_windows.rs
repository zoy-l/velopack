use super::{dialogs_common::*, dialogs_const::*};
use crate::windows::splash;
use anyhow::Result;

use velopack::bundle::Manifest;

pub fn show_restart_required(app: &Manifest) {
    show_warn(
        format!("{} Setup {}", app.title, app.version).as_str(),
        Some("Restart Required"),
        "A restart is required before Setup can continue. Please restart your computer and try again.",
    );
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

    show_ok_cancel(
        format!("{} Update", app.title).as_str(),
        Some(format!("{} would like to update from {} to {}", app.title, from, to).as_str()),
        format!(
            "{} {to} has missing dependencies which need to be installed: {}, would you like to continue?",
            app.title, depedency_string
        )
        .as_str(),
        Some("Install & Update"),
    )
}

pub fn show_setup_missing_dependencies_dialog(app: &Manifest, depedency_string: &str) -> bool {
    if get_silent() {
        return true;
    }

    show_ok_cancel(
        format!("{} Setup {}", app.title, app.version).as_str(),
        Some(format!("{} has missing system dependencies.", app.title).as_str()),
        format!("{} requires the following packages to be installed: {}, would you like to continue?", app.title, depedency_string)
            .as_str(),
        Some("Install"),
    )
}

pub fn show_processes_locking_folder_dialog(app_title: &str, app_version: &str, process_names: &str) -> DialogResult {
    if get_silent() {
        return DialogResult::Cancel;
    }

    let title = format!("{} Update {}", app_title, app_version);
    let header = format!("{} Update", app_title);
    let body = format!(
        "There are programs ({}) preventing the {} update from proceeding. \n\n\
        You can press Continue to have this updater attempt to close them automatically, or if you've closed them yourself press Retry for the updater to check again.",
        process_names, app_title
    );

    let buttons = vec!["Retry".to_string(), "Continue".to_string(), "Cancel".to_string()];
    let idx = splash::show_msg_dialog(title, header, body, "info".to_string(), buttons);
    match idx {
        0 => DialogResult::Retry,
        1 => DialogResult::Continue,
        _ => DialogResult::Cancel,
    }
}

pub fn generate_confirm(
    title: &str,
    header: Option<&str>,
    body: &str,
    ok_text: Option<&str>,
    btns: DialogButton,
    ico: DialogIcon,
) -> Result<DialogResult> {
    let mut btn_labels = Vec::new();
    let mut btn_results = Vec::new();

    if let Some(text) = ok_text {
        btn_labels.push(text.to_string());
        btn_results.push(DialogResult::Ok);
    } else if btns.has_ok() {
        btn_labels.push("OK".to_string());
        btn_results.push(DialogResult::Ok);
    }

    if btns.has_yes() {
        btn_labels.push("Yes".to_string());
        btn_results.push(DialogResult::Yes);
    }
    if btns.has_no() {
        btn_labels.push("No".to_string());
        btn_results.push(DialogResult::No);
    }
    if btns.has_retry() {
        btn_labels.push("Retry".to_string());
        btn_results.push(DialogResult::Retry);
    }
    if btns.has_cancel() {
        btn_labels.push("Cancel".to_string());
        btn_results.push(DialogResult::Cancel);
    }
    if btns.has_close() {
        btn_labels.push("Close".to_string());
        btn_results.push(DialogResult::Cancel);
    }

    let icon_str = match ico {
        DialogIcon::Error => "error",
        DialogIcon::Warning => "warning",
        DialogIcon::Information => "info",
    };

    let idx =
        splash::show_msg_dialog(title.to_string(), header.unwrap_or("").to_string(), body.to_string(), icon_str.to_string(), btn_labels);

    if idx < btn_results.len() {
        Ok(btn_results[idx])
    } else {
        Ok(DialogResult::Cancel)
    }
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
