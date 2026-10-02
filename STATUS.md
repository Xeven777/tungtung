# TungTung — Project Status

A Linux desktop reminder + focus app. Tauri 2 (Rust backend) + React 19 / TypeScript (UI), SQLite for storage.

- Run dev: `bun run tauri dev`
- Build: `bun run build` (`tsc && vite build`)
- Rust tests: `cd src-tauri && cargo test --lib`

Last verified: `cargo build` ✅ · `cargo test --lib` 11/11 ✅ · `bun run build` ✅

---

## Architecture (as built)

- **React** renders the UI. **Rust** owns native behavior and time-sensitive logic. **SQLite** owns durable data.
- Reminder statuses: `scheduled | completed | snoozed | dismissed | skipped | overdue`.
- Recurrence is a simplified RRULE subset — `FREQ=HOURLY|DAILY|WEEKLY|MONTHLY`, optional `INTERVAL`, `BYDAY`. Recurring reminders are **materialized**: the row stores the next due date and is advanced when it fires.
- Scheduler sleeps until the next due time, capped at `MAX_SLEEP = 60s` (safety net for suspend / clock changes, not polling). DB writes call `SchedulerHandle::wake()` to fire immediately.

### Stack
- **Rust deps:** tauri 2 (`tray-icon`, `image-png`, `protocol-asset`), plugins — opener, dialog, global-shortcut, autostart, single-instance, store; notify-rust 4 (native notifications, no permission handshake on Linux); rusqlite 0.32 (bundled), chrono, uuid, thiserror, serde/serde_json.
- **Frontend deps:** react 19, zustand, @phosphor-icons/react, radix-ui, shadcn 4.x, tailwindcss + @tailwindcss/vite 4.3.3, tw-animate-css, @fontsource-variable/nunito.
- Package name `tungtung` (lib `tungtung_lib`); product name **TungTung**, identifier `com.anish.tungtung`.

---

## What's done

### Rust backend (`src-tauri/src/`)
- `models.rs` — serde camelCase models: `Reminder`, `Habit`, `HabitWithStats` (incl. `week_dates`/`week_completed`), `FocusSession`, `Sound`, `SettingEntry`, `Diagnostics`, `TodayView`, `ExportBundle`, `HistoryEntry`.
- `error.rs` — `AppError` + `AppResult`.
- `db.rs` — WAL + foreign keys, forward-only migrations via `PRAGMA user_version`; **v1** full schema + indexes, **v2** `habit_reminder_log`. Seeds 6 bundled sounds. All CRUD, `today_view`, `habits_with_stats` (streaks, Mon–Sun week), `history`, `activity_counts`.
- `recurrence.rs` — `RecurrenceRule` parse / `next_after` / `advance` / `next_due()` (+ 5 unit tests).
- `prefs.rs` — `RuntimePrefs`: `load`, `snapshot`, `apply(key, value)`, `in_quiet_hours()` (handles wrap past midnight), `notifications_suppressed()` (+ 3 unit tests).
- `scheduler.rs` — reminder firing loop, habit reminder loop (respects `reminder_time`, dedups via `habit_reminder_log`, skips already-completed days), and the micro-break nudge. Emits `reminders-changed`, `reminder-fired`, `habit-reminder-fired`, `micro-break-due`.
- `microbreak.rs` — micro-break timer state (`MicroBreak`): in-memory next-break timestamp, `take_due` / `reset` / `fire_now` / `remaining` (+ 3 unit tests).
- `platform.rs` — Linux desktop / display / OS diagnostics.
- `commands.rs` — full command surface: reminder CRUD + complete/reopen/skip/snooze; habit CRUD + archive + toggle; focus sessions start/end/list; sounds list/add/`import_sound`/delete; get/set settings; `today_view`, `history`, `activity_counts`, `diagnostics`, `export_data`, `import_data`, `parse_export`.
- `lib.rs` — plugin wiring (single-instance first), tray menu (Open / New reminder / Start focus / Pause focus / Settings / Quit), close-to-tray, start-minimized, global shortcut `Ctrl+Shift+Space`.
- `tauri.conf.json` — window 900×680, min 720×520, `visible: false`, **`decorations: false`**, asset protocol scoped to `$APPDATA/**`, bundle targets `deb/appimage/rpm`.

### Frontend (`src/`)
- `store.ts` — zustand store: routing, settings, diagnostics, sounds, today, reminders, habits, history, activity, pomodoro state machine, quick-add, command palette, toasts, theming.
- `lib/` — `types.ts`, `api.ts`, `dates.ts`, `recurrence.ts`, `parse.ts` (deterministic natural-language parser), `sounds.ts` (WebAudio synthesis + `convertFileSrc` for imported files), `actions.ts`, `utils.ts`.
- `components/` — layout primitives, `Sidebar`, `TitleBar` (window chrome), `SettingRow` (`Row` / `SoundRow`), `ReminderRow`, `EmptyState`, `MicroBreakOverlay`, `QuickAdd`, `ReminderEditor`, `CommandPalette`.
- `components/ui/sonner.tsx` — the sonner `Toaster`, themed from our own settings rather than `next-themes`.
- `views/` — Today, Reminders, Focus, Breaks, Habits, History, Settings.
- `App.tsx` — initial load, system-theme watching, Tauri event listeners, keyboard shortcuts (`Ctrl+K`, `Ctrl+1..4`).
- UI primitives in `src/components/ui/`: button, card, badge, input, textarea, label, select, switch, dialog, separator, kbd, progress (all Radix + Phosphor, `radix-nova` style).

### Features
| Area | Notes |
| --- | --- |
| Reminders | Create / edit / delete, due dates, repeat (hourly → monthly, interval, weekdays), priority, tags, statuses, snooze, skip, complete / reopen |
| Quick add | `Ctrl+K`-driven natural language: "at 8pm", "tomorrow", "in 2 hours", "every weekday", "next friday" |
| Today | Overdue + due-today roll-up at a glance |
| Focus (Pomodoro) | Configurable focus / short / long break, sessions before long break, auto-start toggles, live timer |
| Habits | Daily / weekday / custom schedules, streaks + best streak, Mon–Sun week grid |
| History | Completed items over time + activity counts graph |
| Settings | Theme (light/dark/system), sound per event + volume, quiet hours, snooze length, startup / tray behavior, diagnostics |
| Notifications | Native OS notifications + sonner toasts with Done / Snooze actions |
| Micro-breaks | Own **Breaks** screen: live countdown to the next break, Start now, work interval, break duration, overlay and sound. Firing takes over the whole screen with a countdown overlay and the spec's presets (eye 30s / stretch 2m / walk 5m / long 10m). Silent during quiet hours |
| Sounds | 6 synthesized built-ins + **custom sound import** (wav/ogg/mp3/flac/m4a/aac/opus) with preview |
| Window chrome | Frameless window with its own titlebar: minimise / maximise-restore / close plus drag and resize grips, so the system decoration theme (e.g. XFCE's) never clashes with the app |
| System tray | Open, new reminder, start/pause focus, settings, quit |
| Background scheduler | Fires reminders, habit nudges, and micro-breaks even while the window is hidden |
| Global shortcut | `Ctrl+Shift+Space` |
| Command palette | Fast navigation / actions |
| Data | Full export / import (JSON) and a parse-dry-run so bad files fail before touching the DB |

### Tier 0 — dead settings made real
1. `startMinimized` and `closeToTray` are honored by Rust (window shows on launch, close hides vs quits based on the live pref).
2. Notification gating: global `notificationsEnabled` + quiet hours suppress the OS notification at fire time; quiet hours also mute the app's own sound.
3. Habit reminders are actually scheduled from `habits.reminder_time` using `habitSound`, deduped per day.
4. Custom sound import end-to-end: dialog plugin → `import_sound` → copy into `<app_data>/sounds/` → asset protocol playback, plus Import / Test / Delete in Settings.
5. Keyboard-initiated overlays (QuickAdd, CommandPalette) have their animations disabled (strict interpretation).

---

## What's left

### Tier 1 — done ✅
**Micro-breaks** (§11) now run end to end:
- The timer lives in Rust (`microbreak.rs`) and is polled by the scheduler, so it fires while the window is hidden in the tray. The next wake-up accounts for it, so a break is not late by more than a second or two.
- Firing emits `micro-break-due` → OS notification ("Time for a break") plus, when `microBreakOverlay` is on, the countdown overlay. With the overlay off, the toast offers **Start break** / **Snooze** instead.
- Quiet hours silence the nudge completely, and the global notification toggle gates the OS popup.
- The overlay supports the spec presets, and closing it restarts the work interval.
- The overlay takes the window fullscreen for the duration (restoring the previous state afterwards), so a break covers the whole screen rather than just the app window. If the window is hidden in the tray there is nothing to cover, and the OS notification is the nudge instead.
- The **Breaks** screen (sidebar, `Ctrl+4`) shows a live **Next break** countdown and a **Start now** button, so the timer is visible rather than hidden behaviour.
- New settings: `microBreakOverlay`, `microBreakSound`. Existing: `microBreaksEnabled`, `microWorkMinutes`, `microBreakMinutes`.

### Tier 2 — design-engineering pass (done ✅)

The skill requires a Before/After table, so this is both the plan and the record:

| Before | After | Why |
| --- | --- | --- |
| No motion tokens; built-in `ease-out` is `cubic-bezier(0, 0, 0.2, 1)` | `@theme` tokens: `--ease-out: cubic-bezier(0.23, 1, 0.32, 1)`, `--ease-in-out`, `--ease-drawer`, plus `--default-transition-duration/-timing-function` | Built-in curves are too weak to read as intentional; overriding the defaults upgrades every unspecified `transition-*` for free |
| Button: `transition-all`, press feedback only `translate-y-px` | `transition-[color,background-color,border-color,box-shadow,transform,translate,scale] duration-150 ease-out` + `active:scale-97` | `all` animates properties that should change instantly; a press needs scale feedback |
| Badge: `transition-all` | `transition-colors duration-150 ease-out` | Badges are not pressable — colours only |
| Progress indicator: `transition-all` | `transition-transform duration-300 ease-out` | Only the transform actually changes |
| Reminder row actions: `transition-opacity` (default curve) | `transition-opacity duration-150 ease-out` | Hover-in should decelerate, not ease-in-out |
| Checkbox in reminder rows / Today: `transition-colors` (default curve) | `transition-colors ease-out` | Same reason, now the stronger curve |
| Dialog + backdrop: `duration-100` | `duration-200` | 100ms is below the 200–500ms range a modal should sit in |
| Select content: `duration-100` | `duration-150 ease-out` | Dropdowns want 150–250ms |
| No `prefers-reduced-motion` handling | Media block that pins `transition-property` to colour/opacity/shadow at 120ms and neutralises keyframe animations | Keeps the feedback that aids comprehension, removes the movement that causes motion sickness |
| Route transitions | Left with none | `Ctrl+1…5` view switches are keyboard-initiated and must stay instant |
| `@keyframes pop` on the checkbox | No change needed — it went with `global.css`; the checkbox uses a transition already | Keyframes restart from zero; transitions retarget |
| Toasts on tw-animate keyframes | No change needed — sonner owns toast motion and uses transitions | Toasts are added rapidly and must be interruptible |

Verified in the built CSS: the custom curve, both default-transition variables, and the reduced-motion block are all present.

### Tier 3 — hardening
- Go through the remaining `shadcn add` primitives for the same motion issues (anything regenerated loses these patches).
- Explicit sleep / resume detection in the scheduler (currently handled implicitly by the 60s sleep cap).
- TypeScript unit tests: quick-add parser, date helpers, recurrence labels.
- Native notification action buttons — Done / Snooze exist only on the in-app toast today.

### Tier 4 — packaging & release
- Flatpak build; verify `.deb` / AppImage.
- Autostart desktop entry.
- Upgrade-without-data-loss test against a populated DB.

### Known debts
- Habit reminder precision is bounded by the `MAX_SLEEP = 60s` cap — a 09:00 habit may fire around 09:00:5x. Micro-breaks already feed the wake-up computation, so the same fix applies to habits.
- The micro-break interval is anchored to the app session (in memory). It is not persisted across restarts, intentionally: the app cannot observe whether the user was actually at the keyboard, and the spec asks not to build invasive activity surveillance.
- Quiet hours never drop a reminder (by design); they only silence the OS popup and the sound.

---

## The frameless window (looking like the app, not like XFCE)

The window manager draws and owns native window decorations, so their buttons come from the *system* theme and cannot be styled by the app. Matching them therefore means drawing them: `decorations: false` plus `src/components/TitleBar.tsx`. Working from the official [Window Customization](https://v2.tauri.app/learn/window-customization/) guide:

- **Dropping `decorations` silently breaks resizing.** A frameless GTK window has no border to grab, so `resizable: true` becomes a no-op. This is a known Tauri issue on Linux. Eight invisible grips (4px edges, 10px corners) call `startResizeDragging` to restore it, and are hidden while maximised.
- **Dragging** uses `data-tauri-drag-region`, which Tauri wires up in JS — no per-mousemove React work, and double-click-to-maximise comes free with it. The attribute does not inherit, so both the bar and its filler carry it.
- **The maximise icon** tracks the real window state from the `resized` event, debounced to the trailing edge: that event fires on every frame of a resize drag, so checking on each one would round-trip to the backend hundreds of times per drag.
- **Close stays honest:** it calls `window.close()`, so it still goes through the Rust `CloseRequested` handler and respects **close to tray**.
- Permissions needed: `allow-minimize`, `allow-toggle-maximize`, `allow-close`, `allow-start-dragging`, `allow-start-resize-dragging`, `allow-is-maximized` (`core:window:default` already covers `allow-internal-toggle-maximize`, which is what powers the double-click).
- Trade-off accepted: the native right-click window menu is gone. Dragging, edge snapping and double-click maximise still work.

## Duplicated work — found and fixed

React StrictMode mounts every effect twice in development. Anything that registers or bootstraps asynchronously therefore runs twice unless it is guarded. All four instances found so far are fixed:

| Where | Symptom | Fix |
| --- | --- | --- |
| `App.tsx` Tauri event listeners | `listen()` resolves after the effect is torn down, so the first listener is never removed: **every event fires twice** (two toasts, two sounds, two `refreshAll` passes — the likely cause of the reported UI flicker) | `disposed` flag plus a `track()` helper that unlistens immediately if already disposed |
| `App.tsx` bootstrap | `loadSettings` / `refreshAll` / `loadDiagnostics` twice under StrictMode | Module-level `bootstrapped` flag — bootstrap is once per app lifetime |
| `store.refreshAll` | One reminder emits both `reminder-fired` and `reminders-changed`, and every action refreshes after writing — two full re-render passes back to back | Coalesced: an in-flight pass finishes, then a queued pass runs once. A request made mid-flight is never dropped |
| `Switch` (`ui/switch.tsx`) | Root animated colours with `transition-all` while the thumb animated its transform — two unsynchronised animations on a 32px control | `transition-colors duration-150` + `transition-transform duration-150 ease-out` |

Other effects audited and confirmed safe: `watchSystemTheme`, the QuickAdd/CommandPalette focus timers, the Reminders search debounce, `ReminderEditor`/`QuickAdd` state resets, the Breaks status poll, and the focus/micro tick intervals (both `startTicking` helpers clear the previous interval first).

## Gotchas worth remembering
1. **`decorations: false` on Linux costs you resizing.** Tauri's config still says `resizable: true` and nothing warns you — the window simply cannot be resized until you add `startResizeDragging` grips. Also note `ResizeDirection` is declared in `@tauri-apps/api/window` but *not* re-exported, so the union has to be mirrored locally.
2. **Async work in effects double-fires under StrictMode.** Four instances of this bug have already been found and fixed — see "Duplicated work". The pattern to watch for is a promise started in an effect that mutates long-lived state, because the effect *will* run twice on mount in development.
3. **Phosphor `*Icon` aliases are not in the published types.** `@phosphor-icons/react@2.1.10` exports both `X` and `XIcon` at runtime but `index.d.ts` only declares the bare names. Use bare names (`X`, `CaretDown`, `Check`, `CaretUp`). Re-running `shadcn add <component>` may reintroduce `*Icon` imports and break `tsc`.
4. **TypeScript 6 deprecates `baseUrl`** (`error TS5101`). Keep only `paths: { "@/*": ["./src/*"] }` — it resolves relative to `tsconfig.json`.
5. **`protocol-asset` cargo feature is required** when `assetProtocol.enable` is true, otherwise the build fails on an allowlist mismatch.
6. The shadcn CLI is only non-interactive with `-p <preset>`, and `init` needs valid path aliases.

## Notifications — current state & pixel-perfect option (decided: keep native for now)

### Short answer

Pixel-perfect is possible, but we stop using the OS notification daemon
entirely and draw the bubble ourselves as a **second Tauri window**. Then it
looks identical everywhere *our CSS runs* — but we inherit a new set of
per-OS window-manager quirks instead.

### The approach: a `notice` overlay window

One new borderless, transparent, always-on-top window (e.g. 380×140px)
parked bottom-right, rendered by our React + Tailwind + shadcn theme — same
accent, same dark mode, with real **Done / Snooze** buttons.

**Example prior art:** `tauri-notice-window` (Tauri v2 + React lib) does
exactly this — Zustand queue, one-at-a-time display window, auto-close
timer, cross-window sync via `localStorage`. Slack/Discord desktop toasts
are the commercial equivalent.

### What would need to change

**1. Rust — new window + manager (~biggest chunk)**
- New `WebviewWindowBuilder("notice", "/notice.html")` with:
  `decorations(false)`, `transparent(true)`, `always_on_top(true)`,
  `skip_taskbar(true)`, `focused(false)`, `visible(false)` initially,
  `resizable(false)`, `shadow(false)`.
- A small queue in Rust (or TS): `show_notice(reminder)` → set position →
  show → start 15s auto-dismiss timer → hide. Stacking (3 reminders = 3
  stacked windows vs. one-at-a-time queue).
- Positioning: `primary_monitor()` → `monitor.size()` × `scale_factor` →
  compute bottom-right with 16px margin. Multi-monitor: which screen has the
  focused window? No single "work area minus panel" API — the XFCE panel
  position eats into naive math.
- Click actions become real IPC: buttons emit `notice-done` /
  `notice-snooze` → call existing `complete_reminder` / `snooze_reminder`
  commands. The OS daemon's click-to-open-app behavior has to be rebuilt
  (focus main window on click).
- `capabilities/default.json`: add `windows: ["notice"]` +
  `core:window:allow-show/hide/set-position/set-focus/inner-position` etc.
- Optional: keep `notify.rs` as fallback (e.g. overlay fails) or delete it.

**2. Frontend — new route + component**
- `src/NoticeWindow.tsx` (or `/notice` route): card with `⏰ title`, notes,
  countdown bar, Done/Snooze buttons — reuses design tokens so it's
  genuinely pixel-identical on every OS.
- Main-window `App.tsx` listeners stay (sound + Sonner toast), or Sonner
  becomes redundant and gets removed for reminder events.
- `tauri.conf.json`: register the `notice` window or keep it fully dynamic
  from Rust.

### Compatibility — honest table

| OS / WM | Native (`notify-rust`, now) | Custom overlay window |
|---|---|---|
| XFCE + X11 + compositor on | ✅ themed, reliable | ✅ works, `alwaysOnTop`/`skipTaskbar` respected |
| XFCE, compositor **off** | ✅ works | ❌ transparency breaks (black box) — needs opaque fallback |
| GNOME Wayland | ✅ works | ⚠️ **no absolute positioning on Wayland** — the compositor places the window, bottom-right anchoring is unreliable; `alwaysOnTop` often ignored |
| KDE Wayland/X11 | ✅ works | ⚠️ mostly works, but KWin focus-stealing prevention can suppress show |
| Windows 10/11 | ✅ Action Center | ✅ works, but Focus Assist / fullscreen suppression must be handled; taskbar area math differs |
| macOS | ✅ Notification Center | ⚠️ needs `LSUIElement`-style accessory handling, notch/Dock area math; permission-less but intrusive |

So: **identical rendering, yes — identical placement/behavior, no.** The
failure modes just move from "theme differences" to "window-manager
differences," and Wayland is the real spoiler.

### Cost estimate

- Overlay window + positioning + queue + actions: ~300–500 lines Rust + 1
  new TSX view + capability entries.
- Edge cases that eat time: compositor-off fallback, Wayland positioning,
  multi-monitor, fullscreen-app suppression, Do-Not-Disturb respect,
  stacking, click-to-focus.
- We also permanently own maintenance the daemon used to do (timeouts,
  persistence, accessibility).

### Recommendation (not applied)

If XFCE is home base: keep `notify-rust` native (what we have) and
optionally polish the **in-app Sonner toast** to carry the branding — best
effort-to-reward. Go custom overlay only if clickable Done/Snooze
*outside the app window* is worth the Wayland caveats.
