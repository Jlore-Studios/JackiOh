// R2784000: `/dev/fx`, the motif gallery and the review surface of every card-animation pass
// (docs/polish/9-card-animations.md). Over the fixture board it plays any drawn motif in any flavor's
// palette or sub-look at any gallery scale, through a director of its own as the stat showcase's, and
// any card's table entry as that card's play through the real board's effects layer, or as the fixture
// set piece (R2785000), its play holding the board for the set piece's beat until a tap, a click or
// Space. R2794000: it lists every card whose entry plays something: a play's motif, a hook's motif
// (through its own director, as no play opens for a hook), a Flourish or a marquee entrance. Dev only:
// chunks.tsx loads it behind DEV_ONLY, so a production build leaves it out and serves a 404 here, as at
// `/dev/hotseat`.

import { useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type ReactElement } from "react";

import type { PlayerView } from "@jackioh/shared";

import { boardShakeSink, resolveAnchor } from "../../fx/anchors.ts";
import { FX_CARD_TABLE } from "../../fx/cards/tables.ts";
import { FX_DEFAULT_SEED, FX_GALLERY_SCALES, FX_INTENSITY_SCALE } from "../../fx/constants.ts";
import { capacityFor, createFxDirector, type FxDirector } from "../../fx/director.ts";
import { FX_PALETTES, isPalette, type FxPalette } from "../../fx/flavors.ts";
import { defaultFrames, defaultNow, defaultVisibility } from "../../fx/FxLayer.tsx";
import {
  galleryCardOf,
  galleryCues,
  galleryHookCues,
  galleryLabel,
  galleryPlayEvents,
  galleryPlayId,
  gallerySetPiece,
  type GalleryCard,
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
} as const;

/** R2794000: every card whose table entry plays something in the gallery, with its name and label. */
export function galleryCards(): (GalleryCard & { name: string; label: string })[] {
  return [...FX_CARD_TABLE].flatMap(([defId, entry]) => {
    const card = galleryCardOf(defId, entry);
    return card === null ? [] : [{ ...card, name: CATALOG[defId]?.name ?? defId, label: galleryLabel(card) }];
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
  const [settings] = useFxSettings();
  const [motif, setMotif] = useState<DrawnMotif>(FX_DRAWN_MOTIFS[0]);
  const [palette, setPalette] = useState<FxPalette>("Fire");
  const [scale, setScale] = useState<number>(1);
  const [defId, setDefId] = useState<string>(cards[0]?.defId ?? "");
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
  const playCard = (asSetPiece: boolean): void => {
    const card = cards.find((candidate) => candidate.defId === defId);
    if (card === undefined) return;
    if (card.kind === "hook") {
      director.current?.play(galleryHookCues(card, base, intensity, settings.speed));
      return;
    }
    plays.current += 1;
    setSetPieceId(asSetPiece ? galleryPlayId(plays.current) : null);
    setShown(withEvents(base, galleryPlayEvents(card, base, plays.current)));
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
                  {card.name} ({card.label})
                </option>
              ))}
            </select>
          </label>
          <button type="button" data-testid={fxGalleryTestid.playCard} onClick={() => playCard(false)}>
            Play card
          </button>
          <button type="button" data-testid={fxGalleryTestid.playSetPiece} onClick={() => playCard(true)}>
            Play as a set piece
          </button>
        </form>
      </div>
    </CatalogContext.Provider>
  );
}
