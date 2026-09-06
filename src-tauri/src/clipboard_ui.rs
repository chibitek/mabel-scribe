//! Menu-bar extra and history window for Mac clipboard history.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
#[cfg(target_os = "macos")]
use tauri::Emitter;

use mabel_lib::clipboard_history::Service;

static POLLER_STARTED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "macos")]
pub fn install_tray(app: &AppHandle) -> Result<(), String> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let history = MenuItem::with_id(app, "clipboard-history", "Clipboard History…", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let quit = MenuItem::with_id(app, "quit", "Quit Mabel", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let menu = Menu::with_items(app, &[&history, &settings, &quit]).map_err(|e| e.to_string())?;

    let mut builder = TrayIconBuilder::with_id("mabel-status")
        .menu(&menu)
        .tooltip("Mabel")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "clipboard-history" => {
                if let Err(e) = show_history_window(app) {
                    eprintln!("[Mabel] open clipboard history: {e}");
                }
            }
            "settings" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn install_tray(_app: &AppHandle) -> Result<(), String> {
    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn show_history_window(app: &AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window("clipboard-history") {
        let _ = existing.show();
        let _ = existing.unminimize();
        let _ = existing.set_focus();
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        "clipboard-history",
        WebviewUrl::App("src/clipboard-history.html".into()),
    )
    .title("Clipboard History")
    .inner_size(400.0, 560.0)
    .min_inner_size(320.0, 360.0)
    .resizable(true)
    .skip_taskbar(true)
    .build()
    .map_err(|e| e.to_string())?;
    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
}

pub fn hide_history_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("clipboard-history") {
        let _ = w.hide();
    }
}

pub fn spawn_poller(app: AppHandle, service: Arc<Service>) {
    if POLLER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let mut last_change: i64 = 0;
        let mut primed = false;
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if !service.is_enabled() {
                // Fail closed / never always-on spy: do not poll
                // changeCount or read contents while opted out.
                primed = false;
                last_change = 0;
                continue;
            }
            #[cfg(target_os = "macos")]
            {
                let svc = service.clone();
                let before = last_change;
                mabel_lib::pasteboard_macos::poll_capture(&svc, &mut last_change, &mut primed);
                if last_change != before && primed {
                    let _ = app.emit("clipboard-history-updated", ());
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = (&app, &service, &mut last_change, &mut primed);
            }
        }
    });
}
