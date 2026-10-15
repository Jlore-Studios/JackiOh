// The card creator (#920, SPEC §12.9): the account's custom cards in a list, and an editor for one
// of them: the frame form, the no-code builder or the code view (K3), the problems the checks found,
// a live preview and the save, delete and "Try it" buttons.
//
// The checks guide and the server decides (CLAUDE.md rule 7). The frame's issues are the engine
// validator's (`validateCustomCard`, the normal module, at once); the script's are the Luau module's
// (`checkCustomCard`, loaded the first time the editor asks, R915009), and a script with a problem is
// not saved, since the server never compiles one (§12.9). A save the server refuses shows its issues
// or its sentence as it sent them.

import { useCallback, useEffect, useMemo, useRef, useState, type ReactElement } from "react";

import type { CardDef, CustomCard, CustomIssue } from "@jackioh/shared";

import { CardFace } from "../cards/CardFace.tsx";
import { faceModel } from "../cards/model.ts";
import { t } from "../i18n/i18n.ts";
import { ApiRequestError } from "../net/api.ts";
import {
  createCustomCard,
  customIssuesOf,
  deleteCustomCard,
  getCustomCards,
  staleVersionOf,
  updateCustomCard,
  type CustomCardInput,
  type CustomCardsResponse,
} from "../net/customCards.ts";
import { checkCustomCard, type ScriptProblem } from "../wasm/index.ts";
import { customDef, validateCustomCard } from "../wire/validator.ts";
import { BuilderView } from "./BuilderView.tsx";
import { CodeView, ScriptProblems } from "./CodeView.tsx";
import {
  builderRefusal,
  cardOf,
  draftOf,
  isDirty,
  newDraft,
  restartInBuilder,
  withFrame,
  withModel,
  withSource,
  withView,
  type Draft,
} from "./draft.ts";
import { FrameForm } from "./FrameForm.tsx";
import { creatorTestid } from "./testids.ts";
import { useCustomCards } from "./useCustomCards.ts";

import "./creator.css";

/** How long the editor waits after the last edit before it asks the Luau module to check the script. */
export const CHECK_DELAY_MS = 300;

/** The server's routes and the Luau module's check, as the page reaches them; a test hands its own. */
export type CreatorApi = {
  list: (token: string) => Promise<CustomCardsResponse>;
  create: (token: string, input: CustomCardInput) => Promise<{ card: CustomCard }>;
  update: (token: string, id: string, version: number, input: CustomCardInput) => Promise<{ card: CustomCard }>;
  remove: (token: string, id: string) => Promise<{ deleted: boolean }>;
  check: (card: CustomCard) => Promise<{ script: ScriptProblem[] }>;
};

export const DEFAULT_CREATOR_API: CreatorApi = {
  list: getCustomCards,
  create: createCustomCard,
  update: updateCustomCard,
  remove: deleteCustomCard,
  check: (card) => checkCustomCard(card),
};

export type CreatorProps = {
  token: string;
  api?: CreatorApi;
  /** Where "Try it" goes: the practice route's path for the saved card. */
  onTry: (cardId: string) => void;
};

type ScriptCheck =
  | { kind: "checking" }
  | { kind: "checked"; source: string; problems: ScriptProblem[] }
  | { kind: "failed"; message: string };

type SaveState =
  | { kind: "idle" }
  | { kind: "saving" }
  | { kind: "saved" }
  | { kind: "refused"; message: string; issues: CustomIssue[] }
  | { kind: "stale"; version: number };

function inputOf(draft: Draft): CustomCardInput {
  const card = cardOf(draft);
  return { frame: card.frame, source: card.source, ...(card.builder === undefined ? {} : { builder: card.builder }) };
}

function messageOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/** The script's problems, checked a moment after the last edit; stale answers are dropped. */
function useScriptCheck(card: CustomCard, check: CreatorApi["check"]): ScriptCheck {
  const [result, setResult] = useState<ScriptCheck>({ kind: "checking" });
  const asked = useRef(0);
  const source = card.source;
  const latest = useRef(card);
  latest.current = card;
  useEffect(() => {
    const ask = asked.current + 1;
    asked.current = ask;
    setResult({ kind: "checking" });
    const timer = setTimeout(() => {
      check(latest.current).then(
        (answer) => {
          if (asked.current === ask) setResult({ kind: "checked", source, problems: answer.script });
        },
        (cause: unknown) => {
          if (asked.current === ask) setResult({ kind: "failed", message: messageOf(cause) });
        },
      );
    }, CHECK_DELAY_MS);
    return () => clearTimeout(timer);
  }, [source, check]);
  return result;
}

function Preview({ card }: { card: CustomCard }): ReactElement {
  const [radiant, setRadiant] = useState(false);
  const def = useMemo((): CardDef | null => {
    try {
      return customDef(card);
    } catch {
      return null;
    }
  }, [card]);
  return (
    <div className="creator-preview" data-testid={creatorTestid.preview}>
      {def === null ? null : (
        <div className="creator-preview__face">
          <CardFace face={faceModel({ defId: card.id, def, radiant })} />
        </div>
      )}
      <label className="creator-check">
        <input type="checkbox" data-testid={creatorTestid.previewRadiant} checked={radiant} onChange={(event) => setRadiant(event.target.checked)} />
        {t("Show its Radiant face")}
      </label>
    </div>
  );
}

function Editor({
  draft,
  saved,
  atLimit,
  check,
  save,
  onDraft,
  onSave,
  onDelete,
  onTry,
  onReloadStale,
}: {
  draft: Draft;
  saved: CustomCard | null;
  atLimit: boolean;
  check: CreatorApi["check"];
  save: SaveState;
  onDraft: (draft: Draft) => void;
  onSave: () => void;
  onDelete: () => void;
  onTry: () => void;
  onReloadStale: () => void;
}): ReactElement {
  const card = useMemo(() => cardOf(draft), [draft]);
  const frameIssues = useMemo((): CustomIssue[] => {
    try {
      return validateCustomCard(card).filter((issue) => issue.field !== "id" && issue.field !== "version");
    } catch {
      return [];
    }
  }, [card]);
  const script = useScriptCheck(card, check);
  const problems = script.kind === "checked" && script.source === draft.source ? script.problems : [];
  const issues = save.kind === "refused" && save.issues.length > 0 ? save.issues : frameIssues;
  const refusal = builderRefusal(draft);
  const dirty = isDirty(draft, saved);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [confirmRestart, setConfirmRestart] = useState(false);
  const scriptReady = script.kind === "checked" && script.source === draft.source && problems.length === 0;
  const canSave = scriptReady && frameIssues.length === 0 && save.kind !== "saving" && (dirty || saved === null) && !(saved === null && atLimit);

  return (
    <section className="creator-editor" data-testid={creatorTestid.editor} aria-label={t("Card editor")}>
      <div className="creator-editor__main">
        <FrameForm frame={draft.frame} issues={issues} editParams={draft.view === "code"} onChange={(frame) => onDraft(withFrame(draft, frame))} />

        <div className="creator-tabs" role="tablist" aria-label={t("How the card's effects are written")}>
          <button
            type="button"
            role="tab"
            aria-selected={draft.view === "builder"}
            className={draft.view === "builder" ? "creator-tab creator-tab--on" : "creator-tab"}
            data-testid={creatorTestid.viewBuilder}
            disabled={refusal !== null}
            onClick={() => onDraft(withView(draft, "builder"))}
          >
            {t("Builder")}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={draft.view === "code"}
            className={draft.view === "code" ? "creator-tab creator-tab--on" : "creator-tab"}
            data-testid={creatorTestid.viewCode}
            onClick={() => onDraft(withView(draft, "code"))}
          >
            {t("Code")}
          </button>
        </div>
        {refusal === null ? null : (
          <div className="creator-note" data-testid={creatorTestid.builderRefusal}>
            <p>{t(refusal)}</p>
            {confirmRestart ? (
              <p className="creator-row">
                <span>{t("Replace the script with an empty one in the builder?")}</span>
                <button
                  type="button"
                  className="button-secondary"
                  data-testid={creatorTestid.restartBuilder}
                  onClick={() => {
                    setConfirmRestart(false);
                    onDraft(restartInBuilder(draft));
                  }}
                >
                  {t("Replace it")}
                </button>
                <button type="button" className="button-secondary" onClick={() => setConfirmRestart(false)}>
                  {t("Keep the code")}
                </button>
              </p>
            ) : (
              <button type="button" className="link-button" onClick={() => setConfirmRestart(true)}>
                {t("Start over in the builder")}
              </button>
            )}
          </div>
        )}

        {draft.view === "builder" && draft.model !== null ? (
          <BuilderView model={draft.model} type={draft.frame.type} onChange={(model) => onDraft(withModel(draft, model))} />
        ) : (
          <CodeView source={draft.source} problems={problems} onChange={(source) => onDraft(withSource(draft, source))} />
        )}

        {script.kind === "checking" ? (
          <p className="creator-hint" data-testid={creatorTestid.checking}>
            {t("Checking the script…")}
          </p>
        ) : null}
        {script.kind === "failed" ? (
          <p className="creator-error" role="alert">
            {t("The script could not be checked: {message}", { message: script.message })}
          </p>
        ) : null}
        <ScriptProblems problems={problems} />
      </div>

      <aside className="creator-editor__side">
        <Preview card={card} />
        <div className="creator-actions">
          <button type="button" className="button-primary" data-testid={creatorTestid.save} disabled={!canSave} onClick={onSave}>
            {save.kind === "saving" ? t("Saving…") : t("Save")}
          </button>
          <button type="button" className="button-secondary" data-testid={creatorTestid.tryIt} disabled={!(canSave || (saved !== null && !dirty))} onClick={onTry}>
            {t("Try it against the AI")}
          </button>
          {saved === null ? null : confirmDelete ? (
            <div className="creator-note">
              <p>{t("Delete this card? A deck that holds it cannot start a game until the card is replaced.")}</p>
              <div className="creator-row">
                <button
                  type="button"
                  className="button-secondary"
                  data-testid={creatorTestid.deleteConfirm}
                  onClick={() => {
                    setConfirmDelete(false);
                    onDelete();
                  }}
                >
                  {t("Delete it")}
                </button>
                <button type="button" className="button-secondary" onClick={() => setConfirmDelete(false)}>
                  {t("Keep it")}
                </button>
              </div>
            </div>
          ) : (
            <button type="button" className="button-secondary" data-testid={creatorTestid.delete} onClick={() => setConfirmDelete(true)}>
              {t("Delete")}
            </button>
          )}
        </div>
        {save.kind === "saved" && !dirty ? (
          <p className="creator-hint" data-testid={creatorTestid.saved} role="status">
            {t("Saved to your account.")}
          </p>
        ) : null}
        {save.kind === "refused" ? (
          <p className="creator-error" data-testid={creatorTestid.saveError} role="alert">
            {save.message}
          </p>
        ) : null}
        {save.kind === "stale" ? (
          <div className="creator-error" data-testid={creatorTestid.saveError} role="alert">
            <p>{t("This card was saved somewhere else since you opened it (version {version}).", { version: String(save.version) })}</p>
            <button type="button" className="button-secondary" data-testid={creatorTestid.reloadStale} onClick={onReloadStale}>
              {t("Open the newer save")}
            </button>
          </div>
        ) : null}
        {!scriptReady && script.kind === "checked" && problems.length > 0 ? (
          <p className="creator-hint">{t("Fix the script's problems to save it.")}</p>
        ) : null}
      </aside>
    </section>
  );
}

export function Creator({ token, api = DEFAULT_CREATOR_API, onTry }: CreatorProps): ReactElement {
  const { state, reload, replace } = useCustomCards(token, api.list);
  const cards = state.kind === "ready" ? state.cards : [];
  const limit = state.kind === "ready" ? state.limit : 0;
  const [draft, setDraft] = useState<Draft>(newDraft);
  const [save, setSave] = useState<SaveState>({ kind: "idle" });
  /** A card the player asked to open while the draft had unsaved changes. */
  const [pendingOpen, setPendingOpen] = useState<CustomCard | "new" | null>(null);
  const saved = draft.id === null ? null : (cards.find((card) => card.id === draft.id) ?? null);
  const dirty = isDirty(draft, saved);
  const reopenAfterReload = useRef<string | null>(null);

  // After "Open the newer save": the list is read again, then that card opened from it.
  useEffect(() => {
    const id = reopenAfterReload.current;
    if (id === null || state.kind !== "ready") return;
    reopenAfterReload.current = null;
    const card = state.cards.find((held) => held.id === id);
    setDraft(card === undefined ? newDraft() : draftOf(card));
    setSave({ kind: "idle" });
  }, [state]);

  const open = useCallback(
    (target: CustomCard | "new", force = false) => {
      if (!force && dirty && !(draft.id === null && JSON.stringify(cardOf(draft)) === JSON.stringify(cardOf(newDraft())))) {
        setPendingOpen(target);
        return;
      }
      setPendingOpen(null);
      setSave({ kind: "idle" });
      setDraft(target === "new" ? newDraft() : draftOf(target));
    },
    [dirty, draft],
  );

  const persist = useCallback(async (): Promise<CustomCard | null> => {
    setSave({ kind: "saving" });
    try {
      const { card } =
        draft.id === null || draft.version === null
          ? await api.create(token, inputOf(draft))
          : await api.update(token, draft.id, draft.version, inputOf(draft));
      const next = cards.some((held) => held.id === card.id) ? cards.map((held) => (held.id === card.id ? card : held)) : [...cards, card];
      replace(next);
      setDraft((current) => ({ ...current, id: card.id, version: card.version }));
      setSave({ kind: "saved" });
      return card;
    } catch (cause) {
      const stale = staleVersionOf(cause);
      if (stale !== null) {
        setSave({ kind: "stale", version: stale });
        return null;
      }
      const issues = customIssuesOf(cause);
      const message = cause instanceof ApiRequestError || cause instanceof Error ? cause.message : String(cause);
      setSave({ kind: "refused", message, issues });
      return null;
    }
  }, [api, cards, draft, replace, token]);

  const onDelete = useCallback(async () => {
    if (draft.id === null) return;
    const id = draft.id;
    try {
      await api.remove(token, id);
      replace(cards.filter((card) => card.id !== id));
      setDraft(newDraft());
      setSave({ kind: "idle" });
    } catch (cause) {
      setSave({ kind: "refused", message: messageOf(cause), issues: [] });
    }
  }, [api, cards, draft.id, replace, token]);

  const onTryIt = useCallback(async () => {
    const card = dirty || draft.id === null ? await persist() : saved;
    if (card !== null) onTry(card.id);
  }, [dirty, draft.id, onTry, persist, saved]);

  const onReloadStale = useCallback(() => {
    reopenAfterReload.current = draft.id;
    reload();
  }, [draft.id, reload]);

  return (
    <div className="creator" data-testid={creatorTestid.screen}>
      <aside className="creator-list" data-testid={creatorTestid.list} aria-label={t("Your custom cards")}>
        <h2>{t("Your custom cards")}</h2>
        {state.kind === "loading" ? <p className="creator-hint">{t("Loading your cards…")}</p> : null}
        {state.kind === "failed" ? (
          <div className="creator-error" data-testid={creatorTestid.loadFailed} role="alert">
            <p>{t("Your custom cards could not be loaded just now.")}</p>
            <button type="button" className="button-secondary" onClick={reload}>
              {t("Try again")}
            </button>
          </div>
        ) : null}
        {state.kind === "ready" ? (
          <p className="creator-hint" data-testid={creatorTestid.listLimit}>
            {t("{count} of {limit} cards", { count: String(cards.length), limit: String(limit) })}
          </p>
        ) : null}
        <ul className="creator-list__items">
          {cards.map((card) => (
            <li key={card.id}>
              <button
                type="button"
                className={card.id === draft.id ? "creator-list__item creator-list__item--on" : "creator-list__item"}
                data-testid={creatorTestid.listItem}
                data-card-id={card.id}
                aria-current={card.id === draft.id ? "true" : undefined}
                onClick={() => open(card)}
              >
                <span className="creator-list__cost">{card.frame.cost}</span>
                <span className="creator-list__name">{card.frame.name}</span>
                <span className="creator-list__type">{t(card.frame.type)}</span>
              </button>
            </li>
          ))}
        </ul>
        <button
          type="button"
          className="button-secondary"
          data-testid={creatorTestid.newCard}
          disabled={state.kind === "ready" && cards.length >= limit}
          onClick={() => open("new")}
        >
          {t("New card")}
        </button>
        {state.kind === "ready" && cards.length >= limit ? (
          <p className="creator-hint">{t("You have the most custom cards an account can keep. Delete one to make another.")}</p>
        ) : null}
      </aside>

      <div className="creator-body">
        {pendingOpen === null ? null : (
          <div className="creator-note" role="alert">
            <p>{t("This card has changes you haven't saved.")}</p>
            <div className="creator-row">
              <button type="button" className="button-secondary" onClick={() => open(pendingOpen, true)}>
                {t("Discard them")}
              </button>
              <button type="button" className="button-secondary" onClick={() => setPendingOpen(null)}>
                {t("Keep editing")}
              </button>
            </div>
          </div>
        )}
        <Editor
          key={draft.id ?? "new"}
          draft={draft}
          saved={saved}
          atLimit={state.kind === "ready" && cards.length >= limit}
          check={api.check}
          save={save}
          onDraft={(next) => {
            setDraft(next);
            if (save.kind !== "saving") setSave({ kind: "idle" });
          }}
          onSave={() => void persist()}
          onDelete={() => void onDelete()}
          onTry={() => void onTryIt()}
          onReloadStale={onReloadStale}
        />
      </div>
    </div>
  );
}
