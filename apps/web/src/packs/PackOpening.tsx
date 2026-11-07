// The pack-opening scene and the card packs panel (#1308, SPEC §14.3, §14.4), on the Shop screen;
// the odds and the pity counters are the Purchases page's (#1651).
//
// THE SERVER DECIDES (CLAUDE.md rule 7). `PackScene` never rolls: it draws the slots `POST
// /api/packs/open` already settled, and nothing on screen changes before that answer; skip-all, the
// holds and the flips only choose how fast it is shown, and reduced motion shows it at once. Open all
// is one `POST /api/packs/open-all` of up to `PACKS_OPEN_ALL_MAX` packs (R3212000), whose openings the
// scene then shows one pack at a time, sending nothing more. The pack
// is a `PackObject` (#1663); the token bars (`OpeningBars`, SPEC §14.5) play once every card shows
// (R1953000, R1999000). A new cosmetic offers Equip (R1955000), a portrait's none. The scene is a
// modal overlay (#1757).
//
// ONE SPEND AT A TIME (R2378000): while any spend of the tab is out (`net/spending.ts`), Open, Open
// all, the sealed pack, Reveal all and Next pack wait, disabled (`economy/spend.css`).

import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent,
  type ReactElement,
} from "react";
import { createPortal } from "react-dom";

import { CATALOG } from "@jackioh/cards";
import { PACKS_OPEN_ALL_MAX } from "@jackioh/server-config";

import { CardBack, CardFace, faceModel, useFullScreenTrigger } from "../cards/index.ts";
import { readerName } from "../cards/chinese.ts";
import { WhenEngineReady } from "../wasm/WhenEngineReady.tsx";
import { useModalOverlay } from "../cards/inspect/store.ts";
import { CosmeticLook, hasLook } from "../collection/CosmeticLook.tsx";
import EquipButton from "../collection/EquipButton.tsx";
import { flightsOf } from "../economy/bars.ts";
import OpeningBars from "../economy/OpeningBars.tsx";
import "../economy/spend.css";
import { EmptyState } from "../empty/EmptyState.tsx";
import { resolveAnchor } from "../fx/anchors.ts";
import { FX_DEFAULT_SEED, FX_INTENSITY_SCALE } from "../fx/constants.ts";
import { capacityFor, createFxDirector, type FxDirector } from "../fx/director.ts";
import { defaultFrames, defaultNow, defaultVisibility } from "../fx/FxLayer.tsx";
import { useFxSettings } from "../fx/settings.ts";
import { createSurface } from "../fx/surface.ts";
import type { FxShakeSink } from "../fx/types.ts";
import { reducedMotionNow } from "../game/animations.ts";
import { DialogClose } from "../game/Dialog.tsx";
import { t } from "../i18n/i18n.ts";
import {
  getPacks,
  keptPaths,
  openAllPacks,
  openPack,
  type BoughtPackResponse,
  type OpenedPackResponse,
  type PacksResponse,
} from "../net/api.ts";
import { cachedRead } from "../net/readCache.ts";
import { spendOutNow, useSpendOut } from "../net/spending.ts";
import { useSetting } from "../settings/store.ts";
import { withBought } from "../shop/shop.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import {
  PACK_FLIP_MS,
  PACK_HOLD_MS,
  PACK_SWIPE_PX,
  PACK_TEAR_MS,
  bestRarity,
  holdsToFlip,
  packName,
  packTestid,
  playStinger,
  revealCues,
  slotOutcome,
  withOpening,
} from "./packs.ts";
import PackObject from "./PackObject.tsx";
import { messageOf } from "../net/errors.ts";
import "./packs.css";

/** The scene shakes nothing: the pack is not the board. */
const NO_SHAKE: FxShakeSink = { apply: () => undefined, clear: () => undefined };

function PackSlotFace({ slot, look }: { slot: OpenedPackResponse["slots"][number]; look: string | null }): ReactElement {
  const def = CATALOG[slot.cardId];
  const fs = useFullScreenTrigger({
    defId: slot.cardId,
    rarity: slot.rarity,
    printedRarity: def?.printedRarity,
    name: def ? readerName(def) : slot.cardId,
  });
  return (
    <div className="pack-slot__face" {...fs.handlers}>
      {/* The shop never waits for the module (#1577), and a face is worked out by it (#778). */}
      <WhenEngineReady>
        {() =>
          look === null ? (
            <CardFace face={faceModel({ defId: slot.cardId, def: CATALOG[slot.cardId], radiant: false })} />
          ) : (
            <CosmeticLook id={look} cardId={slot.cardId} />
          )
        }
      </WhenEngineReady>
    </div>
  );
}

export type PackSceneProps = {
  /** The kind of pack being opened: a shipped set's name, or `global`. */
  pack: string;
  /** The server's answer once it has given one; the pack stays sealed until then. */
  opened: OpenedPackResponse | null;
  /** An open request is out. */
  busy: boolean;
  /** The server's words for a refused open. */
  error: string | null;
  /** The packs still to open after this one in the batch. */
  left: number;
  onTear: () => void;
  onNext: () => void;
  onClose: () => void;
};

export function PackScene({ pack, opened, busy, error, left, onTear, onNext, onClose }: PackSceneProps): ReactElement {
  const [fx] = useFxSettings();
  const panelReduces = useSetting("reduceMotion");
  const reduced = panelReduces || reducedMotionNow(fx);
  const fxOn = !reduced && fx.intensity !== "off";
  const intensity: number = FX_INTENSITY_SCALE[fx.intensity];

  const [torn, setTorn] = useState(false);
  const [skipped, setSkipped] = useState(false);
  const [flipped, setFlipped] = useState<ReadonlySet<number>>(new Set());
  const [holding, setHolding] = useState<number | null>(null);
  const [pressing, setPressing] = useState(false);
  const spendOut = useSpendOut();

  const holdTimer = useRef<number | null>(null);
  const swipeFrom = useRef<number | null>(null);
  const stung = useRef(false);
  const director = useRef<FxDirector | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const domRef = useRef<HTMLDivElement | null>(null);
  const sceneRef = useRef<HTMLDivElement | null>(null);

  // Escape closes as the ✕ does: not while busy.
  useModalOverlay(onClose, sceneRef, !busy);

  // The director lives exactly as long as the canvas and the DOM root it draws into (FxLayer's).
  useLayoutEffect(() => {
    if (!fxOn) return undefined;
    const surface = createSurface(canvasRef.current!);
    const created = createFxDirector({
      surface,
      domRoot: domRef.current!,
      now: defaultNow,
      frames: defaultFrames(),
      visibility: defaultVisibility(),
      measure: resolveAnchor,
      shakeSink: NO_SHAKE,
      seed: FX_DEFAULT_SEED,
      capacity: capacityFor(window.innerWidth),
    });
    director.current = created;
    return () => {
      if (director.current === created) director.current = null;
      created.dispose();
      surface?.dispose();
    };
  }, [fxOn]);

  // The pack tears for `PACK_TEAR_MS` once the server has answered, and only then shows its cards.
  useEffect(() => {
    if (opened === null || reduced) return undefined;
    const timer = window.setTimeout(() => {
      setTorn(true);
    }, PACK_TEAR_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [opened, reduced]);

  // Every card at once is one sting, the best card's.
  useEffect(() => {
    if (opened === null || !(reduced || skipped) || stung.current) return;
    stung.current = true;
    playStinger(bestRarity(opened.slots));
  }, [opened, reduced, skipped]);

  // A refused open leaves the pack sealed, so a later skip-all is asked for again.
  useEffect(() => {
    if (error !== null) setSkipped(false);
  }, [error]);

  useEffect(
    () => () => {
      if (holdTimer.current !== null) window.clearTimeout(holdTimer.current);
    },
    [],
  );

  const slots = opened?.slots ?? [];
  const shows = (index: number): boolean => reduced || skipped || flipped.has(index);
  const all = opened !== null && slots.every((_, index) => shows(index));
  const showSlots = opened !== null && (reduced || skipped || torn);

  const reveal = (index: number): void => {
    const slot = slots[index];
    if (slot === undefined) return;
    setFlipped((before) => new Set(before).add(index));
    playStinger(slot.rarity);
    director.current?.play(revealCues(slot.rarity, packTestid.slot(index), intensity));
  };

  const cancelHold = (): void => {
    if (holdTimer.current !== null) window.clearTimeout(holdTimer.current);
    holdTimer.current = null;
    setHolding(null);
  };

  const startHold = (index: number): void => {
    cancelHold();
    setHolding(index);
    holdTimer.current = window.setTimeout(() => {
      holdTimer.current = null;
      setHolding(null);
      reveal(index);
    }, PACK_HOLD_MS);
  };

  const skipAll = (): void => {
    cancelHold();
    setSkipped(true);
    if (opened === null) onTear();
  };

  const onPackDown = (event: PointerEvent<HTMLButtonElement>): void => {
    swipeFrom.current = event.clientX;
    setPressing(true);
    if (typeof event.currentTarget.setPointerCapture === "function") event.currentTarget.setPointerCapture(event.pointerId);
  };

  const onPackUp = (event: PointerEvent<HTMLButtonElement>): void => {
    const from = swipeFrom.current;
    swipeFrom.current = null;
    setPressing(false);
    if (from !== null && Math.abs(event.clientX - from) >= PACK_SWIPE_PX) onTear();
  };

  const onPackCancel = (): void => {
    swipeFrom.current = null;
    setPressing(false);
  };

  const title = t("Opening {pack}", { pack: packName(pack) });
  const style = {
    "--pack-tear-ms": PACK_TEAR_MS,
    "--pack-hold-ms": PACK_HOLD_MS,
    "--pack-flip-ms": PACK_FLIP_MS,
  } as CSSProperties;

  return createPortal(
    <div
      className="pack-scene"
      data-testid={packTestid.scene}
      data-motion={reduced ? "reduce" : "full"}
      data-hud={opened === null ? undefined : "true"}
      role="dialog"
      aria-modal="true"
      aria-label={title}
      ref={sceneRef}
      tabIndex={-1}
      style={style}
    >
      <h2 className="pack-scene__title">{title}</h2>

      {showSlots ? null : (
        <button
          type="button"
          className="pack-hit"
          data-testid={packTestid.pack}
          data-state={opened === null ? "sealed" : "tearing"}
          data-best={opened === null ? undefined : bestRarity(opened.slots)}
          disabled={busy || spendOut || opened !== null}
          onClick={onTear}
          onPointerDown={onPackDown}
          onPointerUp={onPackUp}
          onPointerCancel={onPackCancel}
        >
          <PackObject
            pack={pack}
            size="scene"
            state={opened !== null ? "tearing" : pressing ? "pressed" : "idle"}
            testid={packTestid.object}
          >
            <span className="pack__hint">{busy || opened !== null ? t("Opening…") : t("Swipe or tap to open")}</span>
          </PackObject>
        </button>
      )}

      {showSlots && opened !== null ? (
        <div className="pack-slots" data-testid={packTestid.slots}>
          {opened.slots.map((slot, index) => {
            const hold = holdsToFlip(slot.rarity);
            const shown = shows(index);
            // R1827000, R1833000: a dropped cosmetic shows its look in the card's place: a portrait's
            // art, a playmat's or backdrop's mini board, a card back, a per-card one on its card.
            const cosmetic = slot.grant.kind === "cosmetic" ? slot.grant : null;
            const look = cosmetic !== null && hasLook(cosmetic.cosmeticId, slot.cardId) ? cosmetic.cosmeticId : null;
            return (
              <div
                key={index}
                className="pack-slot"
                data-testid={packTestid.slot(index)}
                data-rarity={slot.rarity}
                data-grant={slot.grant.kind}
                data-cosmetic-rarity={cosmetic?.cosmeticRarity}
                data-revealed={shown ? "true" : "false"}
                data-holding={holding === index ? "true" : undefined}
              >
                {shown ? (
                  <>
                    <PackSlotFace slot={slot} look={look} />
                    <p className="pack-slot__outcome" data-testid={packTestid.outcome(index)}>
                      {slotOutcome(slot, opened.bars.slotPoints[index] ?? 0)}
                    </p>
                    {/* R3015003: an out-of-order pull granted the levels below it too. */}
                    {cosmetic !== null && (cosmetic.below?.length ?? 0) > 0 ? (
                      <p className="pack-slot__below" data-testid={packTestid.below(index)}>
                        {t("and every level below it")}
                      </p>
                    ) : null}
                    {/* R1833000: a cosmetic's own rarity, never its card's (R1333000), a chase marked. */}
                    {cosmetic === null ? null : (
                      <p className="pack-slot__cosmetic-rarity">
                        {t(cosmetic.cosmeticRarity)}
                        {COSMETICS.some((entry) => entry.id === cosmetic.cosmeticId && entry.chase) ? ` · ${t("Chase")}` : null}
                      </p>
                    )}
                    {cosmetic === null ? null : (
                      <EquipButton cosmeticId={cosmetic.cosmeticId} cardId={slot.cardId} testid={packTestid.equip(index)} />
                    )}
                  </>
                ) : (
                  <button
                    type="button"
                    className="pack-slot__back"
                    data-testid={packTestid.flip(index)}
                    aria-label={hold ? t("Hold to reveal card {n}", { n: index + 1 }) : t("Reveal card {n}", { n: index + 1 })}
                    onClick={(event) => {
                      // A hold turns the card; a keyboard press (a click with no pointer) cannot hold, so it turns it too.
                      if (hold && event.detail !== 0) return;
                      reveal(index);
                    }}
                    onPointerDown={hold ? () => { startHold(index); } : undefined}
                    onPointerUp={hold ? cancelHold : undefined}
                    onPointerLeave={hold ? cancelHold : undefined}
                    onPointerCancel={hold ? cancelHold : undefined}
                  >
                    <CardBack />
                  </button>
                )}
              </div>
            );
          })}
        </div>
      ) : null}

      {error === null ? null : (
        <p className="notice" role="alert" data-testid={packTestid.error}>
          {t(error)}
        </p>
      )}

      {opened !== null ? (
        <OpeningBars
          bars={opened.bars}
          flights={flightsOf(opened.bars, opened.slots.map((slot) => slot.rarity), packTestid.slot)}
          play={all}
          testid={packTestid.bars}
        />
      ) : null}

      <div className="pack-actions">
        {all ? null : (
          <button type="button" data-testid={packTestid.skipAll} disabled={busy || (spendOut && opened === null)} onClick={skipAll}>
            {t("Reveal all")}
          </button>
        )}
        {all && left > 0 ? (
          <button type="button" data-testid={packTestid.next} disabled={spendOut} onClick={onNext}>
            {t("Next pack ({left} left)", { left })}
          </button>
        ) : null}
      </div>

      {fxOn ? (
        <div className="fx-layer pack-fx" data-testid={packTestid.fx} data-fx="on" aria-hidden="true">
          <canvas ref={canvasRef} className="fx-canvas" />
          <div ref={domRef} className="fx-dom" />
        </div>
      ) : null}

      {/* R3221000: the scene keeps its own body and closes by the one ✕ every dialog shows, not while busy. */}
      <DialogClose testid={packTestid.close} disabled={busy} onClick={onClose} />
    </div>,
    document.body,
  );
}

/** `all`: the scene was begun by Open all, whose first tear opens every pack of it at once (R3212000). */
type SceneState = { pack: string; left: number; all: boolean };

/**
 * The panel. `revision` is how many times the page has asked it to read the packs again; `bought` is the
 * last pack the page bought, whose count the panel takes with no read (R3211000). `onChanged` hands the
 * page each opening's answer, and `onClosed` tells it the scene closed after at least one opening, the
 * one moment it reads what the answers did not carry.
 */
export default function PackOpening({
  token,
  revision = 0,
  bought,
  onChanged,
  onClosed,
}: {
  token: string;
  revision?: number;
  bought?: BoughtPackResponse | null;
  onChanged?: (opened: OpenedPackResponse) => void;
  onClosed?: () => void;
}): ReactElement {
  // R2125000: a revisit starts on the packs the last visit kept and reads again behind them.
  const [info, setInfo] = useState<PacksResponse | null>(
    () => cachedRead<PacksResponse>(token, keptPaths.packs) ?? null,
  );
  const [loadError, setLoadError] = useState<string | null>(null);
  const [scene, setScene] = useState<SceneState | null>(null);
  const [round, setRound] = useState(0);
  const [opened, setOpened] = useState<OpenedPackResponse | null>(null);
  const [openError, setOpenError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const [reading, setReading] = useState(true);
  // A swipe's pointerup is followed by a click, and both ask to open: the ref answers the second at once.
  const inFlight = useRef(false);
  // Whether the scene has opened a pack since it began, so closing it tells the page once (R3211000).
  const spent = useRef(false);
  // R3212000: the openings an Open all answered that the scene has not shown yet, in order.
  const queued = useRef<OpenedPackResponse[]>([]);
  const spendOut = useSpendOut();

  useEffect(() => {
    let canceled = false;
    setReading(true);
    getPacks(token).then(
      (response) => {
        if (!canceled) {
          setInfo(response);
          setLoadError(null);
          setReading(false);
        }
      },
      (cause: unknown) => {
        if (!canceled) {
          setReading(false);
          // R2125000: a read that fails behind kept packs leaves them and says nothing; only a first visit says why.
          const kept = cachedRead<PacksResponse>(token, keptPaths.packs);
          if (kept === undefined) {
            setLoadError(messageOf(cause));
          } else {
            setInfo(kept);
            setLoadError(null);
          }
        }
      },
    );
    return () => {
      canceled = true;
    };
  }, [token, revision, attempt]);

  // R3211000: a buy's answer holds the count it left; the panel shows it with no read.
  useEffect(() => {
    if (bought === undefined || bought === null) return;
    setInfo((before) => (before === null ? before : { ...before, unopened: withBought(before.unopened, bought) }));
  }, [bought]);

  const begin = (next: SceneState | null): void => {
    setScene(next);
    setOpened(null);
    setOpenError(null);
  };

  const start = (pack: string, count: number): void => {
    queued.current = [];
    begin({ pack, left: count - 1, all: count > 1 });
    setRound((before) => before + 1);
  };

  const tear = (): void => {
    if (scene === null || inFlight.current || opened !== null) return;
    // R3212000: a pack an Open all already opened shows with no request.
    const shown = queued.current.shift();
    if (shown !== undefined) {
      setOpened(shown);
      return;
    }
    if (spendOutNow()) return;
    inFlight.current = true;
    setBusy(true);
    setOpenError(null);
    const opening = scene.all
      ? openAllPacks(token, scene.pack, scene.left + 1).then(({ openings }) => openings)
      : openPack(token, scene.pack).then((one) => [one]);
    opening
      .then(
        ([first, ...rest]) => {
          const last = rest.at(-1) ?? first;
          if (first === undefined || last === undefined) return;
          queued.current = rest;
          setOpened(first);
          // The last opening's counters are the account's; the panel and the page take them once.
          setInfo((before) => (before === null ? before : withOpening(before, last)));
          spent.current = true;
          onChanged?.(last);
        },
        (cause: unknown) => {
          setOpenError(messageOf(cause));
        },
      )
      .finally(() => {
        inFlight.current = false;
        setBusy(false);
      });
  };

  const left = scene === null ? 0 : opened === null ? scene.left : Math.min(scene.left, opened.unopened);

  const next = (): void => {
    if (scene === null || left < 1 || spendOutNow()) return;
    begin({ ...scene, left: left - 1 });
    setRound((before) => before + 1);
  };

  // The ✕ and Escape both end here; "Next pack" keeps the scene, so a batch settles once (R3211000).
  const close = (): void => {
    if (busy) return;
    queued.current = [];
    begin(null);
    if (spent.current) {
      spent.current = false;
      onClosed?.();
    }
  };

  return (
    <div className="form-card packs-panel" data-testid={packTestid.panel}>
      <h2 className="shop-heading">{t("Card packs")}</h2>
      {loadError === null ? null : (
        <>
          <p className="notice" role="alert" data-testid={packTestid.loadError}>
            {t(loadError)}
          </p>
          <div className="account-actions">
            <button
              type="button"
              data-testid={packTestid.retry}
              disabled={reading}
              onClick={() => {
                setAttempt((before) => before + 1);
              }}
            >
              {t("Try again")}
            </button>
          </div>
        </>
      )}
      {info === null && loadError === null ? (
        <Skeleton variant="rows" label={t("Loading your packs…")} testId={packTestid.loading} />
      ) : null}
      {info === null ? null : (
        <>
          <PacksHeld info={info} waiting={spendOut} onOpen={start} />
        </>
      )}
      {scene === null ? null : (
        <PackScene
          key={round}
          pack={scene.pack}
          opened={opened}
          busy={busy}
          error={openError}
          left={left}
          onTear={tear}
          onNext={next}
          onClose={close}
        />
      )}
    </div>
  );
}

/**
 * The unopened packs the account holds, each opened alone or, from two, all in a row (up to
 * `PACKS_OPEN_ALL_MAX` at a press, R3212000); `waiting` while one of the tab's spends is out (R2378000).
 */
function PacksHeld({
  info,
  waiting,
  onOpen,
}: {
  info: PacksResponse;
  waiting: boolean;
  onOpen: (pack: string, count: number) => void;
}): ReactElement {
  if (info.unopened.length === 0) {
    return <EmptyState text={t("No unopened packs.")} testId={packTestid.none} />;
  }
  return (
    <div data-testid={packTestid.held}>
      {info.unopened.map(({ pack, count }) => (
        <div key={pack} className="packs-held" data-testid={packTestid.heldRow(pack)}>
          <span>{t("{pack}: {count} unopened", { pack: packName(pack), count })}</span>
          <div className="account-actions">
            <button
              type="button"
              className="spend-wait"
              data-testid={packTestid.open(pack)}
              disabled={waiting}
              aria-busy={waiting}
              onClick={() => {
                onOpen(pack, 1);
              }}
            >
              {t("Open")}
            </button>
            {count > 1 ? (
              <button
                type="button"
                className="spend-wait"
                data-testid={packTestid.openAll(pack)}
                disabled={waiting}
                aria-busy={waiting}
                onClick={() => {
                  onOpen(pack, Math.min(count, PACKS_OPEN_ALL_MAX));
                }}
              >
                {t("Open all {count}", { count: Math.min(count, PACKS_OPEN_ALL_MAX) })}
              </button>
            ) : null}
          </div>
        </div>
      ))}
    </div>
  );
}
