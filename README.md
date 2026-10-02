<p align="center">
  <img src="./public/icon.png" width="128" height="128" alt="TungTung logo" />
</p>

<h1 align="center">TungTung 🍊⏰</h1>

<p align="center">
  <b>A lightweight reminder & focus companion for all platforms ✨</b><br/>
  Never miss a thing — reminders, habits, pomodoro & micro-breaks, all in one cozy little app.
</p>

<p align="center">

![Tauri](https://img.shields.io/badge/Tauri-2.0-orange?style=flat-square&logo=tauri)
![React](https://img.shields.io/badge/React-19-61dafb?style=flat-square&logo=react)
![TypeScript](https://img.shields.io/badge/TypeScript-6-3178c6?style=flat-square&logo=typescript)
![Rust](https://img.shields.io/badge/Rust-2021-orange?style=flat-square&logo=rust)
![SQLite](https://img.shields.io/badge/SQLite-bundled-003b57?style=flat-square&logo=sqlite)
![Linux](https://img.shields.io/badge/Linux-first-FCC624?style=flat-square&logo=linux)

</p>

---

## ✨ Why TungTung?

> Most reminder apps are either bloated Electron monsters 👹 or boring system utilities 🥱.
> **TungTung** is the sweet spot — native-speed Rust core 🦀, beautiful React UI ⚛️, tiny footprint, and zero cloud nonsense. **Your data stays yours** 🔒.

- 🔔 **Reminders that actually fire** — even when the window is hidden
- 🔁 **Smart recurrence** — hourly → monthly, intervals, weekdays
- ⚡ **Quick-add with natural language** — _"gym tomorrow at 8pm"_ 🧠
- 🍅 **Pomodoro focus timer** — focus / short / long breaks
- 🌱 **Habits + streaks** — Mon–Sun week grid, gentle nudges
- ☕ **Micro-breaks** — eye / stretch / walk reminders with fullscreen overlay
- 🎨 **One brand identity** — warm orange 🍊 + rounded Nunito type
- 🌗 **Light / dark** — follows your desktop
- 🔊 **Custom sounds** — import your own wav/ogg/mp3
- 📦 **Full backup** — export / import everything as JSON

---

## 🚀 Quick start

### Prerequisites 🧰

- [Bun](https://bun.sh/) 🍞
- [Rust](https://rustup.rs/) 🦀
- Tauri Linux deps ([guide](https://v2.tauri.app/start/prerequisites/)) 🐧

```bash
# 1️⃣ Clone it
git clone https://github.com/xeven777/tungtung.git
cd tungtung

# 2️⃣ Install frontend deps
bun install

# 3️⃣ Run in dev mode (hot-reload ⚡)
bun run tauri dev
```

### 🏗️ Build it

```bash
# Web frontend only
bun run build

# Full desktop app (.deb / .rpm) 📦
bun run tauri build
```

### ✅ Tests

```bash
# Rust unit tests (recurrence, prefs, micro-breaks)
cd src-tauri && cargo test --lib

# Typecheck + frontend build
bun run build
```

---

## ⌨️ Shortcuts

| Keys | Action |
|------|--------|
| `Ctrl + Shift + Space` 🌍 | Quick-add from anywhere |
| `Ctrl + K` 🔍 | Command palette |
| `Ctrl + 1…4` 🧭 | Today / Reminders / Focus / Habits |
| `Esc` 🚪 | Close dialog |

---

## 🗂️ Project structure

```
tungtung/
├── 🎨 public/            # icon.png, favicons
├── ⚛️ src/               # React + TypeScript UI
│   ├── components/       # Sidebar, TitleBar, dialogs, ui/*
│   ├── views/            # Today, Reminders, Focus, Breaks, Habits, History, Settings
│   ├── lib/              # api, dates, natural-language parser, sounds
│   └── store.ts          # zustand store 🐻
├── 🦀 src-tauri/         # Rust backend
│   ├── src/              # db, scheduler, notify, commands, prefs…
│   ├── icons/            # auto-generated app icons 🖼️
│   └── tauri.conf.json   # productName: TungTung 🍊
├── 📜 scripts/
│   └── reicon.sh         # regenerate ALL icons from one PNG ✨
└── 📊 STATUS.md          # deep-dive architecture notes
```

### 🧠 How it works

- **React** renders the UI · **Rust** owns time-sensitive logic · **SQLite** owns durable data
- Scheduler **sleeps until the next due time** 😴 (capped at 60s as a safety net — not polling)
- Recurring reminders are **materialized** — the row stores the next due date and advances when it fires
- Notifications go through native `org.freedesktop.Notifications` — themed by *your* desktop (XFCE, GNOME, KDE all work 🎨)
- Frameless window (`decorations: false`) with its own titlebar, so it looks like TungTung everywhere 😎
- Lives in the **system tray** — close the window, reminders keep firing 🔥

---

## 🎨 Branding

- **Name:** TungTung 🍊
- **Primary:** Orange `oklch(0.646 0.222 41.116)` 🟠
- **Font:** [Nunito Variable](https://fonts.google.com/specimen/Nunito) — rounded, friendly, chunky headings (800)
- **Icon:** one source PNG → everything else is generated:

```bash
./scripts/reicon.sh path/to/new-icon.png
```

> Regenerates `icon.png`, `public/` favicons, and all `src-tauri/icons/` via `tauri icon`. Add `--no-tauri` to skip the Tauri step.

---

## 🤝 Contributing

Contributions are **very welcome**! 💛 Here's the vibe:

1. 🍴 Fork it
2. 🌱 Create a branch — `git checkout -b feat/my-cool-thing`
3. ✨ Make it awesome (please keep the orange 🍊)
4. ✅ Run `bun run build` + `cargo test --lib`
5. 📬 Open a PR!

Ideas 💡: more screenshots 📸, Wayland tray polish, new built-in sounds 🔊, translations 🌍, Windows/macOS builds 🪟🍎.

---

<p align="center">
  Made with 🦀 + ⚛️ + 🍊 on Linux 🐧 by <a href="https://anish7.me">Anish</a><br/>
  <b>TungTung</b> — <i>tung-tung-tung-Sahur, time to do the thing!</i> ⏰✨
</p>

