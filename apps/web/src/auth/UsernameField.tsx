// The username field (R1435): the prompt after activation and the account page both use it.
//
// THE SERVER JUDGES, THE FIELD ASKS (CLAUDE.md rule 7). Nothing here checks a name: not its length,
// its characters, its script or the filter (R1432, R1433). `USERNAME_PREVIEW_DEBOUNCE_MS` after the
// last keystroke the field asks `GET /api/username/preview` what a save would give, aborting the
// question before it, and shows the answer as it stands: the exact username, tag and all ("Max is
// taken, so you’d be Max#3"), or the server's own sentence for a refusal. The length numbers in the
// hint are only words for the player, and the input carries no `maxLength`, since the server counts
// what a player sees (graphemes), not what a browser counts.
//
// A PLAYER IS NEVER GIVEN A TAG THEY WERE NOT SHOWN. Save is live only while the preview on screen
// is a yes for the name in the box, and sends exactly the username that preview named. If the
// outcome moved meanwhile (someone took `Max`, so it is `Max#1` now), the server refuses the save
// with a fresh preview, which replaces the old one with a line saying so, and the player saves again
// knowing what they get.

import { useEffect, useId, useState, type FormEvent, type ReactElement } from "react";

import { USERNAME_MAX_LENGTH, USERNAME_MIN_LENGTH, USERNAME_PREVIEW_DEBOUNCE_MS } from "@jackioh/server-config";
import {
  previewUsername,
  saveUsername,
  usernamePreviewOf,
  type OwnUsername,
  type UsernamePreview,
} from "../net/api.ts";
import Username from "./Username.tsx";
import { usernameTestid } from "./testids.ts";

/** A preview could not be read (the network, a rate limit): said plainly, never as a verdict. */
export const USERNAME_PREVIEW_FAILED = "Couldn’t check that name just now. Try again in a moment.";
/** A save failed for a reason that carried no preview. */
export const USERNAME_SAVE_FAILED = "Couldn’t save your username just now. Try again in a moment.";
/** A save the server refused because the outcome changed since the preview (a 409's fresh one). */
export const USERNAME_CHANGED = "That changed since you looked. Check the name above, then save again.";

/** R1435: when the next change is allowed, in the player's own clock and locale. The preview's
 * cooldown refusal and the account page's cooldown line both say it. */
export function nextChangeSentence(nextChangeAt: number): string {
  return `You can change your username again on ${new Date(nextChangeAt).toLocaleString()}.`;
}

/** The server's answer for one name exactly as typed; on screen only while the box still holds it. */
type Shown = { name: string; preview: UsernamePreview; changed: boolean };

export type UsernameFieldProps = {
  token: string;
  /** The save went through: the caller's username as the server now holds it. */
  onSaved: (own: OwnUsername) => void;
  /** The field's label. */
  label?: string;
  /** Locks the field while something else answers for it (the prompt's Skip). */
  disabled?: boolean;
};

/** A preview in a player's words, every name in it isolated (R1436). */
function PreviewWords({ preview }: { preview: UsernamePreview }): ReactElement {
  if (!preview.ok) {
    // The cooldown's end comes with the refusal (R1435), the time the server sent and not one the
    // field works out.
    if (preview.reason === "cooldown") return <>{`${preview.message} ${nextChangeSentence(preview.nextChangeAt)}`}</>;
    return <>{preview.message}</>;
  }
  if (preview.tagged) {
    return (
      <>
        <Username name={preview.base} />
        {" is taken, so you’d be "}
        <Username name={preview.username} />
      </>
    );
  }
  return (
    <>
      {"You’d be "}
      <Username name={preview.username} />
    </>
  );
}

export default function UsernameField({ token, onSaved, label = "Username", disabled = false }: UsernameFieldProps): ReactElement {
  const id = useId();
  const hintId = `${id}-hint`;
  const previewId = `${id}-preview`;
  const [value, setValue] = useState("");
  const [shown, setShown] = useState<Shown | null>(null);
  const [failure, setFailure] = useState<{ name: string; message: string } | null>(null);
  const [saving, setSaving] = useState(false);

  // The debounced preview: one question per pause in the typing, the one before it aborted. An
  // empty box asks nothing.
  useEffect(() => {
    if (value.trim().length === 0) return;
    const controller = new AbortController();
    const timer = window.setTimeout(() => {
      previewUsername(token, value, controller.signal).then(
        (preview) => {
          if (controller.signal.aborted) return;
          setShown({ name: value, preview, changed: false });
          setFailure(null);
        },
        () => {
          if (controller.signal.aborted) return;
          setFailure({ name: value, message: USERNAME_PREVIEW_FAILED });
        },
      );
    }, USERNAME_PREVIEW_DEBOUNCE_MS);
    return () => {
      window.clearTimeout(timer);
      controller.abort();
    };
  }, [token, value]);

  const current = shown !== null && shown.name === value ? shown : null;
  const failed = failure !== null && failure.name === value ? failure.message : null;
  const asking = value.trim().length > 0 && current === null && failed === null;
  const offer = current !== null && current.preview.ok ? current.preview.username : null;
  const canSave = offer !== null && !saving && !disabled;

  const save = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    if (offer === null || !canSave) return;
    const name = value;
    setSaving(true);
    setFailure(null);
    saveUsername(token, offer)
      .then((own) => {
        setValue("");
        setShown(null);
        onSaved(own);
      })
      .catch((cause: unknown) => {
        // A 400 or 409 carries the server's preview as it stands now (R1435); anything else is ours.
        const fresh = usernamePreviewOf(cause);
        if (fresh === null) setFailure({ name, message: USERNAME_SAVE_FAILED });
        else setShown({ name, preview: fresh, changed: fresh.ok });
      })
      .finally(() => {
        setSaving(false);
      });
  };

  return (
    <form className="form-card username-field" noValidate onSubmit={save}>
      <label htmlFor={id}>{label}</label>
      <input
        id={id}
        data-testid={usernameTestid.input}
        type="text"
        autoComplete="username"
        autoCapitalize="off"
        autoCorrect="off"
        spellCheck={false}
        value={value}
        disabled={saving || disabled}
        aria-invalid={current !== null && !current.preview.ok ? true : undefined}
        aria-describedby={`${hintId} ${previewId}`}
        onChange={(event) => {
          setValue(event.target.value);
        }}
      />
      <p className="auth-hint" id={hintId}>
        {String(USERNAME_MIN_LENGTH)} to {String(USERNAME_MAX_LENGTH)} letters, digits or underscores. If someone
        already has the name, you get a number after it.
      </p>
      <div id={previewId} aria-live="polite">
        {current !== null ? (
          <p
            className={current.preview.ok ? "username-field__preview" : "username-field__preview auth-field-error"}
            data-testid={usernameTestid.preview}
            data-ok={String(current.preview.ok)}
          >
            <PreviewWords preview={current.preview} />
          </p>
        ) : null}
        {current?.changed === true ? (
          <p className="username-field__changed" data-testid={usernameTestid.changed}>
            {USERNAME_CHANGED}
          </p>
        ) : null}
        {asking ? <p className="auth-hint">Checking…</p> : null}
      </div>
      {failed !== null ? (
        <p className="auth-field-error" data-testid={usernameTestid.error} role="alert">
          {failed}
        </p>
      ) : null}
      <button type="submit" data-testid={usernameTestid.save} disabled={!canSave} aria-busy={saving}>
        {saving ? "Saving…" : "Save"}
      </button>
    </form>
  );
}
