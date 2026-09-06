//! Menu-bar extra: clipboard history + Polish + Settings.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
#[cfg(target_os = "macos")]
use tauri::Emitter;

use mabel_lib::clipboard_history::Service;
use mabel_lib::polish;

static POLLER_STARTED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "macos")]
fn current_polish_mode(app: &AppHandle) -> String {
    app.try_state::<crate::AppState>()
        .map(|s| polish::effective_mode(&s.settings.lock().unwrap().polish_mode))
        .unwrap_or_else(polish::default_mode)
}

#[cfg(target_os = "macos")]
fn build_menu(app: &AppHandle) -> Result<tauri::menu::Menu<tauri::Wry>, String> {
    use tauri::menu::{CheckMenuItem, Menu, MenuItem, Submenu};

    let mode = current_polish_mode(app);
    let entitled = mabel_lib::storekit::current_entitlement().entitled;
    let history = MenuItem::with_id(app, "clipboard-history", "Clipboard History…", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let polish_off = CheckMenuItem::with_id(
        app,
        "polish-off",
        "Off",
        true,
        mode == polish::MODE_OFF,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    // Live modes stay visible so Free sees the surface, but they are locked
    // until Pro. Click still upsells via require_pro → Plans.
    let polish_casual = CheckMenuItem::with_id(
        app,
        "polish-casual",
        "Casual",
        entitled,
        entitled && mode == polish::MODE_CASUAL,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let polish_professional = CheckMenuItem::with_id(
        app,
        "polish-professional",
        "Professional",
        entitled,
        entitled && mode == polish::MODE_PROFESSIONAL,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let polish_polite = CheckMenuItem::with_id(
        app,
        "polish-polite",
        "Polite",
        entitled,
        entitled && mode == polish::MODE_POLITE,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let polish_menu = Submenu::with_id_and_items(
        app,
        "polish",
        "Polish",
        true,
        &[
            &polish_off,
            &polish_casual,
            &polish_professional,
            &polish_polite,
        ],
    )
    .map_err(|e| e.to_string())?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let quit = MenuItem::with_id(app, "quit", "Quit Mabel", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    Menu::with_items(app, &[&history, &polish_menu, &settings, &quit]).map_err(|e| e.to_string())
}

pub fn refresh_tray(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    if let Some(tray) = app.tray_by_id("mabel-status") {
        if let Ok(menu) = build_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

#[cfg(target_os = "macos")]
fn apply_polish_from_tray(app: &AppHandle, mode: &str) {
    match persist_polish(app, mode) {
        Ok(mode) => {
            let _ = app.emit("polish-changed", &mode);
            refresh_tray(app);
        }
        Err(_) => {
            // Free cannot enable — Settings → Plans, never a website.
            show_main_window(app);
            let _ = app.emit("open-plans", ());
        }
    }
}

#[cfg(target_os = "macos")]
fn persist_polish(app: &AppHandle, mode: &str) -> Result<String, String> {
    // Enforcer BOUND: Polish only. Do not write the clipboard opt-in or a Nexus store.
    let mode = polish::require_mode_allowed(mode)?;
    let Some(state) = app.try_state::<crate::AppState>() else {
        return Err("app state unavailable".into());
    };
    let mut held = state.settings.lock().unwrap();
    held.polish_mode = mode.clone();
    if polish::is_live(&mode) {
        held.cleanup_mode = "llm".into();
    }
    held.save(&state.app_dir)?;
    Ok(mode)
}

#[cfg(target_os = "macos")]
pub fn install_tray(app: &AppHandle) -> Result<(), String> {
    use tauri::tray::TrayIconBuilder;

    let menu = build_menu(app)?;

    let mut builder = TrayIconBuilder::with_id("mabel-status")
        .menu(&menu)
        .tooltip("Mabel")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "clipboard-history" => {
                if let Err(e) = show_history_window(app) {
                    eprintln!("[Mabel] open clipboard history: {e}");
                }
            }
            "polish-off" => apply_polish_from_tray(app, polish::MODE_OFF),
            "polish-casual" => apply_polish_from_tray(app, polish::MODE_CASUAL),
            "polish-professional" => apply_polish_from_tray(app, polish::MODE_PROFESSIONAL),
            "polish-polite" => apply_polish_from_tray(app, polish::MODE_POLITE),
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
