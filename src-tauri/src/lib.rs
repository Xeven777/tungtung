mod commands;
mod db;
mod error;
mod microbreak;
mod models;
mod notify;
mod pending;
mod platform;
mod prefs;
mod recurrence;
mod scheduler;
mod sound;

use std::sync::{Arc, Mutex};

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use commands::AppState;
use microbreak::MicroBreak;
use pending::PendingAction;
use prefs::RuntimePrefs;

fn quick_add_shortcut() -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space)
}

/// Build the single main webview window with the app's custom chrome.
///
/// The window is created lazily — on first open, and again after it has been
/// destroyed when the app was hidden to the tray — rather than at startup. The
/// WebKit renderer is the bulk of the app's memory footprint, so not creating
/// it while the app lives in the tray keeps the background cost tiny.
fn create_main_window(app: &tauri::AppHandle) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("TungTung")
        .inner_size(900.0, 680.0)
        .min_inner_size(720.0, 520.0)
        .resizable(true)
        .center()
        .decorations(false)
        .transparent(true)
        .visible(false)
        .build()
}

/// Show (creating if necessary) the main window and focus it.
fn show_main_window(app: &tauri::AppHandle) {
    let window = match app.get_webview_window("main") {
        Some(window) => window,
        None => match create_main_window(app) {
            Ok(window) => window,
            Err(err) => {
                eprintln!("failed to create main window: {err}");
                return;
            }
        },
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

/// Show the window and deliver a user action to it. If the webview was just
/// (re)created, `dispatch` buffers the action until the UI calls `ui_ready`.
fn open_action(app: &tauri::AppHandle, action: PendingAction) {
    show_main_window(app);
    pending::dispatch(app, action);
}

/// Best-effort recreation of the main window for an event that must surface.
pub(crate) fn pending_window(app: &tauri::AppHandle, action: PendingAction) -> bool {
    if app.get_webview_window("main").is_some() {
        return false;
    }
    if create_main_window(app).is_err() {
        return false;
    }
    pending::dispatch(app, action);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    true
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
                        open_action(app, PendingAction::QuickAdd);
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
            if window.label() != "main" {
                return;
            }
            match event {
                WindowEvent::CloseRequested { api, .. } => {
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
                        // Destroy rather than hide: this tears down the WebKit
                        // renderer (the bulk of the app's memory) while the tray
                        // icon and scheduler keep the process alive. The window
                        // is rebuilt on demand by `show_main_window`.
                        pending::set_ready(false);
                        let _ = window.destroy();
                    }
                }
                WindowEvent::Destroyed => pending::set_ready(false),
                _ => {}
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
            commands::ui_ready,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // With close-to-tray the main window is *destroyed* (not hidden) so
            // the WebKit renderer is freed, which means destroying it trips the
            // framework's last-window shutdown. Keep the tray + scheduler alive
            // unless an explicit exit was requested (tray "Quit" calls
            // `app.exit(0)`, which arrives here with `code = Some(..)`).
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    let close_to_tray = app
                        .try_state::<AppState>()
                        .map(|state| state.prefs.snapshot().close_to_tray)
                        .unwrap_or(false);
                    if close_to_tray {
                        api.prevent_exit();
                    }
                }
            }
        });
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
            "new-reminder" => open_action(app, PendingAction::QuickAdd),
            "start-focus" => open_action(app, PendingAction::FocusToggle { running: true }),
            "pause-focus" => {
                // Pausing only makes sense while the UI is open; if it is not,
                // there is no running session to pause.
                let _ = tauri::Emitter::emit(app, "focus-toggle", false);
            }
            "settings" => open_action(
                app,
                PendingAction::Navigate {
                    route: "settings".into(),
                },
            ),
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
