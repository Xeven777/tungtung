import { useEffect, useState } from "react";
import {
  DownloadSimple,
  Info,
  Trash,
  UploadSimple,
} from "@phosphor-icons/react";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { open } from "@tauri-apps/plugin-dialog";
import { Page, PageHeader, Section } from "@/components/layout";
import { Row, SoundRow } from "@/components/SettingRow";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Kbd } from "@/components/ui/kbd";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { api } from "@/lib/api";
import { previewSound } from "@/lib/sounds";
import { cn } from "@/lib/utils";
import { useStore } from "@/store";

export function Settings() {
  const settings = useStore((s) => s.settings);
  const update = useStore((s) => s.updateSetting);
  const sounds = useStore((s) => s.sounds);
  const diagnostics = useStore((s) => s.diagnostics);
  const loadDiagnostics = useStore((s) => s.loadDiagnostics);
  const pushToast = useStore((s) => s.pushToast);
  const [importing, setImporting] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);
  const customSounds = sounds.filter((sound) => !sound.bundled);

  useEffect(() => {
    void loadDiagnostics();
  }, [loadDiagnostics]);

  useEffect(() => {
    isEnabled()
      .then((enabled) => update("launchAtStartup", String(enabled)))
      .catch(() => undefined);
  }, [update]);

  const set = (key: string, value: string) => void update(key, value);

  async function exportData() {
    const bundle = await api.exportData();
    const blob = new Blob([JSON.stringify(bundle, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `tungtung-backup-${new Date().toISOString().slice(0, 10)}.json`;
    anchor.click();
    URL.revokeObjectURL(url);
    pushToast({
      title: "Exported",
      body: `${bundle.reminders.length} reminders`,
    });
  }

  async function importData(file: File) {
    setImporting(true);
    try {
      const bundle = await api.parseExport(await file.text());
      await api.importData(bundle);
      await useStore.getState().refreshAll();
      pushToast({
        title: "Import complete",
        body: `${bundle.reminders.length} reminders restored`,
      });
    } catch (err) {
      pushToast({ title: "Import failed", body: String(err) });
    } finally {
      setImporting(false);
    }
  }

  async function importSoundFile() {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "Audio",
            extensions: ["wav", "ogg", "mp3", "flac", "m4a", "aac", "opus"],
          },
        ],
      });
      if (!selected || Array.isArray(selected)) return;

      const fileName = selected.split(/[\\/]/).pop() ?? "Custom sound";
      const label = fileName.replace(/\.[^.]+$/, "");

      await api.importSound(label, selected);
      await useStore.getState().loadSounds();
      pushToast({ title: "Sound imported", body: label });
    } catch (err) {
      pushToast({ title: "Import failed", body: String(err) });
    }
  }

  async function removeSound(id: string) {
    await api.deleteSound(id);
    await useStore.getState().loadSounds();
  }

  async function clearData() {
    try {
      await api.clearAllData();
      await useStore.getState().refreshAll();
      setConfirmClear(false);
      pushToast({
        title: "All data cleared",
        body: "Reminders, habits and history removed.",
      });
    } catch (err) {
      pushToast({ title: "Could not clear data", body: String(err) });
    }
  }

  async function toggleAutostart() {
    const next = settings.launchAtStartup !== "true";
    try {
      if (next) await enable();
      else await disable();
      await update("launchAtStartup", String(next));
    } catch (err) {
      pushToast({ title: "Autostart unavailable", body: String(err) });
    }
  }

  return (
    <Page>
      <PageHeader title="Settings" />
      <Section title="General">
        <Card>
          <CardContent className="divide-y">
            <Row
              label="Launch at startup"
              hint="Open TungTung when you log in"
              control={
                <Switch
                  checked={settings.launchAtStartup === "true"}
                  onCheckedChange={toggleAutostart}
                />
              }
            />
            <Row
              label="Close to tray"
              hint="Keep reminders running after closing the window"
              control={
                <Switch
                  checked={settings.closeToTray === "true"}
                  onCheckedChange={(checked) =>
                    set("closeToTray", String(checked))
                  }
                />
              }
            />
            <Row
              label="Start minimized"
              hint="Start hidden in the tray"
              control={
                <Switch
                  checked={settings.startMinimized === "true"}
                  onCheckedChange={(checked) =>
                    set("startMinimized", String(checked))
                  }
                />
              }
            />
          </CardContent>
        </Card>
      </Section>

      <Section title="Appearance">
        <Card>
          <CardContent className="divide-y">
            <Row label="Theme"
              control={
                <div className="flex rounded-lg border p-0.5">
                  {(["system", "light", "dark"] as const).map((mode) => (
                    <Button
                      key={mode}
                      size="sm"
                      variant="ghost"
                      onClick={() => set("theme", mode)}
                      className={cn(
                        "h-7 rounded-md px-2.5 text-xs font-normal text-muted-foreground",
                        settings.theme === mode &&
                          "bg-muted font-medium text-foreground",
                      )}
                    >
                      {mode[0].toUpperCase() + mode.slice(1)}
                    </Button>
                  ))}
                </div>
              }
            />
          </CardContent>
        </Card>
      </Section>

      <Section title="Notifications">
        <Card>
          <CardContent className="divide-y">
            <Row
              label="Enable notifications"
              hint="Use the system notification service"
              control={
                <Switch
                  checked={settings.notificationsEnabled === "true"}
                  onCheckedChange={(checked) =>
                    set("notificationsEnabled", String(checked))
                  }
                />
              }
            />
            <Row
              label="Sound"
              hint="Play a sound when a notification fires"
              control={
                <Switch
                  checked={settings.soundEnabled === "true"}
                  onCheckedChange={(checked) =>
                    set("soundEnabled", String(checked))
                  }
                />
              }
            />
            <Row
              label="Volume"
              hint={`${settings.volume}%`}
              control={
                <input
                  type="range"
                  min={0}
                  max={100}
                  aria-label="Volume"
                  value={Number(settings.volume)}
                  onChange={(event) => set("volume", event.target.value)}
                  className="w-40 accent-primary"
                />
              }
            />
            <SoundRow
              label="Reminder sound"
              value={settings.reminderSound}
              sounds={sounds}
              volume={Number(settings.volume)}
              onChange={(value) => set("reminderSound", value)}
            />
            <SoundRow
              label="Pomodoro sound"
              value={settings.pomodoroSound}
              sounds={sounds}
              volume={Number(settings.volume)}
              onChange={(value) => set("pomodoroSound", value)}
            />
            <SoundRow
              label="Break sound"
              value={settings.breakSound}
              sounds={sounds}
              volume={Number(settings.volume)}
              onChange={(value) => set("breakSound", value)}
            />
            <Row
              label="Default snooze"
              hint="minutes"
              control={
                <Input
                  type="number"
                  min={1}
                  max={120}
                  className="w-20"
                  value={settings.snoozeMinutes}
                  onChange={(event) => set("snoozeMinutes", event.target.value)}
                />
              }
            />
            <Row
              label="Quiet hours"
              hint={`${settings.quietStart} – ${settings.quietEnd}`}
              control={
                <Switch
                  checked={settings.quietHoursEnabled === "true"}
                  onCheckedChange={(checked) =>
                    set("quietHoursEnabled", String(checked))
                  }
                />
              }
            />
            {settings.quietHoursEnabled === "true" ? (
              <div className="grid grid-cols-2 gap-4 py-3">
                <div className="grid gap-2">
                  <Label htmlFor="quiet-start">Start</Label>
                  <Input
                    id="quiet-start"
                    type="time"
                    value={settings.quietStart}
                    onChange={(event) => set("quietStart", event.target.value)}
                  />
                </div>
                <div className="grid gap-2">
                  <Label htmlFor="quiet-end">End</Label>
                  <Input
                    id="quiet-end"
                    type="time"
                    value={settings.quietEnd}
                    onChange={(event) => set("quietEnd", event.target.value)}
                  />
                </div>
              </div>
            ) : null}
          </CardContent>
        </Card>
      </Section>

      <Section title="Custom sounds">
        <Card>
          <CardContent className="divide-y">
            <Row
              label="Import sound"
              hint={
                diagnostics?.audioPlayer
                  ? `WAV, OGG or MP3 · short clips work best · closed-app playback uses ${diagnostics.audioPlayer}`
                  : "WAV, OGG or MP3 · short clips work best · sounds only play while the app is open unless you install one of pw-play, paplay, ffplay, sox, vlc or alsa-utils"
              }
              control={
                <Button variant="outline" onClick={importSoundFile}>
                  <UploadSimple size={14} data-icon="inline-start" /> Import
                </Button>
              }
            />
            {customSounds.length === 0 ? (
              <div className="py-3 text-xs text-muted-foreground">
                No custom sounds yet. Imported files are copied into the app's
                data folder.
              </div>
            ) : (
              customSounds.map((sound) => (
                <Row
                  key={sound.id}
                  label={sound.name}
                  hint={sound.filePath ?? undefined}
                  control={
                    <div className="flex items-center gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() =>
                          previewSound(
                            sound.id,
                            Number(settings.volume),
                            sound.filePath,
                          )
                        }
                      >
                        Test
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        aria-label={`Delete ${sound.name}`}
                        onClick={() => void removeSound(sound.id)}
                      >
                        <Trash size={15} />
                      </Button>
                    </div>
                  }
                />
              ))
            )}
          </CardContent>
        </Card>
      </Section>

      <Section title="Shortcuts">
        <Card>
          <CardContent className="divide-y">
            <Row label="Quick add" control={<Kbd>Ctrl + Shift + Space</Kbd>} />
            <Row label="Command palette" control={<Kbd>Ctrl + K</Kbd>} />
            <Row
              label="Today / Reminders / Focus / Habits"
              control={<Kbd>Ctrl + 1…4</Kbd>}
            />
            <Row label="Close dialog" control={<Kbd>Esc</Kbd>} />
          </CardContent>
        </Card>
      </Section>

      <Section title="Data">
        <Card>
          <CardContent className="divide-y">
            <Row
              label="Export data"
              hint="Download a JSON backup"
              control={
                <Button variant="outline" onClick={exportData}>
                  <DownloadSimple size={14} data-icon="inline-start" /> Export
                </Button>
              }
            />
            <Row
              label="Import data"
              hint="Restore from a backup file"
              control={
                <Button variant="outline" asChild>
                  <label className="cursor-pointer">
                    <UploadSimple size={14} data-icon="inline-start" />
                    {importing ? "Importing…" : "Import"}
                    <input
                      type="file"
                      accept="application/json"
                      className="hidden"
                      onChange={(event) => {
                        const file = event.target.files?.[0];
                        if (file) void importData(file);
                      }}
                    />
                  </label>
                </Button>
              }
            />
          </CardContent>
        </Card>

        <Card className="mt-3 border-destructive/40">
          <CardContent className="divide-y">
            <Row
              label="Clear all data"
              hint="Permanently delete every reminder, habit and focus session. This cannot be undone."
              control={
                <Button
                  variant="destructive"
                  onClick={() => setConfirmClear(true)}
                >
                  <Trash size={14} data-icon="inline-start" /> Clear
                </Button>
              }
            />
          </CardContent>
        </Card>

        <Dialog open={confirmClear} onOpenChange={setConfirmClear}>
          <DialogContent className="sm:max-w-md">
            <DialogHeader>
              <DialogTitle>Clear all data?</DialogTitle>
              <DialogDescription>
                Every reminder, habit, streak and focus session will be
                permanently deleted. Your settings and bundled sounds are kept.
                Export a backup first if you might want any of this back.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="ghost" onClick={() => setConfirmClear(false)}>
                Cancel
              </Button>
              <Button variant="destructive" onClick={clearData}>
                <Trash size={14} data-icon="inline-start" /> Delete everything
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </Section>

      <Section title="About">
        <Card>
          <CardContent className="divide-y">
            <Row
              label="Version"
              control={
                <Badge variant="secondary">
                  {diagnostics?.appVersion ?? "0.1.0"}
                </Badge>
              }
            />
            <Row
              label="Platform"
              control={
                <Badge variant="secondary">
                  {diagnostics?.platform ?? "—"}
                </Badge>
              }
            />
            <Row
              label="Desktop"
              control={
                <Badge variant="secondary">{diagnostics?.desktop ?? "—"}</Badge>
              }
            />
            <Row
              label="Display server"
              control={
                <Badge variant="secondary">{diagnostics?.display ?? "—"}</Badge>
              }
            />
            <Row
              label={
                <span className="flex items-center gap-1.5">
                  <Info size={12} /> Notifications
                </span>
              }
              hint={
                diagnostics?.notificationAvailable ? "Available" : "Unavailable"
              }
              control={
                <Badge
                  variant={
                    diagnostics?.notificationAvailable
                      ? "secondary"
                      : "destructive"
                  }
                >
                  {diagnostics?.notificationAvailable ? "Yes" : "No"}
                </Badge>
              }
            />
            <Row
              label="Database"
              hint={
                <span className="break-all">{diagnostics?.dbPath ?? "—"}</span>
              }
              control={null}
            />
            <Row
              label={
                <span className="flex items-center gap-1.5">
                  <Info size={12} /> Background audio
                </span>
              }
              hint={
                diagnostics?.audioPlayer
                  ? `Sounds play even while closed, using ${diagnostics.audioPlayer}`
                  : "Not available — install one of pw-play, paplay, ffplay, sox, vlc or alsa-utils for sounds while closed"
              }
              control={
                <Badge variant={diagnostics?.audioPlayer ? "secondary" : "destructive"}>
                  {diagnostics?.audioPlayer ? "Yes" : "No"}
                </Badge>
              }
            />
          </CardContent>
        </Card>
      </Section>
    </Page>
  );
}
