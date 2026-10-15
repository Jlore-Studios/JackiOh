// The practice sandbox's dev tools (issue #923, SPEC §9.9, R923000–R923003): search for any card and
// spawn it for either side into any zone, set a hero's health and Armor and a player's mana, draw,
// destroy or remove a card, and hold the AI's turn while the board is arranged.
//
// It sends intent and nothing else (CLAUDE.md rule 7): every tool is a `sandbox` action the practice
// worker hands the engine, which refuses what its rules refuse, and the board shows the refusal as it
// shows any other. The bounds come from the engine's config (rule 9); the lanes from the view.

import { useId, useMemo, useState, type ReactElement } from "react";

import {
  SANDBOX_ARMOR_MAX,
  SANDBOX_DRAW_MAX,
  SANDBOX_HERO_HEALTH_MAX,
  SANDBOX_MANA_MAX,
} from "@jackioh/engine/config";
import { opponentOf, type CardDef, type CardDefs, type PlayerId, type PlayerView, type SandboxOp, type SpawnZone } from "@jackioh/shared";

import { CardFace, faceModel } from "../cards/index.ts";
import { t } from "../i18n/i18n.ts";
import { searchCards, spawnableDefs, visibleCards, type VisibleCard } from "./sandbox.ts";
import { practiceTestid } from "./testids.ts";

import "./sandbox.css";

export type SandboxPanelProps = {
  view: PlayerView;
  /** The worker's catalog; null while it loads. */
  defs: CardDefs | null;
  /** Whether the AI's next step waits (the controller's `sandbox` hold). */
  held: boolean;
  onHold(held: boolean): void;
  onTool(op: SandboxOp): void;
};

type Seat = "you" | "opponent";

/** Where the spawn picker puts a card: its `<select>` value and the zone it names. */
type Where = { value: string; label: string; zone: SpawnZone };

function wheres(view: PlayerView, seat: Seat): Where[] {
  const side = seat === "you" ? view.you : view.opponent;
  const lanes = (count: number, row: "units" | "backrow"): Where[] =>
    Array.from({ length: count }, (_, at) => {
      const lane = at + 1;
      return {
        value: `${row}:${String(lane)}`,
        label:
          row === "units"
            ? t("Unit row, lane {lane}", { lane: String(lane) })
            : t("Backrow, lane {lane}", { lane: String(lane) }),
        zone: { zone: "field", row, lane },
      };
    });
  return [
    { value: "hand", label: t("Hand"), zone: { zone: "hand" } },
    ...lanes(side.units.length, "units"),
    ...lanes(side.backrow.length, "backrow"),
    { value: "graveyard", label: t("Graveyard"), zone: { zone: "graveyard" } },
    { value: "libraryTop", label: t("Top of the deck"), zone: { zone: "libraryTop" } },
  ];
}

function nameOf(defs: CardDefs | null, view: PlayerView, defId: string): string {
  return defs?.[defId]?.name ?? view.defs?.[defId]?.name ?? defId;
}

/** A whole number from an input, or null when it is empty or not one. */
function whole(text: string): number | null {
  return /^\d+$/u.test(text.trim()) ? Number(text.trim()) : null;
}

function SeatPicker({ value, onChange, testid }: { value: Seat; onChange(seat: Seat): void; testid: string }): ReactElement {
  return (
    <select className="sandbox-panel__select" data-testid={testid} value={value} onChange={(event) => onChange(event.currentTarget.value as Seat)}>
      <option value="you">{t("You")}</option>
      <option value="opponent">{t("The AI")}</option>
    </select>
  );
}

function ResultCard({ def, radiant, picked, onPick }: { def: CardDef; radiant: boolean; picked: boolean; onPick(): void }): ReactElement {
  const face = useMemo(() => faceModel({ defId: def.id, def, radiant }), [def, radiant]);
  return (
    <li className="sandbox-panel__result">
      <button
        type="button"
        className="sandbox-panel__pick"
        data-testid={practiceTestid.sandboxResult(def.id)}
        aria-pressed={picked}
        aria-label={def.name}
        onClick={onPick}
      >
        <CardFace face={face} layout="compact" lazyArt />
      </button>
    </li>
  );
}

export function SandboxPanel({ view, defs, held, onHold, onTool }: SandboxPanelProps): ReactElement {
  const baseId = useId();
  const [open, setOpen] = useState(true);
  const [query, setQuery] = useState("");
  const [radiant, setRadiant] = useState(false);
  const [picked, setPicked] = useState<string | null>(null);
  const [spawnSeat, setSpawnSeat] = useState<Seat>("you");
  const [where, setWhere] = useState("hand");
  const [heroSeat, setHeroSeat] = useState<Seat>("you");
  const [health, setHealth] = useState("");
  const [armor, setArmor] = useState("");
  const [manaSeat, setManaSeat] = useState<Seat>("you");
  const [current, setCurrent] = useState("");
  const [max, setMax] = useState("");
  const [drawSeat, setDrawSeat] = useState<Seat>("you");
  const [drawCount, setDrawCount] = useState("1");
  const [target, setTarget] = useState("");

  const pool = useMemo(() => (defs === null ? [] : spawnableDefs(defs)), [defs]);
  const results = useMemo(() => searchCards(pool, query), [pool, query]);
  const places = wheres(view, spawnSeat);
  const place = places.find((option) => option.value === where) ?? places[0];
  const cards = visibleCards(view);
  const chosen: VisibleCard | undefined = cards.find((card) => card.instanceId === target);
  const seatOf = (seat: Seat): PlayerId => (seat === "you" ? view.viewer : opponentOf(view.viewer));

  const healthValue = whole(health);
  const armorValue = whole(armor);
  const currentValue = whole(current);
  const maxValue = whole(max);
  const drawValue = whole(drawCount);

  if (!open) {
    return (
      <button type="button" className="sandbox-toggle" data-testid={practiceTestid.sandboxToggle} onClick={() => setOpen(true)}>
        {t("Dev tools")}
      </button>
    );
  }

  return (
    <aside className="sandbox-panel" data-testid={practiceTestid.sandboxPanel} aria-labelledby={`${baseId}-title`}>
      <header className="sandbox-panel__head">
        <h2 className="sandbox-panel__title" id={`${baseId}-title`}>
          {t("Dev tools")}
        </h2>
        <button type="button" className="sandbox-panel__close" data-testid={practiceTestid.sandboxToggle} onClick={() => setOpen(false)}>
          {t("Hide")}
        </button>
      </header>

      <label className="sandbox-panel__check">
        <input type="checkbox" data-testid={practiceTestid.sandboxHold} checked={held} onChange={(event) => onHold(event.currentTarget.checked)} />
        {t("Hold the AI's turn")}
      </label>

      <section className="sandbox-panel__section" aria-labelledby={`${baseId}-find`}>
        <h3 className="sandbox-panel__heading" id={`${baseId}-find`}>
          {t("Find a card")}
        </h3>
        <input
          type="search"
          className="sandbox-panel__input"
          data-testid={practiceTestid.sandboxSearch}
          placeholder={t("Name, text, id, set, type, cost or rarity")}
          aria-label={t("Find a card")}
          value={query}
          onChange={(event) => setQuery(event.currentTarget.value)}
        />
        <label className="sandbox-panel__check">
          <input type="checkbox" data-testid={practiceTestid.sandboxRadiant} checked={radiant} onChange={(event) => setRadiant(event.currentTarget.checked)} />
          {t("Radiant")}
        </label>
        {defs === null ? (
          <p className="sandbox-panel__note">{t("Loading the cards…")}</p>
        ) : results.length === 0 ? (
          <p className="sandbox-panel__note">{t("No card matches.")}</p>
        ) : (
          <ul className="sandbox-panel__results">
            {results.map((def) => (
              <ResultCard key={def.id} def={def} radiant={radiant} picked={picked === def.id} onPick={() => setPicked(def.id)} />
            ))}
          </ul>
        )}
      </section>

      <section className="sandbox-panel__section" aria-labelledby={`${baseId}-spawn`}>
        <h3 className="sandbox-panel__heading" id={`${baseId}-spawn`}>
          {picked === null ? t("Spawn a card") : t("Spawn {name}", { name: nameOf(defs, view, picked) })}
        </h3>
        <div className="sandbox-panel__row">
          <SeatPicker value={spawnSeat} onChange={setSpawnSeat} testid={practiceTestid.sandboxSpawnSeat} />
          <select
            className="sandbox-panel__select"
            data-testid={practiceTestid.sandboxSpawnWhere}
            aria-label={t("Where")}
            value={place?.value ?? "hand"}
            onChange={(event) => setWhere(event.currentTarget.value)}
          >
            {places.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <button
            type="button"
            className="sandbox-panel__button"
            data-testid={practiceTestid.sandboxSpawn}
            disabled={picked === null || place === undefined}
            onClick={() => {
              if (picked === null || place === undefined) return;
              onTool({
                kind: "spawn",
                player: seatOf(spawnSeat),
                defId: picked,
                ...(radiant ? { radiant: true } : {}),
                to: place.zone,
              });
            }}
          >
            {t("Spawn")}
          </button>
        </div>
      </section>

      <section className="sandbox-panel__section" aria-labelledby={`${baseId}-hero`}>
        <h3 className="sandbox-panel__heading" id={`${baseId}-hero`}>
          {t("Hero")}
        </h3>
        <div className="sandbox-panel__row">
          <SeatPicker value={heroSeat} onChange={setHeroSeat} testid={practiceTestid.sandboxHeroSeat} />
          <input
            type="number"
            inputMode="numeric"
            className="sandbox-panel__number"
            data-testid={practiceTestid.sandboxHealth}
            aria-label={t("Health")}
            placeholder={t("Health")}
            min={1}
            max={SANDBOX_HERO_HEALTH_MAX}
            value={health}
            onChange={(event) => setHealth(event.currentTarget.value)}
          />
          <input
            type="number"
            inputMode="numeric"
            className="sandbox-panel__number"
            data-testid={practiceTestid.sandboxArmor}
            aria-label={t("Armor")}
            placeholder={t("Armor")}
            min={0}
            max={SANDBOX_ARMOR_MAX}
            value={armor}
            onChange={(event) => setArmor(event.currentTarget.value)}
          />
          <button
            type="button"
            className="sandbox-panel__button"
            data-testid={practiceTestid.sandboxSetHero}
            disabled={healthValue === null && armorValue === null}
            onClick={() =>
              onTool({
                kind: "setHero",
                player: seatOf(heroSeat),
                ...(healthValue === null ? {} : { health: healthValue }),
                ...(armorValue === null ? {} : { armor: armorValue }),
              })
            }
          >
            {t("Apply")}
          </button>
        </div>
      </section>

      <section className="sandbox-panel__section" aria-labelledby={`${baseId}-mana`}>
        <h3 className="sandbox-panel__heading" id={`${baseId}-mana`}>
          {t("Mana")}
        </h3>
        <div className="sandbox-panel__row">
          <SeatPicker value={manaSeat} onChange={setManaSeat} testid={practiceTestid.sandboxManaSeat} />
          <input
            type="number"
            inputMode="numeric"
            className="sandbox-panel__number"
            data-testid={practiceTestid.sandboxManaCurrent}
            aria-label={t("Current")}
            placeholder={t("Current")}
            min={0}
            max={SANDBOX_MANA_MAX}
            value={current}
            onChange={(event) => setCurrent(event.currentTarget.value)}
          />
          <input
            type="number"
            inputMode="numeric"
            className="sandbox-panel__number"
            data-testid={practiceTestid.sandboxManaMax}
            aria-label={t("Maximum")}
            placeholder={t("Maximum")}
            min={0}
            max={SANDBOX_MANA_MAX}
            value={max}
            onChange={(event) => setMax(event.currentTarget.value)}
          />
          <button
            type="button"
            className="sandbox-panel__button"
            data-testid={practiceTestid.sandboxSetMana}
            disabled={currentValue === null && maxValue === null}
            onClick={() =>
              onTool({
                kind: "setMana",
                player: seatOf(manaSeat),
                ...(currentValue === null ? {} : { current: currentValue }),
                ...(maxValue === null ? {} : { max: maxValue }),
              })
            }
          >
            {t("Apply")}
          </button>
        </div>
      </section>

      <section className="sandbox-panel__section" aria-labelledby={`${baseId}-draw`}>
        <h3 className="sandbox-panel__heading" id={`${baseId}-draw`}>
          {t("Draw cards")}
        </h3>
        <div className="sandbox-panel__row">
          <SeatPicker value={drawSeat} onChange={setDrawSeat} testid={practiceTestid.sandboxDrawSeat} />
          <input
            type="number"
            inputMode="numeric"
            className="sandbox-panel__number"
            data-testid={practiceTestid.sandboxDrawCount}
            aria-label={t("Cards")}
            min={1}
            max={SANDBOX_DRAW_MAX}
            value={drawCount}
            onChange={(event) => setDrawCount(event.currentTarget.value)}
          />
          <button
            type="button"
            className="sandbox-panel__button"
            data-testid={practiceTestid.sandboxDraw}
            disabled={drawValue === null}
            onClick={() => {
              if (drawValue !== null) onTool({ kind: "draw", player: seatOf(drawSeat), count: drawValue });
            }}
          >
            {t("Draw cards")}
          </button>
        </div>
      </section>

      <section className="sandbox-panel__section" aria-labelledby={`${baseId}-remove`}>
        <h3 className="sandbox-panel__heading" id={`${baseId}-remove`}>
          {t("Destroy or remove")}
        </h3>
        <div className="sandbox-panel__row">
          <select
            className="sandbox-panel__select sandbox-panel__select--wide"
            data-testid={practiceTestid.sandboxTarget}
            aria-label={t("A card you can see")}
            value={chosen?.instanceId ?? ""}
            onChange={(event) => setTarget(event.currentTarget.value)}
          >
            <option value="">{t("A card you can see")}</option>
            {cards.map((card) => (
              <option key={card.instanceId} value={card.instanceId}>
                {t("{name} ({side}, {where})", {
                  name: nameOf(defs, view, card.defId),
                  side: card.side === "you" ? t("yours") : t("the AI's"),
                  where: card.where === "field" ? t("field") : card.where === "hand" ? t("hand") : t("graveyard"),
                })}
              </option>
            ))}
          </select>
          <button
            type="button"
            className="sandbox-panel__button"
            data-testid={practiceTestid.sandboxDestroy}
            disabled={chosen?.where !== "field"}
            onClick={() => {
              if (chosen !== undefined) onTool({ kind: "destroy", instanceId: chosen.instanceId });
            }}
          >
            {t("Destroy")}
          </button>
          <button
            type="button"
            className="sandbox-panel__button"
            data-testid={practiceTestid.sandboxRemove}
            disabled={chosen === undefined}
            onClick={() => {
              if (chosen !== undefined) onTool({ kind: "remove", instanceId: chosen.instanceId });
            }}
          >
            {t("Remove")}
          </button>
        </div>
      </section>
    </aside>
  );
}
