import { useId, useState, type FormEvent } from "react";
import { ChevronsUpDown, UserRound } from "lucide-react";
import { api, toAppError, type Profile } from "../api";
import { Modal } from "../components/Modal";
import { useT } from "../i18n";
import { useProfiles } from "../queries/profiles";
import { useStore } from "../state/store";

type NameSheet = { kind: "new" } | { kind: "rename"; profile: Profile };

const showError = (e: unknown) => useStore.getState().showToast("error", toAppError(e).message);

/** The name of a new profile, or a new name for `profile`. */
function ProfileNameSheet({ sheet, onClose }: { sheet: NameSheet; onClose: () => void }) {
  const id = useId();
  const { shell, components } = useT();
  const text = shell.profiles;
  const [name, setName] = useState(sheet.kind === "rename" ? sheet.profile.name : "");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const submit = (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    // The sheet stays open after creating: the app restarts on the new profile.
    const createProfile = () => api.profileCreate(name);
    const renameProfile = (profile: Profile) => api.profileRename(profile.id, name).then(onClose);
    (sheet.kind === "new" ? createProfile() : renameProfile(sheet.profile))
      .catch((err: unknown) => setError(toAppError(err).message))
      .finally(() => setBusy(false));
  };
  return (
    <Modal labelledBy={`${id}-title`} className="sheet--narrow" onClose={onClose}>
      <h2 id={`${id}-title`}>{sheet.kind === "new" ? text.newTitle : text.renameTitle}</h2>
      {sheet.kind === "new" && <p>{text.newText}</p>}
      <form onSubmit={submit}>
        <div className="form-field">
          <label htmlFor={`${id}-name`}>{text.name}</label>
          <input
            id={`${id}-name`}
            className="field"
            value={name}
            autoFocus
            aria-invalid={!!error}
            aria-describedby={`${id}-error`}
            onChange={(e) => {
              setName(e.target.value);
              setError("");
            }}
          />
          <div id={`${id}-error`} className="form-field__error" role={error ? "alert" : undefined}>
            {error}
          </div>
        </div>
        <div className="actions">
          <button type="button" className="btn" onClick={onClose}>
            {components.cancel}
          </button>
          <button type="submit" className="btn btn--primary" disabled={busy || !name.trim()}>
            {sheet.kind === "new" ? text.create : text.rename}
          </button>
        </div>
      </form>
    </Modal>
  );
}

/** The active profile in the sidebar; its menu switches, makes or renames profiles (ADR 0060). */
export function ProfileSwitcher() {
  const { data } = useProfiles();
  const text = useT().shell.profiles;
  const [sheet, setSheet] = useState<NameSheet | null>(null);
  const active = data?.profiles.find((p) => p.id === data.active);
  if (!data || !active) return null;
  const switchTo = (profile: Profile) => {
    if (profile.id !== active.id) api.profileSwitch(profile.id).catch(showError);
  };
  const open = (at: HTMLElement) =>
    useStore.getState().openMenu(
      [
        { caption: text.caption },
        ...data.profiles.map((p) => ({ text: p.name, checked: p.id === active.id, run: () => switchTo(p) })),
        "separator",
        { text: text.newItem, run: () => setSheet({ kind: "new" }) },
        { text: text.renameItem, run: () => setSheet({ kind: "rename", profile: active }) },
      ],
      at,
      text.label,
    );
  return (
    <>
      <button className="profile-btn" aria-haspopup="menu" aria-label={text.current(active.name)} onClick={(e) => open(e.currentTarget)}>
        <UserRound size={14} />
        <span className="profile-btn__name">{active.name}</span>
        <ChevronsUpDown size={12} />
      </button>
      {sheet && <ProfileNameSheet sheet={sheet} onClose={() => setSheet(null)} />}
    </>
  );
}
