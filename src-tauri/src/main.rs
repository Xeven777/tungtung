// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Memory footprint tuning for the WebKitGTK stack. This must be set before
    // GTK / WebKit initialize (and before any thread is spawned) so the child
    // WebProcess/NetworkProcess inherit it. See STATUS.md for the rationale.
    //
    // - MALLOC_ARENA_MAX: the web process runs ~47 threads; glibc otherwise
    //   creates a per-thread malloc arena whose fragmentation inflates RSS.
    //
    // NOTE: do NOT disable WebKit compositing here. The window is transparent
    // with rounded corners, and transparency requires the accelerated
    // compositing path — `WEBKIT_DISABLE_COMPOSITING_MODE` /
    // `WEBKIT_DISABLE_DMABUF_RENDERER` make the window render garbled and
    // unclickable on this setup. They can still be tested per-run by exporting
    // them before launching (they are honoured because we only set defaults).
    set_if_unset("MALLOC_ARENA_MAX", "2");

    tungtung_lib::run()
}

/// Set `key` to `value` unless the user already provided one, so power users can
/// still override the defaults from their environment / .desktop launcher.
fn set_if_unset(key: &str, value: &str) {
    if std::env::var_os(key).is_none() {
        std::env::set_var(key, value);
    }
}
