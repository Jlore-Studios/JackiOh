// R3349000 (part of #3330, blocked by #3345): every variant-level grant reveals the card wearing it.
// A duplicate's roll (§14.3), a cosmetic pack's slot (§14.6), a token redemption (§14.5) and a Chase
// Shard buy (§14.6) each show the card first in its previous look, then wipe into the new level with
// its name and rarity beneath it; several grants at once play in order, the rarest last.
//
// THE SERVER DECIDES (CLAUDE.md rule 7): `items` are the grants the server's answer already named,
// in the order the caller sorted them (`packs.ts::orderGrantsRarestLast`), and the previous look is
// the highest level the grant carries below it (`below`, R3015003), else the highest below the level
// in its ladder, else the normal face. The treatment follows the level's own rarity (R1333000): Rare
// and Epic play on the panel with a sound by rarity, Legendary dims the panel and grows the card in
// with light and its intro sting (R3332002), Mythic and a Legendary or Mythic chase take the screen in
// the full-screen view (R3345000). Reduced motion shows each reveal as a still before and after, and
// `bigMoments` short holds the full-screen ones briefly, off shows a still frame (R2785000). A tap,
// Enter, Space or Escape always moves on.

import { useEffect, useRef, useState, type CSSProperties, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import { CardFace, FullScreenArt, faceModel } from "../cards/index.ts";
import { useModalOverlay } from "../cards/inspect/store.ts";
import { readerName } from "../cards/chinese.ts";
import { useFxSettings } from "../fx/settings.ts";
import { reducedMotionNow } from "../game/animations.ts";
import { t } from "../i18n/i18n.ts";
import { useSetting } from "../settings/store.ts";
import { WhenEngineReady } from "../wasm/WhenEngineReady.tsx";
import { NO_FULL_ART_ID } from "../collection/collection.ts";
import { CosmeticLook } from "../collection/CosmeticLook.tsx";
import { ShownLevel } from "../collection/LookViews.tsx";
import { levelOf, levelsBelow } from "../cards/versions.ts";
import {
  GRANT_BEFORE_MS,
  GRANT_TAKEOVER_SHORT_MS,
  GRANT_WIPE_MS,
  playStinger,
  treatmentOf,
  type GrantTreatment,
} from "../packs/packs.ts";
import { COSMETICS, type CosmeticRarity } from "../wire/generated/cosmetics.ts";

import "./grantReveal.css";

/** One variant-level grant the server named: the level, the card wearing it (null for an account cosmetic), and its own rarity. */
export type GrantItem = {
  /** The granted level's registry id. */
  cosmeticId: string;
  /** The card a per-card cosmetic goes on, null for an account cosmetic. */
  cardId: string | null;
  /** The level's own rarity, never the card's (R1333000). */
  rarity: CosmeticRarity;
  /** Whether a Chase Shard bar bought it: a Legendary or Mythic chase takes the screen. */
  chase?: boolean;
  /** R3015003: the levels granted with it, lowest first, by an out-of-order pull; the previous look is the highest of these. */
  below?: string[];
  /** The level's name in the reader's language, from the caller's own registry; the wire name stands in when it is missing. */
  name?: string;
};

export const grantTestid = {
  scene: "grant-scene",
  item: (index: number): string => `grant-item-${String(index)}`,
  before: (index: number): string => `grant-before-${String(index)}`,
  after: (index: number): string => `grant-after-${String(index)}`,
  name: (index: number): string => `grant-name-${String(index)}`,
  skip: "grant-skip",
} as const;

/** The treatment an item of `rarity`, bought as `chase`, plays. */
export function grantTreatmentOf(item: GrantItem): GrantTreatment {
  return treatmentOf(item.rarity, item.chase ?? false);
}

/** The look the card wears before the grant: the highest level below it, else the normal face. */
export function previousLookOf(item: GrantItem): string | null {
  return item.below?.at(-1) ?? levelsBelow(item.cosmeticId).at(-1) ?? null;
}

/** The level's name in the reader's language, else its id. */
export function grantNameOf(item: GrantItem): string {
  if (item.name !== undefined) return item.name;
  const name = COSMETICS.find((cosmetic) => cosmetic.id === item.cosmeticId)?.name;
  return name === undefined ? item.cosmeticId : t(name);
}

function GrantedFace({ item, level }: { item: GrantItem; level: string | null }): ReactElement | null {
  if (item.cardId === null) {
    return level === null ? null : <CosmeticLook id={level} cardId={null} />;
  }
  const def = CATALOG[item.cardId];
  if (def === undefined) return null;
  // A granted level draws as the card wearing it; any other cosmetic draws its own look.
  if (level !== null && levelOf(level) === null) return <CosmeticLook id={level} cardId={item.cardId} />;
  const cardId = item.cardId;
  return (
    <WhenEngineReady>
      {() => (
        <ShownLevel defId={cardId} level={level ?? NO_FULL_ART_ID}>
          <CardFace face={faceModel({ defId: cardId, def: CATALOG[cardId], radiant: false })} />
        </ShownLevel>
      )}
    </WhenEngineReady>
  );
}

export default function GrantReveal({ items, onDone }: { items: readonly GrantItem[]; onDone?: () => void }): ReactElement | null {
  const [fx] = useFxSettings();
  const panelReduces = useSetting("reduceMotion");
  const reduced = panelReduces || reducedMotionNow(fx);
  const bigMoments = fx.bigMoments;
  const [index, setIndex] = useState(0);
  // An account cosmetic sits on no card, so it has no previous look: it starts revealed.
  const [stage, setStage] = useState<"before" | "after">(items[0]?.cardId == null ? "after" : "before");
  const sceneRef = useRef<HTMLDivElement>(null);
  const callbacks = useRef({ onDone });
  const stung = useRef<string | null>(null);
  useEffect(() => {
    callbacks.current = { onDone };
  });

  const item = items[index] ?? null;
  // The full-screen view needs the card: an account cosmetic grows in instead.
  const treatment = item === null ? "panel" : grantTreatmentOf(item) === "takeover" && item.cardId === null ? "grow" : grantTreatmentOf(item);
  // The full-screen view owns the moment: no before/after staging, a tap closes it.
  const takeover = treatment === "takeover" && item !== null && item.cardId !== null && !reduced && bigMoments !== "off";
  const shown = takeover ? "after" : stage;

  const advance = (): void => {
    if (!takeover && stage === "before") {
      setStage("after");
      return;
    }
    const next = items[index + 1];
    if (next !== undefined) {
      setIndex(index + 1);
      setStage(next.cardId == null ? "after" : "before");
      return;
    }
    callbacks.current.onDone?.();
  };
  const advanceRef = useRef(advance);
  useEffect(() => {
    advanceRef.current = advance;
  });
  // Escape moves on as a tap does.
  useModalOverlay(
    () => {
      advanceRef.current();
    },
    sceneRef,
    item !== null,
  );

  // One sting per grant, its own rarity's; the full-screen view plays its own intro instead.
  const stingKey = item === null ? null : `${item.cosmeticId}:${String(item.cardId ?? "")}`;
  useEffect(() => {
    if (item === null || takeover || stung.current === stingKey) return;
    stung.current = stingKey;
    playStinger(item.rarity);
  });

  // The previous look holds, then wipes into the new level; a short Big moment holds the takeover briefly.
  useEffect(() => {
    if (item === null || reduced || stage !== "before") return undefined;
    if (takeover && bigMoments !== "short") return undefined;
    const timer = window.setTimeout(
      () => {
        advanceRef.current();
      },
      takeover ? GRANT_TAKEOVER_SHORT_MS : GRANT_BEFORE_MS,
    );
    return () => {
      window.clearTimeout(timer);
    };
  }, [item, reduced, stage, takeover, bigMoments]);

  if (item === null) return null;
  const def = item.cardId === null ? undefined : CATALOG[item.cardId];
  const cardName = def === undefined ? null : readerName(def);
  const title = cardName === null ? t("New cosmetic") : t("New cosmetic for {card}", { card: cardName });
  const style = {
    "--grant-wipe": `${String(GRANT_WIPE_MS)}ms`,
  } as CSSProperties;
  return (
    <div
      className="grant-scene"
      data-testid={grantTestid.scene}
      data-treatment={treatment}
      data-motion={reduced ? "reduce" : "full"}
      data-stage={shown}
      role="dialog"
      aria-modal="true"
      aria-label={title}
      ref={sceneRef}
      tabIndex={-1}
      style={style}
      onClick={advance}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          advance();
        }
      }}
    >
      {takeover && item.cardId !== null ? (
        <>
          <FullScreenArt defId={item.cardId} name={cardName ?? item.cardId} onClose={advance} />
          <p className="grant-name grant-name--over" data-testid={grantTestid.name(index)}>
            {grantNameOf(item)} · {t(item.rarity)}
            {item.chase === true ? ` · ${t("Chase")}` : null}
          </p>
        </>
      ) : (
        <div
          key={index}
          className="grant-item"
          data-testid={grantTestid.item(index)}
          data-stage={shown}
          data-rarity={item.rarity}
          data-chase={item.chase === true ? "true" : undefined}
        >
          {shown === "before" ? (
            <div key="before" className="grant-before" data-testid={grantTestid.before(index)}>
              <GrantedFace item={item} level={previousLookOf(item)} />
            </div>
          ) : (
            <div key="after" className="grant-after grant-wipe" data-testid={grantTestid.after(index)}>
              <GrantedFace item={item} level={item.cosmeticId} />
            </div>
          )}
          <p className="grant-name" data-testid={grantTestid.name(index)}>
            {grantNameOf(item)} · {t(item.rarity)}
            {item.chase === true ? ` · ${t("Chase")}` : null}
          </p>
        </div>
      )}
      <div className="grant-actions">
        <button
          type="button"
          data-testid={grantTestid.skip}
          onClick={(event) => {
            event.stopPropagation();
            callbacks.current.onDone?.();
          }}
        >
          {t("Skip")}
        </button>
      </div>
    </div>
  );
}
