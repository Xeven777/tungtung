/**
 * Notification sounds are synthesized with the Web Audio API so the app ships
 * no binary audio assets and works fully offline. Custom imported files are
 * played through an <audio> element, resolved via Tauri's asset protocol.
 */

import animeAhhUrl from "@/assets/sounds/anime-ahh.opus";
import catLaughUrl from "@/assets/sounds/cat-laugh.opus";
import faaahUrl from "@/assets/sounds/faaah.opus";
import screeamUrl from "@/assets/sounds/screeam.opus";
import sirenUrl from "@/assets/sounds/siren.opus";
import tungtungUrl from "@/assets/sounds/tungtung.opus";
import { convertFileSrc } from "@tauri-apps/api/core";

/**
 * Clips shipped with the app. Vite emits them into `dist/assets/` with
 * content hashes and gives us the final URLs here, so playback works in
 * both `tauri dev` and bundled builds (no hardcoded `/sounds/...` paths
 * that can 404 after a rebuild).
 */
export const BUNDLED_CLIP_URLS: Record<string, string> = {
  "bundled-tungtung": tungtungUrl,
  "bundled-siren": sirenUrl,
  "bundled-screeam": screeamUrl,
  "bundled-anime-ahh": animeAhhUrl,
  "bundled-faaah": faaahUrl,
  "bundled-cat-laugh": catLaughUrl,
};

export interface BuiltinSound {
  id: string;
  name: string;
  play: (ctx: AudioContext, volume: number) => void;
}

let ctx: AudioContext | null = null;

function audioContext(): AudioContext | null {
  try {
    if (!ctx) {
      const Ctor = window.AudioContext ?? (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      ctx = new Ctor();
    }
    if (ctx.state === "suspended") void ctx.resume();
    return ctx;
  } catch {
    return null;
  }
}

interface ToneOptions {
  freq: number;
  start: number;
  duration: number;
  gain?: number;
  type?: OscillatorType;
}

function tone(c: AudioContext, opts: ToneOptions) {
  const osc = c.createOscillator();
  const gain = c.createGain();
  const peak = opts.gain ?? 0.6;
  osc.type = opts.type ?? "sine";
  osc.frequency.value = opts.freq;

  const t0 = c.currentTime + opts.start;
  gain.gain.setValueAtTime(0.0001, t0);
  gain.gain.exponentialRampToValueAtTime(peak, t0 + 0.012);
  gain.gain.exponentialRampToValueAtTime(0.0001, t0 + opts.duration);

  osc.connect(gain).connect(c.destination);
  osc.start(t0);
  osc.stop(t0 + opts.duration + 0.02);
}

export const BUILTIN_SOUNDS: BuiltinSound[] = [
  {
    id: "builtin-soft",
    name: "Soft",
    play: (c) => tone(c, { freq: 660, start: 0, duration: 0.45, gain: 0.5 }),
  },
  {
    id: "builtin-classic",
    name: "Classic",
    play: (c) => {
      tone(c, { freq: 880, start: 0, duration: 0.28 });
      tone(c, { freq: 660, start: 0.16, duration: 0.42 });
    },
  },
  {
    id: "builtin-bell",
    name: "Bell",
    play: (c) => {
      tone(c, { freq: 1046, start: 0, duration: 1.1, gain: 0.55 });
      tone(c, { freq: 2093, start: 0, duration: 0.6, gain: 0.18 });
      tone(c, { freq: 1568, start: 0.02, duration: 0.5, gain: 0.14 });
    },
  },
  {
    id: "builtin-wood",
    name: "Wood",
    play: (c) => {
      tone(c, { freq: 240, start: 0, duration: 0.14, gain: 0.7, type: "triangle" });
      tone(c, { freq: 180, start: 0.06, duration: 0.1, gain: 0.4, type: "triangle" });
    },
  },
  {
    id: "builtin-digital",
    name: "Digital",
    play: (c) => {
      tone(c, { freq: 1200, start: 0, duration: 0.09, gain: 0.4, type: "square" });
      tone(c, { freq: 1200, start: 0.14, duration: 0.09, gain: 0.4, type: "square" });
    },
  },
  {
    id: "builtin-minimal",
    name: "Minimal",
    play: (c) => tone(c, { freq: 520, start: 0, duration: 0.2, gain: 0.45 }),
  },
];

let currentSource: AudioBufferSourceNode | null = null;
let playToken = 0;

/** Play a bundled sound by id, or a custom file path when provided. */
export function playSound(id: string | null | undefined, opts: { volume?: number; filePath?: string | null } = {}) {
  if (!id || id === "none") return;
  const volume = Math.min(1, Math.max(0, (opts.volume ?? 70) / 100));

  // Shipped clips resolve through Vite's emitted asset URLs — already webview
  // URLs, so they need no protocol conversion.
  const bundledUrl = BUNDLED_CLIP_URLS[id];
  if (bundledUrl) {
    playFile(bundledUrl, volume);
    return;
  }

  // Imported files live on disk and must go through the asset protocol. NOTE:
  // absolute POSIX paths start with "/", so a root-relative check would swallow
  // them and hand `new Audio()` a filesystem path it cannot fetch.
  if (opts.filePath) {
    playFile(toAssetUrl(opts.filePath), volume);
    return;
  }

  const builtin = BUILTIN_SOUNDS.find((s) => s.id === id);
  if (!builtin) return;
  const c = audioContext();
  if (!c) return;
  builtin.play(c, volume);
}

function toAssetUrl(filePath: string): string {
  try {
    return convertFileSrc(filePath);
  } catch {
    return filePath;
  }
}

/**
 * Files are fetched and decoded rather than streamed into an <audio> element.
 * Tauri's `tauri://localhost` protocol answers Range-less 200s, which WebKit's
 * media stack refuses to play, and `get_asset` answers any unknown path with
 * the app shell (`200 text/html`) instead of a 404 — so a bad URL failed
 * silently. Decoding bytes we already hold sidesteps both.
 */
function playFile(url: string, volume: number) {
  const c = audioContext();
  if (!c) return;

  const token = ++playToken;
  if (currentSource) currentSource.stop();
  currentSource = null;

  void fetch(url)
    .then((res) => {
      if (!res.ok) throw new Error(`HTTP ${res.status} for ${url}`);
      return res.arrayBuffer();
    })
    .then((bytes) => c.decodeAudioData(bytes))
    .then((buffer) => {
      // A newer sound started while this one was decoding.
      if (token !== playToken) return;
      const gain = c.createGain();
      gain.gain.value = volume;
      const source = c.createBufferSource();
      source.buffer = buffer;
      source.connect(gain).connect(c.destination);
      source.start();
      currentSource = source;
    })
    .catch((err) => {
      console.warn(`[sounds] could not play ${url}`, err);
    });
}

/** Short preview used in settings. */
export function previewSound(id: string, volume = 70, filePath?: string | null) {
  playSound(id, { volume, filePath });
}
