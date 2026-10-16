import { useEffect, useRef, useState } from "react";

import type { ActionBody } from "@jackioh/shared";

import type {
  CardType,
  CraftEffect,
  CraftHatKind,
  CraftRecipe,
  CraftVerb,
  Keyword,
  PendingOption,
  PendingPromptView,
} from "../../wire/index.ts";
import {
  CRAFT_ADJECTIVES,
  CRAFT_HAT_PRICES,
  CRAFT_KEYWORD_PRICES,
  CRAFT_MAX_N,
  CRAFT_NOUNS,
  CRAFT_VERB_PRICES,
} from "../../wire/engineConfig.ts";
import { craftPreview } from "../../wire/engine.ts";
import { CardFace, faceModel, useInspectTrigger } from "../../cards/index.ts";
import { readerTerms } from "../../cards/chinese.ts";
import { t, useLocale } from "../../i18n/i18n.ts";

import {
  HAT_LABELS,
  VERB_LABELS,
  addEffect,
  addHat,
  moveEffect,
  removeEffect,
  removeHat,
  setEffectSlot,
  setType,
  stepKeywordN,
  stepN,
  stepStat,
  takesHat,
  takesKeyword,
  toggleKeyword,
} from "./recipe.ts";
import "./craft.css";

/** ME-CRAFT (Meditative #17, R880–R883): the Scratch-style block editor. Blocks snap together for
 * type, stats, keywords, hats and verbs; the meters and the face are the engine's own
 * `craftPreview`, so the client decides nothing (CLAUDE.md rule 7). The turn clock keeps running
 * while the player crafts (R79): this is an answer like any other, and the opponent sees only that
 * a prompt is open. Pointer drag moves blocks with a mouse or a finger; every drag has a click,
 * tap and keyboard equivalent, so nothing needs a drag. */

const TYPES: readonly CardType[] = ["Unit", "Spell", "Field Spell", "Trap"];

/** `prompt.css`:608 — the phone sheet query. Read in a try/catch, as `tutorial/Coach.tsx` does. */
const CRAFT_SHEET_QUERY = "(max-width: 600px), (orientation: landscape) and (max-height: 500px)";

function sheetQuery(): MediaQueryList | null {
  try {
    return typeof window.matchMedia === "function" ? window.matchMedia(CRAFT_SHEET_QUERY) : null;
  } catch {
    return null;
  }
}

/** A blank Unit to stand on when no preset carries a recipe (the engine always sends four). */
const FALLBACK_RECIPE: CraftRecipe = {
  cost: 0,
  type: "Unit",
  adjective: "Pure",
  noun: "Closure",
  attack: 1,
  health: 1,
  keywords: [],
  echo: 0,
  hats: [],
};

type DragPayload =
  | { from: "palette-hat"; hat: CraftHatKind }
  | { from: "palette-effect"; verb: CraftVerb }
  | { from: "canvas"; hat: number; effect: number };

type DragState = { payload: DragPayload; label: string; x: number; y: number; moved: boolean };

/** A card type in the card frame's own words in the reader's language (cards/chinese.ts, R1303). */
function typeName(type: CardType): string {
  return readerTerms()?.types[type] ?? type;
}

/** A keyword in the card frame's own words in the reader's language. */
function keywordName(kind: Keyword["kind"]): string {
  return readerTerms()?.keywords[kind] ?? kind;
}

// The prices are read where they are drawn, never at module scope, so a language switch changes them.
function verbPriceText(verb: CraftVerb): string {
  const row = CRAFT_VERB_PRICES.find((entry) => entry.verb === verb);
  if (row === undefined) return "";
  const points = row.perN > 0 ? `${row.points}+${row.perN}×N` : `${row.points}`;
  const halved = row.halve ? t(", halved") : "";
  return `${t("{points} pts", { points })}${halved} · ${t("{lines} lines", { lines: row.lines })}${row.targeted ? ` · ${t("targets")}` : ""}`;
}

function keywordPriceText(kind: Keyword["kind"]): string {
  const row = CRAFT_KEYWORD_PRICES.find((entry) => entry.kind === kind);
  if (row === undefined) return "";
  return row.perN > 0 ? t("{points} pts", { points: `${row.perN}×N` }) : t("{points} pts", { points: row.points });
}

function hatPriceText(hat: CraftHatKind): string {
  const row = CRAFT_HAT_PRICES.find((entry) => entry.hat === hat);
  if (row === undefined) return "";
  return `${t("{lines} lines", { lines: row.lines })}${row.multiplier > 1 ? ` · ${t("×{n} pts", { n: row.multiplier })}` : ""}`;
}

function effectText(effect: CraftEffect): string {
  const label = t(VERB_LABELS[effect.verb]);
  if (effect.n !== undefined) return t("{label} {n}", { label, n: effect.n });
  if (effect.keyword !== undefined) return t("{label}: {keyword}", { label, keyword: keywordName(effect.keyword) });
  if (effect.cardType !== undefined) return t("{label}: {type}", { label, type: typeName(effect.cardType) });
  return label;
}

export function CraftEditor(props: { pending: PendingPromptView; onAction: (body: ActionBody) => void }) {
  useLocale();
  const presets: PendingOption[] = props.pending.options;
  const cost = props.pending.budget ?? 0;
  const radiant = presets.some((option) => option.radiant === true);
  const [recipe, setRecipe] = useState<CraftRecipe>(
    () => presets.find((option) => option.recipe !== undefined)?.recipe ?? FALLBACK_RECIPE,
  );
  const [selectedHat, setSelectedHat] = useState(0);
  const [drag, setDrag] = useState<DragState | null>(null);
  const [sheet, setSheet] = useState(() => sheetQuery()?.matches === true);
  const suppressClick = useRef(false);
  const dragMoved = useRef(false);
  const dragStart = useRef<{ x: number; y: number; payload: DragPayload } | null>(null);

  useEffect(() => {
    const query = sheetQuery();
    if (query === null) return;
    const onChange = (): void => setSheet(query.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  const preview = craftPreview(recipe, cost);
  const hatCount = recipe.hats.length;
  const hatIndex = Math.min(selectedHat, Math.max(0, hatCount - 1));

  function send(): void {
    if (!preview.valid) return;
    props.onAction({
      type: "answer",
      choiceId: props.pending.choiceId,
      selection: [{ pick: "craft", recipe }],
    });
  }

  function drop(payload: DragPayload, target: Element | null): void {
    const hatAttr = target?.closest("[data-drop-hat]")?.getAttribute("data-drop-hat");
    const atAttr = target?.closest("[data-drop-at]")?.getAttribute("data-drop-at");
    const toPalette = target?.closest("[data-drop-palette]") !== null;
    if (payload.from === "palette-hat") {
      setRecipe((prev) => addHat(prev, payload.hat));
      return;
    }
    if (payload.from === "palette-effect") {
      if (hatAttr === null || hatAttr === undefined) return;
      const hat = Number(hatAttr);
      const at = atAttr === null || atAttr === undefined ? undefined : Number(atAttr);
      setRecipe((prev) => {
        if (prev.hats[hat] === undefined) return prev;
        const withEffect = addEffect(prev, hat, payload.verb);
        const placed = withEffect.hats[hat];
        if (at === undefined || placed === undefined) return withEffect;
        const from = { hat, effect: placed.effects.length - 1 };
        return moveEffect(withEffect, from, { hat, effect: at });
      });
      setSelectedHat(hat);
      return;
    }
    if (toPalette) {
      setRecipe((prev) => removeEffect(prev, payload.hat, payload.effect));
      return;
    }
    if (hatAttr === null || hatAttr === undefined) return;
    const toHat = Number(hatAttr);
    const at = atAttr === null || atAttr === undefined ? undefined : Number(atAttr);
    setRecipe((prev) => moveEffect(prev, { hat: payload.hat, effect: payload.effect }, { hat: toHat, effect: at }));
    setSelectedHat(toHat);
  }

  // Pointer drag, as Scratch does: press and hold still is a click (or a tap); moving past
  // 8px is a drag whose ghost follows the pointer until release. `dragMoved` is a ref because the
  // release handler reads it, not the render. Every drag has a click, tap and keyboard equivalent.
  useEffect(() => {
    if (dragStart.current === null) return;
    const start = dragStart.current;
    const onMove = (event: PointerEvent): void => {
      if (Math.abs(event.clientX - start.x) <= 8 && Math.abs(event.clientY - start.y) <= 8) return;
      dragMoved.current = true;
      setDrag((prev) =>
        prev === null
          ? null
          : { payload: prev.payload, label: prev.label, x: event.clientX, y: event.clientY, moved: true },
      );
    };
    const onUp = (event: PointerEvent): void => {
      const target = document.elementFromPoint(event.clientX, event.clientY);
      const wasDrag = dragMoved.current;
      dragMoved.current = false;
      dragStart.current = null;
      setDrag(null);
      if (wasDrag) {
        suppressClick.current = true;
        drop(start.payload, target);
      }
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
  });

  function beginDrag(event: React.PointerEvent, payload: DragPayload, label: string): void {
    if (event.pointerType === "mouse" && event.button !== 0) return;
    dragMoved.current = false;
    dragStart.current = { x: event.clientX, y: event.clientY, payload };
    setDrag({ payload, label, x: event.clientX, y: event.clientY, moved: false });
  }

  function clickGuard(run: () => void): void {
    if (suppressClick.current) {
      suppressClick.current = false;
      return;
    }
    run();
  }

  const def = preview.def;
  const face = faceModel({ defId: def.id, def, radiant });
  // #552: the card being crafted is read as any card is, a hover preview with its glossary and, on a
  // phone, the long-press sheet, so the words its blocks print are a press away. Lines of code is a
  // hidden stat in matches (the meters above say this card's).
  const inspect = useInspectTrigger({ key: "craft-preview", face }, { prefer: "above", showLoc: false });
  return (
    <div className="prompt-scrim" data-testid="prompt-scrim">
      <div
        className="prompt craft-editor"
        data-testid="prompt-modal"
        data-prompt-kind="craft"
        data-layout={sheet ? "sheet" : "panel"}
        role="dialog"
        aria-modal="true"
        aria-label={t("Craft a card")}
      >
        <p className="prompt-title">{t("Craft a card")}</p>
        <div className="craft-columns">
          <nav className="craft-palette" aria-label={t("Blocks")} data-drop-palette="true">
            <section aria-label={t("Type and stats")}>
              <h3>{t("Type and stats")}</h3>
              <div className="craft-row">
                {TYPES.map((type) => (
                  <button
                    key={type}
                    type="button"
                    className="craft-block"
                    aria-pressed={recipe.type === type}
                    onClick={() => setRecipe((prev) => setType(prev, type))}
                  >
                    {typeName(type)}
                  </button>
                ))}
              </div>
              {recipe.type === "Unit" && (
                <div className="craft-row">
                  <Stepper label={t("Attack")} value={recipe.attack} onStep={(delta) => setRecipe((prev) => stepStat(prev, "attack", delta))} />
                  <Stepper label={t("Health")} value={recipe.health} onStep={(delta) => setRecipe((prev) => stepStat(prev, "health", delta))} />
                </div>
              )}
              {recipe.type === "Spell" && (
                <div className="craft-row">
                  <Stepper
                    label={t("Echo")}
                    value={recipe.echo}
                    onStep={(delta) =>
                      setRecipe((prev) => ({ ...prev, echo: Math.min(CRAFT_MAX_N, Math.max(0, prev.echo + delta)) }))
                    }
                  />
                </div>
              )}
              <div className="craft-row">
                <label>
                  {t("Name")}{" "}
                  <select
                    data-testid="craft-adjective"
                    value={recipe.adjective}
                    onChange={(event) => setRecipe((prev) => ({ ...prev, adjective: event.target.value }))}
                  >
                    {CRAFT_ADJECTIVES.map((word) => (
                      <option key={word} value={word}>
                        {word}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  <select
                    data-testid="craft-noun"
                    value={recipe.noun}
                    aria-label={t("Second name word")}
                    onChange={(event) => setRecipe((prev) => ({ ...prev, noun: event.target.value }))}
                  >
                    {CRAFT_NOUNS.map((word) => (
                      <option key={word} value={word}>
                        {word}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
            </section>
            <section aria-label={t("Keywords")}>
              <h3>{t("Keywords")}</h3>
              {recipe.keywords.length > 0 && (
                <div className="craft-row" aria-label={t("Chosen keywords")}>
                  {recipe.keywords.map((block) => (
                    <span key={block.kind} className="craft-chosen">
                      {block.kind === "Armor" ? t("Armor {n}", { n: block.n ?? 1 }) : keywordName(block.kind)}
                      {block.kind === "Armor" && (
                        <span className="craft-stepper" role="group" aria-label={t("Armor")}>
                          <button type="button" aria-label={t("Decrease {label}", { label: t("Armor") })} onClick={() => setRecipe((prev) => stepKeywordN(prev, "Armor", -1))}>
                            −
                          </button>
                          <button type="button" aria-label={t("Increase {label}", { label: t("Armor") })} onClick={() => setRecipe((prev) => stepKeywordN(prev, "Armor", 1))}>
                            +
                          </button>
                        </span>
                      )}
                    </span>
                  ))}
                </div>
              )}
              <div className="craft-row">
                {CRAFT_KEYWORD_PRICES.map((row) => {
                  // R883: a type that cannot take this keyword leaves its click inert. The button still
                  // starts the drag of a "Give a unit X" effect, which any type may hold, so it is
                  // `aria-disabled`, not `disabled`, which would swallow the pointerdown.
                  const takes = takesKeyword(recipe.type, row.kind);
                  return (
                    <button
                      key={row.kind}
                      type="button"
                      className="craft-block"
                      aria-pressed={recipe.keywords.some((block) => block.kind === row.kind)}
                      aria-disabled={!takes}
                      title={
                        takes
                          ? keywordPriceText(row.kind)
                          : t("A {type} cannot take {keyword}", { type: typeName(recipe.type), keyword: keywordName(row.kind) })
                      }
                      onClick={() =>
                        clickGuard(() =>
                          setRecipe((prev) => (takesKeyword(prev.type, row.kind) ? toggleKeyword(prev, row.kind) : prev)),
                        )
                      }
                      onPointerDown={(event) => beginDrag(event, { from: "palette-effect", verb: "grantKeyword" }, t("Give a unit {keyword}", { keyword: keywordName(row.kind) }))}
                    >
                      {keywordName(row.kind)} · {keywordPriceText(row.kind)}
                    </button>
                  );
                })}
              </div>
            </section>
            <section aria-label={t("Hats")}>
              <h3>{t("Hats")}</h3>
              <div className="craft-row">
                {CRAFT_HAT_PRICES.filter((row) => takesHat(recipe.type, row.hat)).map((row) => (
                  <button
                    key={row.hat}
                    type="button"
                    className="craft-block"
                    title={hatPriceText(row.hat)}
                    onPointerDown={(event) => beginDrag(event, { from: "palette-hat", hat: row.hat }, t(HAT_LABELS[row.hat]))}
                    onClick={() => clickGuard(() => setRecipe((prev) => addHat(prev, row.hat)))}
                  >
                    {t(HAT_LABELS[row.hat])} · {hatPriceText(row.hat)}
                  </button>
                ))}
              </div>
            </section>
            <section aria-label={t("Effects")}>
              <h3>{t("Effects")}</h3>
              <div className="craft-row">
                {CRAFT_VERB_PRICES.map((row) => (
                  <button
                    key={row.verb}
                    type="button"
                    className="craft-block"
                    title={verbPriceText(row.verb)}
                    onPointerDown={(event) => beginDrag(event, { from: "palette-effect", verb: row.verb }, t(VERB_LABELS[row.verb]))}
                    onClick={() =>
                      clickGuard(() =>
                        setRecipe((prev) => {
                          if (prev.hats[hatIndex] === undefined) return prev;
                          return addEffect(prev, hatIndex, row.verb);
                        }),
                      )
                    }
                  >
                    {t(VERB_LABELS[row.verb])} · {verbPriceText(row.verb)}
                  </button>
                ))}
              </div>
            </section>
          </nav>
          <div className="craft-canvas" aria-label={t("Your card")}>
            {recipe.hats.map((hat, hi) => (
              <section
                key={`${hat.hat}-${hi}`}
                className="craft-hat"
                data-drop-hat={hi}
                aria-pressed={hi === hatIndex}
                aria-label={t("{hat} hat", { hat: t(HAT_LABELS[hat.hat]) })}
                onClick={() => setSelectedHat(hi)}
              >
                <header>
                  <strong>{t(HAT_LABELS[hat.hat])}</strong>
                  <button type="button" aria-label={t("Remove {hat} hat", { hat: t(HAT_LABELS[hat.hat]) })} onClick={() => setRecipe((prev) => removeHat(prev, hi))}>
                    ×
                  </button>
                </header>
                {hat.effects.map((effect, ei) => (
                  <div
                    key={ei}
                    className="craft-block craft-placed"
                    data-drop-hat={hi}
                    data-drop-at={ei}
                    tabIndex={0}
                    role="button"
                    aria-label={t("{effect} under {hat}", { effect: effectText(effect), hat: t(HAT_LABELS[hat.hat]) })}
                    onPointerDown={(event) => beginDrag(event, { from: "canvas", hat: hi, effect: ei }, effectText(effect))}
                    onClick={() => clickGuard(() => setSelectedHat(hi))}
                    onKeyDown={(event) => {
                      if (event.key === "+" || event.key === "=") setRecipe((prev) => stepN(prev, hi, ei, 1));
                      else if (event.key === "-" || event.key === "_") setRecipe((prev) => stepN(prev, hi, ei, -1));
                      else if (event.key === "Delete" || event.key === "Backspace") setRecipe((prev) => removeEffect(prev, hi, ei));
                      else if (event.key === "ArrowUp") setRecipe((prev) => moveEffect(prev, { hat: hi, effect: ei }, { hat: hi, effect: ei - 1 }));
                      else if (event.key === "ArrowDown") setRecipe((prev) => moveEffect(prev, { hat: hi, effect: ei }, { hat: hi, effect: ei + 1 }));
                      else if (event.key === "Enter") setSelectedHat(hi);
                      else return;
                      event.preventDefault();
                    }}
                  >
                    <span>{effectText(effect)}</span>
                    {effect.n !== undefined && (
                      <span className="craft-stepper" role="group" aria-label={t("Number")}>
                        <button type="button" aria-label={t("Decrease number")} onClick={() => setRecipe((prev) => stepN(prev, hi, ei, -1))}>
                          −
                        </button>
                        <button type="button" aria-label={t("Increase number")} onClick={() => setRecipe((prev) => stepN(prev, hi, ei, 1))}>
                          +
                        </button>
                      </span>
                    )}
                    {effect.verb === "grantKeyword" && (
                      <select
                        aria-label={t("Granted keyword")}
                        value={effect.keyword ?? ""}
                        onChange={(event) =>
                          setRecipe((prev) =>
                            setEffectSlot(prev, hi, ei, {
                              keyword:
                                event.target.value === ""
                                  ? undefined
                                  : (event.target.value as Keyword["kind"]),
                            }),
                          )
                        }
                      >
                        <option value="">{t("Choose")}</option>
                        {CRAFT_KEYWORD_PRICES.filter((row) => row.kind !== "Armor").map((row) => (
                          <option key={row.kind} value={row.kind}>
                            {keywordName(row.kind)}
                          </option>
                        ))}
                      </select>
                    )}
                    {(effect.verb === "addRandom" || effect.verb === "discover") && (
                      <select
                        aria-label={t("Card type")}
                        value={effect.cardType ?? ""}
                        onChange={(event) =>
                          setRecipe((prev) =>
                            setEffectSlot(prev, hi, ei, {
                              cardType:
                                event.target.value === "" ? undefined : (event.target.value as CardType),
                            }),
                          )
                        }
                      >
                        <option value="">{t("Any")}</option>
                        {TYPES.map((type) => (
                          <option key={type} value={type}>
                            {typeName(type)}
                          </option>
                        ))}
                      </select>
                    )}
                    <button type="button" aria-label={t("Remove {effect}", { effect: effectText(effect) })} onClick={() => setRecipe((prev) => removeEffect(prev, hi, ei))}>
                      ×
                    </button>
                  </div>
                ))}
                <div className="craft-gap" data-drop-hat={hi} data-drop-at={hat.effects.length} aria-hidden="true" />
              </section>
            ))}
          </div>
          <aside className="craft-side">
            <p className="craft-meters">
              <span data-testid="craft-points">
                {t("{points}/{budget} pts", { points: preview.points, budget: preview.pointsBudget })}
              </span>{" "}
              <span data-testid="craft-loc">
                {t("{loc}/{budget} lines", { loc: preview.loc, budget: preview.locBudget })}
              </span>
            </p>
            <ul className="craft-reasons" role="status" aria-label={t("Why this recipe is refused")}>
              {preview.reasons.map((reason) => (
                <li key={reason}>{t(reason)}</li>
              ))}
            </ul>
            <span className="craft-face" data-testid="craft-face" {...inspect.handlers}>
              <CardFace face={face} />
            </span>
            {inspect.overlay}
            <div className="craft-suggestions" aria-label={t("Suggestions")}>
              {presets.map((option, index) =>
                option.recipe === undefined ? null : (
                  <button
                    key={option.key}
                    type="button"
                    data-testid={`craft-preset-${index}`}
                    onClick={() => setRecipe(option.recipe ?? FALLBACK_RECIPE)}
                  >
                    {option.label}
                  </button>
                ),
              )}
            </div>
            <button
              type="button"
              data-testid="prompt-submit"
              className="prompt-submit"
              aria-disabled={!preview.valid}
              onClick={send}
            >
              {t("Craft")}
            </button>
          </aside>
        </div>
      </div>
      {drag?.moved === true && <div className="craft-ghost" style={{ left: drag.x, top: drag.y }}>{drag.label}</div>}
    </div>
  );
}

function Stepper(props: { label: string; value: number; onStep: (delta: number) => void }) {
  return (
    <span className="craft-stepper" role="group" aria-label={props.label}>
      <span aria-hidden="true">{props.label} {props.value}</span>
      <button type="button" aria-label={t("Decrease {label}", { label: props.label })} onClick={() => props.onStep(-1)}>
        −
      </button>
      <button type="button" aria-label={t("Increase {label}", { label: props.label })} onClick={() => props.onStep(1)}>
        +
      </button>
    </span>
  );
}
