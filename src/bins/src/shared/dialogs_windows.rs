use super::{dialogs_common::*, dialogs_const::*};
use anyhow::Result;

use velopack::bundle::Manifest;
use winsafe::{self as w, co, prelude::*, WString};

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

    let mut config: w::TASKDIALOGCONFIG = Default::default();
    config.set_pszMainIcon(w::IconIdTdicon::Tdicon(co::TD_ICON::INFORMATION));

    let mut update_name = WString::from_str(format!("{} Update {}", app_title, app_version));
    let mut instruction = WString::from_str(format!("{} Update", app_title));

    let mut content = WString::from_str(format!(
        "There are programs ({}) preventing the {} update from proceeding. \n\n\
        You can press Continue to have this updater attempt to close them automatically, or if you've closed them yourself press Retry for the updater to check again.",
        process_names, app_title));

    let mut btn_retry_txt = WString::from_str("Retry\nTry again if you've closed the program(s)");
    let mut btn_continue_txt = WString::from_str("Continue\nAttempt to close the program(s) automatically");
    let mut btn_cancel_txt = WString::from_str("Cancel\nThe update will not continue");

    let mut btn_retry = w::TASKDIALOG_BUTTON::default();
    btn_retry.set_nButtonID(co::DLGID::RETRY.into());
    btn_retry.set_pszButtonText(Some(&mut btn_retry_txt));

    let mut btn_continue = w::TASKDIALOG_BUTTON::default();
    btn_continue.set_nButtonID(co::DLGID::CONTINUE.into());
    btn_continue.set_pszButtonText(Some(&mut btn_continue_txt));

    let mut btn_cancel = w::TASKDIALOG_BUTTON::default();
    btn_cancel.set_nButtonID(co::DLGID::CANCEL.into());
    btn_cancel.set_pszButtonText(Some(&mut btn_cancel_txt));

    let mut custom_btns = vec![btn_retry, btn_continue, btn_cancel];
    config.dwFlags = co::TDF::USE_COMMAND_LINKS;
    config.set_pButtons(Some(&mut custom_btns));
    config.set_pszWindowTitle(Some(&mut update_name));
    config.set_pszMainInstruction(Some(&mut instruction));
    config.set_pszContent(Some(&mut content));

    let (btn, _) = w::TaskDialogIndirect(&config, None).ok().unwrap_or((co::DLGID::CANCEL, 0));
    DialogResult::from_win(btn)
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
