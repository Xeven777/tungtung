//! Backend audio playback for notifications that fire while the UI is not ready.
//!
//! The main webview is created lazily and only hidden to the tray, so it may
//! not exist yet when a notification fires (or its listeners may not be
//! registered). The frontend normally owns sound playback, but until it
//! signals readiness via `ui_ready` this module lets the scheduler play the
//! user's chosen sound itself.
//!
//! A sound id resolves to one of three sources:
//! * `builtin-*`  — synthesized here as a small WAV (mirrors `src/lib/sounds.ts`).
//! * `bundled-*`  — a clip shipped as an app resource (`resources/sounds/*.opus`).
//! * anything else — a user-imported file whose path lives in the `sounds` table.
//!
//! Playback is delegated to the first available system player (`pw-play`,
//! `paplay`, `ffplay`, `play`, `cvlc`, `aplay`) so we neither link an audio
//! stack nor decode in-process. Failures are logged, never fatal.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use chrono::{DateTime, Local};
use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use crate::prefs::RuntimePrefs;

/// Which settings key drives a given notification's sound.
#[derive(Clone, Copy)]
pub enum SoundSlot {
    Reminder,
    Habit,
    MicroBreak,
    /// Chime for a naturally completed focus phase.
    Pomodoro,
}

/// Play the configured sound for `slot`, but only until the UI is ready: once
/// the frontend has registered its listeners (see `pending::is_ready`) it
/// plays sounds itself (with its own volume + preview state), so playing here
/// too would double up.
pub fn play_if_hidden(
    app: &AppHandle,
    conn: &Connection,
    prefs: &Arc<RuntimePrefs>,
    slot: SoundSlot,
    now: DateTime<Local>,
) {
    if crate::pending::is_ready() {
        return;
    }

    let prefs = prefs.snapshot();
    if !prefs.sound_enabled {
        return;
    }
    if prefs.in_quiet_hours(now) {
        return;
    }

    let id = match slot {
        SoundSlot::Reminder => prefs.reminder_sound.as_str(),
        SoundSlot::Habit => prefs.habit_sound.as_str(),
        SoundSlot::MicroBreak => prefs.micro_break_sound.as_str(),
        SoundSlot::Pomodoro => prefs.pomodoro_sound.as_str(),
    };
    if id.is_empty() || id == "none" {
        return;
    }

    match resolve(app, conn, id) {
        Some(file) => spawn_player(&file, prefs.volume),
        None => eprintln!("sound: could not resolve a file for id '{id}'"),
    }
}

/// Map a stored sound id to an audio file on disk.
fn resolve(app: &AppHandle, conn: &Connection, id: &str) -> Option<PathBuf> {
    if let Some(name) = id.strip_prefix("builtin-") {
        return synth_builtin(name);
    }
    if let Some(name) = id.strip_prefix("bundled-") {
        return resolve_bundled(app, name);
    }
    // Imported sound: use the path recorded in the database.
    crate::db::sound_file_path(conn, id)
        .ok()
        .flatten()
        .map(PathBuf::from)
        .filter(|p| p.is_file())
}

/// Bundled meme clips ship as app resources. The id (`bundled-anime-ahh`) maps
/// 1:1 to the file name (`anime-ahh.opus`). Extra candidates cover dev layouts.
fn resolve_bundled(app: &AppHandle, name: &str) -> Option<PathBuf> {
    let file = format!("{name}.opus");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = app.path().resource_dir() {
        candidates.push(dir.join("resources/sounds").join(&file));
        candidates.push(dir.join("sounds").join(&file));
    }
    candidates.push(PathBuf::from("src-tauri/resources/sounds").join(&file));
    candidates.push(PathBuf::from("src/assets/sounds").join(&file));
    candidates.into_iter().find(|p| p.is_file())
}

// ---------------------------------------------------------------------------
// Built-in tone synthesis (mirrors BUILTIN_SOUNDS in src/lib/sounds.ts)
// ---------------------------------------------------------------------------

const SAMPLE_RATE: u32 = 44_100;

#[derive(Clone, Copy)]
enum Wave {
    Sine,
    Triangle,
    Square,
}

struct Tone {
    freq: f32,
    start: f32,
    duration: f32,
    gain: f32,
    wave: Wave,
}

fn tone_spec(name: &str) -> Option<Vec<Tone>> {
    let t = |freq, start, duration, gain, wave| Tone { freq, start, duration, gain, wave };
    Some(match name {
        "soft" => vec![t(660.0, 0.0, 0.45, 0.5, Wave::Sine)],
        "classic" => vec![
            t(880.0, 0.0, 0.28, 0.6, Wave::Sine),
            t(660.0, 0.16, 0.42, 0.6, Wave::Sine),
        ],
        "bell" => vec![
            t(1046.0, 0.0, 1.1, 0.55, Wave::Sine),
            t(2093.0, 0.0, 0.6, 0.18, Wave::Sine),
            t(1568.0, 0.02, 0.5, 0.14, Wave::Sine),
        ],
        "wood" => vec![
            t(240.0, 0.0, 0.14, 0.7, Wave::Triangle),
            t(180.0, 0.06, 0.1, 0.4, Wave::Triangle),
        ],
        "digital" => vec![
            t(1200.0, 0.0, 0.09, 0.4, Wave::Square),
            t(1200.0, 0.14, 0.09, 0.4, Wave::Square),
        ],
        "minimal" => vec![t(520.0, 0.0, 0.2, 0.45, Wave::Sine)],
        _ => return None,
    })
}

/// Render (and cache) a built-in tone as a mono 16-bit WAV.
fn synth_builtin(name: &str) -> Option<PathBuf> {
    let tones = tone_spec(name)?;
    let dir = std::env::temp_dir().join("tungtung-sounds");
    let path = dir.join(format!("builtin-{name}.wav"));
    if path.is_file() {
        return Some(path);
    }
    std::fs::create_dir_all(&dir).ok()?;
    std::fs::write(&path, render_wav(&tones)).ok()?;
    Some(path)
}

fn render_wav(tones: &[Tone]) -> Vec<u8> {
    let total = tones
        .iter()
        .map(|t| t.start + t.duration)
        .fold(0.0f32, f32::max)
        + 0.05;
    let samples = (total * SAMPLE_RATE as f32).ceil() as usize;
    let mut mix = vec![0.0f32; samples];

    for tone in tones {
        let start = (tone.start * SAMPLE_RATE as f32) as usize;
        let len = (tone.duration * SAMPLE_RATE as f32) as usize;
        for i in 0..len {
            let idx = start + i;
            if idx >= mix.len() {
                break;
            }
            let t = i as f32 / SAMPLE_RATE as f32;
            let phase = std::f32::consts::TAU * tone.freq * t;
            let raw = match tone.wave {
                Wave::Sine => phase.sin(),
                // asin(sin(x)) is an exact triangle wave.
                Wave::Triangle => phase.sin().asin() * (2.0 / std::f32::consts::PI),
                Wave::Square => {
                    if phase.sin() >= 0.0 {
                        1.0
                    } else {
                        -1.0
                    }
                }
            };
            mix[idx] += raw * envelope(t, tone.duration) * tone.gain;
        }
    }

    let mut out = Vec::with_capacity(44 + samples * 2);
    write_wav_header(&mut out, samples);
    for sample in mix {
        let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

/// Exponential attack/decay matching WebAudio's `exponentialRampToValueAtTime`
/// usage in `sounds.ts` (0.0001 -> full over 12ms, then full -> 0.0001).
fn envelope(t: f32, duration: f32) -> f32 {
    const FLOOR: f32 = 0.0001;
    const ATTACK: f32 = 0.012;
    if t < 0.0 {
        return 0.0;
    }
    if t < ATTACK {
        return FLOOR * (1.0 / FLOOR).powf(t / ATTACK);
    }
    if t < duration {
        let k = (t - ATTACK) / (duration - ATTACK).max(0.0001);
        return (FLOOR).powf(k);
    }
    0.0
}

fn write_wav_header(out: &mut Vec<u8>, samples: usize) {
    let data_len = (samples * 2) as u32;
    let byte_rate = SAMPLE_RATE * 2; // mono * 16-bit
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
}


// ---------------------------------------------------------------------------
// Playback via an external player
// ---------------------------------------------------------------------------

/// Spawn the first installed system player that can handle `file`. The child is
/// reaped on a detached thread so it never becomes a zombie.
fn spawn_player(file: &Path, volume: u8) {
    let vol = volume.min(100) as f32 / 100.0;
    let path = file.to_string_lossy().to_string();

    // (program, args) — order is preference: PipeWire -> PulseAudio -> ffmpeg ->
    // SoX -> VLC -> ALSA. Any of them accepts WAV/ogg/opus/mp3/flac.
    let candidates: Vec<(&str, Vec<String>)> = vec![
        ("pw-play", vec![format!("--volume={vol:.2}"), path.clone()]),
        (
            "paplay",
            vec![format!("--volume={}", (vol * 65536.0) as u32), path.clone()],
        ),
        (
            "ffplay",
            vec![
                "-nodisp".into(),
                "-autoexit".into(),
                "-loglevel".into(),
                "quiet".into(),
                "-volume".into(),
                format!("{}", (vol * 100.0).round() as u32),
                path.clone(),
            ],
        ),
        (
            "play",
            vec!["-q".into(), "-v".into(), format!("{vol:.2}"), path.clone()],
        ),
        (
            "cvlc",
            vec![
                "--play-and-exit".into(),
                "--intf".into(),
                "dummy".into(),
                path.clone(),
            ],
        ),
        ("aplay", vec!["-q".into(), path.clone()]),
    ];

    for (program, args) in candidates {
        match Command::new(program)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                return;
            }
            // Not installed — try the next player.
            Err(_) => continue,
        }
    }

    eprintln!(
        "sound: no system audio player found (tried pw-play, paplay, ffplay, play, cvlc, aplay)"
    );
}

/// Names, in preference order, of the system players `spawn_player` can use.
pub const PLAYER_CANDIDATES: &[&str] =
    &["pw-play", "paplay", "ffplay", "play", "cvlc", "aplay"];

/// First installed player name, if any. Used by diagnostics so the Settings
/// screen can tell the user what to install when nothing is found. Probing
/// spawns processes, so the result is computed once and cached.
pub fn available_player() -> Option<&'static str> {
    static CACHE: std::sync::OnceLock<Option<&'static str>> = std::sync::OnceLock::new();
    *CACHE.get_or_init(probe_players)
}

/// Probe `PATH` for the first known player. A bare `--version` spawn only
/// tells us the binary starts; the real play still falls through the list.
fn probe_players() -> Option<&'static str> {
    PLAYER_CANDIDATES.iter().copied().find(|program| {
        std::process::Command::new(program)
            .arg("--version")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map(|mut child| {
                let _ = child.wait();
                true
            })
            .unwrap_or(false)
    })
}


