// R2784000: `/dev/fx`, the motif gallery and the review surface of every card-animation pass
// (docs/polish/9-card-animations.md). Over the fixture board it plays any drawn motif in any flavor's
// palette or sub-look at any gallery scale, through a director of its own as the stat showcase's, and
// any card's table entry as that card's play through the real board's effects layer (#2793: a motif,
// a recipe, a set piece or a Field card's aura, the card standing in the backrow; R2797000: a `death`,
// `trigger` or `activate` motif from its hook's marker under the play; R2794000: a marquee Unit's
// entrance as it lands), or as the fixture set piece (R2785000), its play holding the board for the set
// piece's beat until a tap, a click or Space. R3332000: it lists every card's signature by set, for a
// person to review each one; R3332001 a signature whose entrance reads the board plays over its own
// (`galleryBoardOf`: Felinor Fiender's over three Felinors). Dev only: chunks.tsx loads it behind
// DEV_ONLY, so a production build leaves it out and serves a 404 here, as at `/dev/hotseat`.
// R3346000: it plays every play impact, each type at each cost tier, with the board shake a heavy or
// massive one brings, and a trap's set click and snap, side by side for review by ear.

import { useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type ReactElement } from "react";
import { flushSync } from "react-dom";

import type { PlayerView } from "@jackioh/shared";

import { playImpactCue } from "../../audio/cues.ts";
import { PLAY_IMPACT_OPPONENT_GAIN } from "../../audio/constants.ts";
import { getAudioEngine } from "../../audio/engine.ts";
import type { PlayTier, PlayType } from "../../game/damageFeel.ts";
import { PLAY_TIERS_ASCENDING } from "../../game/damageFeel.ts";
import { boardShakeSink, resolveAnchor } from "../../fx/anchors.ts";
import { FX_CARD_FILES, FX_CARD_TABLE, isSignature } from "../../fx/cards/tables.ts";
import { FX_DEFAULT_SEED, FX_GALLERY_SCALES, FX_INTENSITY_SCALE } from "../../fx/constants.ts";
import { capacityFor, createFxDirector, type FxDirector } from "../../fx/director.ts";
import { FX_PALETTES, isPalette, type FxPalette } from "../../fx/flavors.ts";
import { defaultFrames, defaultNow, defaultVisibility } from "../../fx/FxLayer.tsx";
import { planFx, playTrauma } from "../../fx/cues.ts";
import { createFxMemory } from "../../fx/memory.ts";
import {
  SYSTEM_MOMENTS,
  galleryBoardOf,
  galleryCardEvents,
  galleryCues,
  galleryEntry,
  galleryFieldView,
  galleryPlayId,
  galleryPlayOf,
  gallerySetPiece,
  systemMomentFixture,
  type GalleryCardPlay,
  type SystemMoment,
} from "../../fx/motifs/gallery.ts";
import { FX_DRAWN_MOTIFS, isDrawnMotif, type DrawnMotif } from "../../fx/motifs/names.ts";
import { useFxSettings } from "../../fx/settings.ts";
import { createSurface } from "../../fx/surface.ts";
import { reducedMotionNow } from "../../game/animations.ts";
import { CatalogContext, lookupFromDefs } from "../../game/catalog.ts";
import Game from "../../game/Game.tsx";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import "../../fx/fx.css";

export const fxGalleryTestid = {
  root: "fx-gallery",
  motif: "fx-gallery-motif",
  palette: "fx-gallery-palette",
  scale: "fx-gallery-scale",
  card: "fx-gallery-card",
  play: "fx-gallery-play",
  playCard: "fx-gallery-play-card",
  playSetPiece: "fx-gallery-play-set-piece",
  signature: "fx-gallery-signature",
  playSignature: "fx-gallery-play-signature",
  moment: "fx-gallery-moment",
  playMoment: "fx-gallery-play-moment",
  impact: "fx-gallery-impact",
  impactOpponent: "fx-gallery-impact-opponent",
  trapSet: "fx-gallery-trap-set",
  trapSnap: "fx-gallery-trap-snap",
} as const;

/** R3346000: the play types the gallery plays an impact for, each with how the panel labels it. */
const IMPACT_TYPES: readonly { type: PlayType; label: string }[] = [
  { type: "unit", label: "Unit" },
  { type: "spell", label: "Spell" },
  { type: "field", label: "Field Spell" },
];

function isSystemMomentName(name: string): name is SystemMoment {
  return (SYSTEM_MOMENTS as readonly string[]).includes(name);
}

/** What a card's option says it plays: its motif, its recipe or entrance (R2794000), a set piece or its aura. */
function playLabel(play: GalleryCardPlay): string {
  switch (play.kind) {
    case "board":
      return play.motif;
    case "hook":
      return `${play.hook} ${play.motif}`;
    case "recipe":
    case "entrance":
      return play.key;
    case "setPiece":
      return "set piece";
    case "aura":
      return "aura";
  }
}

type GalleryCard = { defId: string; name: string; label: string; play: GalleryCardPlay };

/** #2793: every card whose table entry the gallery can play, in table order, with how it plays it. */
export function galleryCards(): GalleryCard[] {
  return [...FX_CARD_TABLE].flatMap(([defId, entry]) => {
    const play = galleryPlayOf(defId, entry);
    if (play === null) return [];
    const name = CATALOG[defId]?.name ?? defId;
    return [{ defId, name, label: `${name} (${playLabel(play)})`, play }];
  });
}

/** R3332000: every card whose entry is its signature, by set in the tables' order, each set's in table order; a set with none is left out. */
export function gallerySignatures(): { set: string; cards: GalleryCard[] }[] {
  const signed = galleryCards().filter((card) => isSignature(FX_CARD_TABLE.get(card.defId)));
  const sets = [...new Set(FX_CARD_FILES.map((file) => file.set))];
  return sets.flatMap((set) => {
    const cards = signed.filter((card) => CATALOG[card.defId]?.set === set);
    return cards.length === 0 ? [] : [{ set, cards }];
  });
}

const PANEL: CSSProperties = {
  position: "fixed",
  top: 8,
  right: 8,
  zIndex: 60,
  display: "grid",
  gap: 6,
  padding: 10,
  borderRadius: 8,
  background: "rgba(16, 18, 26, 0.92)",
  color: "#f4f1e8",
  font: "13px/1.3 system-ui, sans-serif",
};

const ROW: CSSProperties = { display: "flex", gap: 6, alignItems: "center", justifyContent: "space-between" };

const noop = (): void => undefined;

export default function FxGallery(): ReactElement {
  const base = useMemo(() => fullBoardView(), []);
  const lookup = useMemo(() => lookupFromDefs(CATALOG), []);
  const cards = useMemo(() => galleryCards(), []);
  const signatures = useMemo(() => gallerySignatures(), []);
  const [settings] = useFxSettings();
  const [motif, setMotif] = useState<DrawnMotif>(FX_DRAWN_MOTIFS[0]);
  const [palette, setPalette] = useState<FxPalette>("Fire");
  const [scale, setScale] = useState<number>(1);
  const [defId, setDefId] = useState<string>(cards[0]?.defId ?? "");
  const [signature, setSignature] = useState<string>(signatures[0]?.cards[0]?.defId ?? "");
  const [moment, setMoment] = useState<SystemMoment>(SYSTEM_MOMENTS[0] ?? "questProgressed");
  // R3346000: whether the impacts play as the other seat's, a step quieter.
  const [asOpponent, setAsOpponent] = useState(false);
  // Each card play is a new view whose events the board has not seen, so its effects layer plays them.
  const [shown, setShown] = useState<PlayerView>(base);
  const plays = useRef(0);
  // R2785000: the play the fixture set piece claims, the last one played as a set piece.
  const [setPieceId, setSetPieceId] = useState<string | null>(null);
  const setPieceOf = useMemo(() => (setPieceId === null ? undefined : gallerySetPiece(setPieceId)), [setPieceId]);

  const director = useRef<FxDirector | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const domRef = useRef<HTMLDivElement | null>(null);
  // The director lives exactly as long as the canvas and the DOM root it draws into (StatShowcase's).
  useLayoutEffect(() => {
    const surface = createSurface(canvasRef.current!);
    const created = createFxDirector({
      surface,
      domRoot: domRef.current!,
      now: defaultNow,
      frames: defaultFrames(),
      visibility: defaultVisibility(),
      measure: resolveAnchor,
      shakeSink: boardShakeSink(document),
      seed: FX_DEFAULT_SEED,
      capacity: capacityFor(window.innerWidth),
    });
    director.current = created;
    return () => {
      if (director.current === created) director.current = null;
      created.dispose();
      surface?.dispose();
    };
  }, []);

  // As the board's own layer: nothing with the effects off or under reduced motion.
  const intensity = reducedMotionNow(settings) ? 0 : FX_INTENSITY_SCALE[settings.intensity];
  const playMotif = (): void => {
    director.current?.play(galleryCues(motif, palette, scale, base, intensity, settings.speed));
  };
  const playCardOf = (id: string, asSetPiece: boolean): void => {
    const card = cards.find((candidate) => candidate.defId === id);
    if (card === undefined) return;
    plays.current += 1;
    setSetPieceId(asSetPiece ? galleryPlayId(plays.current) : null);
    // R3332001: a board of the entrance's own goes up first, with no new events, so the board plans the
    // play against it (Game plans against the view it still shows).
    const board = galleryBoardOf(card.defId, base);
    if (board !== base) flushSync(() => setShown(board));
    setShown(withEvents(galleryFieldView(card.defId, board), galleryCardEvents(card.defId, card.play, board, plays.current)));
  };
  // R2784000: every system moment through the real planner, on the gallery director.
  const playMoment = (): void => {
    const events = systemMomentFixture(moment, base);
    if (events.length === 0) return;
    const entry = galleryEntry(events, base, settings.speed);
    const memory = createFxMemory();
    memory.remember(entry.events);
    director.current?.play(planFx(entry, base, { intensity, card: () => undefined, memory }));
  };

  // R3346000: one impact as the board plays it (sfx.ts's recipe through the engine), and, for a heavy or
  // massive tier, the shake `planFx` gives it, under the same intensity.
  const playImpact = (tier: PlayTier, type: PlayType): void => {
    const cue = playImpactCue(tier, type, !asOpponent);
    getAudioEngine().playSfx(cue.id, cue.params, 0);
    const trauma = Math.min(1, playTrauma(tier) * intensity);
    if (trauma > 0) director.current?.play([{ kind: "shake", trauma, delayMs: 0 }]);
  };
  const playTrap = (id: "trapSet" | "trapSnap"): void => {
    getAudioEngine().playSfx(id, id === "trapSnap" && asOpponent ? { gain: PLAY_IMPACT_OPPONENT_GAIN } : undefined, 0);
  };

  return (
    <CatalogContext.Provider value={lookup}>
      <div className="app-shell app-shell--wide" data-testid={fxGalleryTestid.root}>
        <Game view={shown} legal={[]} onAction={noop} setPieceOf={setPieceOf} />
        <div className="fx-layer" data-fx="on" aria-hidden="true">
          <canvas ref={canvasRef} className="fx-canvas" />
          <div ref={domRef} className="fx-dom" />
        </div>
        <form
          style={PANEL}
          aria-label="Effects gallery"
          onSubmit={(event) => {
            event.preventDefault();
          }}
        >
          <label style={ROW}>
            Motif
            <select
              data-testid={fxGalleryTestid.motif}
              value={motif}
              onChange={(event) => {
                if (isDrawnMotif(event.target.value)) setMotif(event.target.value);
              }}
            >
              {FX_DRAWN_MOTIFS.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </select>
          </label>
          <label style={ROW}>
            Palette
            <select
              data-testid={fxGalleryTestid.palette}
              value={palette}
              onChange={(event) => {
                if (isPalette(event.target.value)) setPalette(event.target.value);
              }}
            >
              {FX_PALETTES.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </select>
          </label>
          <label style={ROW}>
            Scale
            <select data-testid={fxGalleryTestid.scale} value={String(scale)} onChange={(event) => setScale(Number(event.target.value))}>
              {FX_GALLERY_SCALES.map((value) => (
                <option key={value} value={String(value)}>
                  ×{value}
                </option>
              ))}
            </select>
          </label>
          <button type="button" data-testid={fxGalleryTestid.play} onClick={playMotif}>
            Play motif
          </button>
          <label style={ROW}>
            Card
            <select data-testid={fxGalleryTestid.card} value={defId} onChange={(event) => setDefId(event.target.value)}>
              {cards.map((card) => (
                <option key={card.defId} value={card.defId}>
                  {card.label}
                </option>
              ))}
            </select>
          </label>
          <button type="button" data-testid={fxGalleryTestid.playCard} onClick={() => playCardOf(defId, false)}>
            Play card
          </button>
          <button type="button" data-testid={fxGalleryTestid.playSetPiece} onClick={() => playCardOf(defId, true)}>
            Play as a set piece
          </button>
          <label style={ROW}>
            Signature
            <select data-testid={fxGalleryTestid.signature} value={signature} onChange={(event) => setSignature(event.target.value)}>
              {signatures.map((group) => (
                <optgroup key={group.set} label={group.set}>
                  {group.cards.map((card) => (
                    <option key={card.defId} value={card.defId}>
                      {card.label}
                    </option>
                  ))}
                </optgroup>
              ))}
            </select>
          </label>
          <button type="button" data-testid={fxGalleryTestid.playSignature} onClick={() => playCardOf(signature, false)}>
            Play signature
          </button>
          <label style={ROW}>
            Moment
            <select
              data-testid={fxGalleryTestid.moment}
              value={moment}
              onChange={(event) => {
                if (isSystemMomentName(event.target.value)) setMoment(event.target.value);
              }}
            >
              {SYSTEM_MOMENTS.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </select>
          </label>
          <button type="button" data-testid={fxGalleryTestid.playMoment} onClick={playMoment}>
            Play moment
          </button>
          <fieldset style={{ display: "grid", gap: 4 }}>
            <legend>Plays that land with weight</legend>
            <label style={ROW}>
              <input type="checkbox" data-testid={fxGalleryTestid.impactOpponent} checked={asOpponent} onChange={(event) => setAsOpponent(event.target.checked)} />
              As the opponent
            </label>
            {IMPACT_TYPES.map(({ type, label }) => (
              <div key={type} style={ROW}>
                {label}
                {PLAY_TIERS_ASCENDING.map((tier) => (
                  <button key={tier} type="button" data-testid={fxGalleryTestid.impact} data-type={type} data-tier={tier} onClick={() => playImpact(tier, type)}>
                    {tier}
                  </button>
                ))}
              </div>
            ))}
            <div style={ROW}>
              Trap
              <button type="button" data-testid={fxGalleryTestid.trapSet} onClick={() => playTrap("trapSet")}>
                Set click
              </button>
              <button type="button" data-testid={fxGalleryTestid.trapSnap} onClick={() => playTrap("trapSnap")}>
                Snap
              </button>
            </div>
          </fieldset>
        </form>
      </div>
    </CatalogContext.Provider>
  );
}
