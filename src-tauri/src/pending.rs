//! Deferred UI actions for the lazily-created main window.
//!
//! The main webview is created on demand (lazy start) and hidden to the tray,
//! so a tray click, global shortcut, or scheduled event may arrive while no
//! webview — and therefore no React listener — exists yet. Instead of emitting
//! into the void, actions are stashed here and drained by the frontend once it
//! has registered its event listeners and calls the `ui_ready` command.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// A user intent that must be delivered to the UI exactly once.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PendingAction {
    /// Open the quick-add dialog (tray "New reminder", global shortcut).
    QuickAdd,
    /// Navigate to a route (tray "Settings").
    Navigate { route: String },
    /// A micro break became due while the webview did not exist; carries the
    /// backend-computed break length so a recreated window can open the overlay
    /// itself on mount.
    MicroBreakDue { break_seconds: i64 },
}

/// The action the frontend has to apply on its next `ui_ready` call, if any.
static PENDING: Mutex<Option<PendingAction>> = Mutex::new(None);
/// Whether a webview currently exists whose React listeners are registered.
static READY: AtomicBool = AtomicBool::new(false);

/// Mark the UI ready after its listeners are registered (or `false` when the
/// window no longer has a mounted UI — e.g. it was destroyed).
pub fn set_ready(ready: bool) {
    READY.store(ready, Ordering::SeqCst);
}

pub fn is_ready() -> bool {
    READY.load(Ordering::SeqCst)
}

/// Take the stashed action, if any. Called by the `ui_ready` command.
pub fn take() -> Option<PendingAction> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).take()
}

fn stash(action: PendingAction) {
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(action);
}

/// Deliver `action` to the UI: emit it immediately when the UI is mounted,
/// otherwise buffer it until the next `ui_ready`.
pub fn dispatch(app: &AppHandle, action: PendingAction) {
    if is_ready() {
        emit(app, &action);
    } else {
        stash(action);
    }
}

fn emit(app: &AppHandle, action: &PendingAction) {
    match action {
        PendingAction::QuickAdd => {
            let _ = app.emit("quick-add", ());
        }
        PendingAction::Navigate { route } => {
            let _ = app.emit("navigate", route.clone());
        }
        PendingAction::MicroBreakDue { break_seconds } => {
            let _ = app.emit(
                "micro-break-due",
                serde_json::json!({ "breakSeconds": break_seconds }),
            );
        }
    }
}
