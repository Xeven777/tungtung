import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { api } from "@/lib/api";
import { fromLocalInputValue, toIsoUtc, toLocalInputValue } from "@/lib/dates";
import { RECURRENCE_OPTIONS } from "@/lib/recurrence";
import { useStore } from "@/store";

const NONE = "none";
const DEFAULT_SOUND = "default";

export function ReminderEditor() {
  const editing = useStore((s) => s.editing);
  const setEditing = useStore((s) => s.setEditing);
  const sounds = useStore((s) => s.sounds);
  const pushToast = useStore((s) => s.pushToast);

  const [title, setTitle] = useState("");
  const [notes, setNotes] = useState("");
  const [when, setWhen] = useState("");
  const [rule, setRule] = useState(NONE);
  const [soundId, setSoundId] = useState(DEFAULT_SOUND);
  const [notify, setNotify] = useState(true);

  useEffect(() => {
    if (!editing) return;
    setTitle(editing.title);
    setNotes(editing.notes ?? "");
    setWhen(toLocalInputValue(new Date(editing.dueAt)));
    setRule(editing.recurrenceRule || NONE);
    setSoundId(editing.soundId || DEFAULT_SOUND);
    setNotify(editing.notificationEnabled);
  }, [editing]);

  const close = () => setEditing(null);
  if (!editing) return null;

  async function save() {
    if (!title.trim() || !editing) return;
    await api.updateReminder(editing.id, {
      title: title.trim(),
      notes: notes.trim() || null,
      dueAt: toIsoUtc(fromLocalInputValue(when)),
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      recurrenceRule: rule === NONE ? null : rule,
      soundId: soundId === DEFAULT_SOUND ? null : soundId,
      notificationEnabled: notify,
    });
    await useStore.getState().refreshAll();
    pushToast({ title: "Reminder updated" });
    close();
  }

  async function remove() {
    if (!editing) return;
    await api.deleteReminder(editing.id);
    await useStore.getState().refreshAll();
    pushToast({ title: "Reminder deleted" });
    close();
  }

  return (
    <Dialog open={Boolean(editing)} onOpenChange={(next) => !next && close()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Edit reminder</DialogTitle>
        </DialogHeader>

        <div className="grid gap-4">
          <div className="grid gap-2">
            <Label htmlFor="reminder-title">Title</Label>
            <Input
              id="reminder-title"
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              autoFocus
            />
          </div>

          <div className="grid gap-2">
            <Label htmlFor="reminder-notes">Notes</Label>
            <Textarea
              id="reminder-notes"
              rows={2}
              value={notes}
              placeholder="Optional details"
              onChange={(event) => setNotes(event.target.value)}
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="grid gap-2">
              <Label htmlFor="reminder-when">Date &amp; time</Label>
              <Input
                id="reminder-when"
                type="datetime-local"
                value={when}
                onChange={(event) => setWhen(event.target.value)}
              />
            </div>
            <div className="grid gap-2">
              <Label>Repeat</Label>
              <Select value={rule} onValueChange={setRule}>
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {RECURRENCE_OPTIONS.map((option) => (
                    <SelectItem key={option.value || NONE} value={option.value || NONE}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="grid grid-cols-2 items-end gap-4">
            <div className="grid gap-2">
              <Label>Sound</Label>
              <Select value={soundId} onValueChange={setSoundId}>
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={DEFAULT_SOUND}>Default</SelectItem>
                  <SelectItem value={NONE}>None</SelectItem>
                  {sounds.map((sound) => (
                    <SelectItem key={sound.id} value={sound.id}>
                      {sound.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="flex items-center justify-between rounded-lg border px-3 py-2">
              <Label htmlFor="reminder-notify" className="text-sm">
                Notify
              </Label>
              <Switch id="reminder-notify" checked={notify} onCheckedChange={setNotify} />
            </div>
          </div>
        </div>

        <DialogFooter className="sm:justify-between">
          <Button variant="destructive" onClick={remove}>
            Delete
          </Button>
          <div className="flex gap-2">
            <Button variant="ghost" onClick={close}>
              Cancel
            </Button>
            <Button onClick={save} disabled={!title.trim()}>
              Save
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
