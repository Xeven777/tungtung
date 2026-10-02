mod commands;
mod db;
mod error;
mod microbreak;
mod models;
mod notify;
mod platform;
mod prefs;
mod recurrence;
mod scheduler;

use std::sync::{Arc, Mutex};

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use commands::AppState;
use microbreak::MicroBreak;
use prefs::RuntimePrefs;

fn quick_add_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space)
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance must be the first plugin registered.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    if shortcut == &quick_add_shortcut() {
                        show_main_window(app);
                        let _ = tauri::Emitter::emit(app, "quick-add", ());
                    }
                })
                .build(),
        )
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            let db_path = app_dir.join("tungtung.sqlite");
            let conn = db::open(&db_path)?;

            let prefs = Arc::new(RuntimePrefs::load(&conn));
            let micro_break = Arc::new(MicroBreak::new());
            let scheduler = scheduler::spawn(
                app.handle().clone(),
                db_path.clone(),
                prefs.clone(),
                micro_break.clone(),
            );

            app.manage(AppState {
                db: Mutex::new(conn),
                db_path: db_path.to_string_lossy().to_string(),
                data_dir: app_dir.to_string_lossy().to_string(),
                scheduler,
                prefs: prefs.clone(),
                micro_break,
            });

            build_tray(app.handle())?;

            // Best-effort: a missing shortcut (e.g. another app owns it) must
            // not block startup.
            if let Err(err) = app.global_shortcut().register(quick_add_shortcut()) {
                eprintln!("global shortcut unavailable: {err}");
            }

            // The window is created hidden so `start minimized` never flashes.
            if !prefs.snapshot().start_minimized {
                show_main_window(app.handle());
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != "main" {
                    return;
                }
                // Close to tray: keep the app (and scheduler) alive so
                // background reminders keep firing. When the user turns this
                // off, closing the window quits the app.
                let close_to_tray = window
                    .app_handle()
                    .state::<AppState>()
                    .prefs
                    .snapshot()
                    .close_to_tray;
                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_reminders,
            commands::create_reminder,
            commands::update_reminder,
            commands::complete_reminder,
            commands::reopen_reminder,
            commands::skip_reminder,
            commands::snooze_reminder,
            commands::delete_reminder,
            commands::list_habits,
            commands::create_habit,
            commands::update_habit,
            commands::archive_habit,
            commands::toggle_habit,
            commands::start_focus_session,
            commands::end_focus_session,
            commands::list_focus_sessions,
            commands::micro_break_status,
            commands::micro_break_reset,
            commands::micro_break_trigger,
            commands::micro_break_snooze,
            commands::list_sounds,
            commands::add_sound,
            commands::import_sound,
            commands::delete_sound,
            commands::get_settings,
            commands::set_setting,
            commands::today_view,
            commands::history,
            commands::activity_counts,
            commands::diagnostics,
            commands::export_data,
            commands::import_data,
            commands::parse_export,
            commands::clear_all_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let new_reminder = MenuItem::with_id(app, "new-reminder", "New reminder", true, None::<&str>)?;
    let start_focus = MenuItem::with_id(app, "start-focus", "Start focus", true, None::<&str>)?;
    let pause_focus = MenuItem::with_id(app, "pause-focus", "Pause focus", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&open, &new_reminder, &start_focus, &pause_focus, &sep1, &settings, &quit],
    )?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("TungTung")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "new-reminder" => {
                show_main_window(app);
                let _ = tauri::Emitter::emit(app, "quick-add", ());
            }
            "start-focus" => {
                show_main_window(app);
                let _ = tauri::Emitter::emit(app, "focus-toggle", true);
            }
            "pause-focus" => {
                let _ = tauri::Emitter::emit(app, "focus-toggle", false);
            }
            "settings" => {
                show_main_window(app);
                let _ = tauri::Emitter::emit(app, "navigate", "settings");
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    // Prefer the icon file on disk so re-running `reicon.sh` shows up on the
    // next launch without a full clean rebuild; fall back to the icon
    // embedded at compile time (bundled release layout).
    let tray_icon = ["src-tauri/icons/32x32.png", "src-tauri/icons/icon.png"]
        .iter()
        .find_map(|p| tauri::image::Image::from_path(p).ok())
        .or_else(|| app.default_window_icon().cloned());

    if let Some(icon) = tray_icon {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}
