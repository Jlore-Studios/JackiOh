// The pack-opening scene and the card packs panel (#1308, SPEC §14.3, §14.4): the odds and the pity
// counters the server states, the packs the account holds, and the scene that opens one.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). `PackScene` never rolls: it draws the slots `POST
// /api/packs/open` already settled, and nothing on screen changes before that answer. Tearing is a
// request; skip-all, the holds and the flips only choose how fast an answer already in hand is
// shown. A refused open leaves the pack sealed on screen with the server's words beside it.
//
// REDUCED MOTION (the panel's setting or the system's) shows the results at once: no tear, no flip,
// no hold and no effects. A skip-all shows the same, with the best card's sting played once.

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

import { CardBack, CardFace, faceModel } from "../cards/index.ts";
import { resolveAnchor } from "../fx/anchors.ts";
import { FX_DEFAULT_SEED, FX_INTENSITY_SCALE } from "../fx/constants.ts";
import { capacityFor, createFxDirector, type FxDirector } from "../fx/director.ts";
import { defaultFrames, defaultNow, defaultVisibility } from "../fx/FxLayer.tsx";
import { useFxSettings } from "../fx/settings.ts";
import { createSurface } from "../fx/surface.ts";
import type { FxShakeSink } from "../fx/types.ts";
import { reducedMotionNow } from "../game/animations.ts";
import { t } from "../i18n/i18n.ts";
import { getPacks, openPack, type OpenedPackResponse, type PacksResponse } from "../net/api.ts";
import { useSetting } from "../settings/store.ts";
import {
  PACK_FLIP_MS,
  PACK_HOLD_MS,
  PACK_SWIPE_PX,
  PACK_TEAR_MS,
  TRACKED_RARITIES,
  bestRarity,
  holdsToFlip,
  packName,
  packTestid,
  percent,
  playStinger,
  revealCues,
  slotOutcome,
  withOpening,
} from "./packs.ts";
import "./packs.css";

/** The scene shakes nothing: the pack is not the board. */
const NO_SHAKE: FxShakeSink = { apply: () => undefined, clear: () => undefined };

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

  const holdTimer = useRef<number | null>(null);
  const swipeFrom = useRef<number | null>(null);
  const stung = useRef(false);
  const director = useRef<FxDirector | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const domRef = useRef<HTMLDivElement | null>(null);

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
    if (typeof event.currentTarget.setPointerCapture === "function") event.currentTarget.setPointerCapture(event.pointerId);
  };

  const onPackUp = (event: PointerEvent<HTMLButtonElement>): void => {
    const from = swipeFrom.current;
    swipeFrom.current = null;
    if (from !== null && Math.abs(event.clientX - from) >= PACK_SWIPE_PX) onTear();
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
      role="dialog"
      aria-modal="true"
      aria-label={title}
      style={style}
    >
      <h2 className="pack-scene__title">{title}</h2>

      {showSlots ? null : (
        <button
          type="button"
          className="pack"
          data-testid={packTestid.pack}
          data-state={opened === null ? "sealed" : "tearing"}
          data-best={opened === null ? undefined : bestRarity(opened.slots)}
          disabled={busy || opened !== null}
          onClick={onTear}
          onPointerDown={onPackDown}
          onPointerUp={onPackUp}
        >
          <span className="pack__top" aria-hidden="true" />
          <span className="pack__name">{packName(pack)}</span>
          <span className="pack__hint">{busy || opened !== null ? t("Opening…") : t("Swipe or tap to open")}</span>
        </button>
      )}

      {showSlots && opened !== null ? (
        <div className="pack-slots" data-testid={packTestid.slots}>
          {opened.slots.map((slot, index) => {
            const hold = holdsToFlip(slot.rarity);
            const shown = shows(index);
            return (
              <div
                key={index}
                className="pack-slot"
                data-testid={packTestid.slot(index)}
                data-rarity={slot.rarity}
                data-grant={slot.grant.kind}
                data-revealed={shown ? "true" : "false"}
                data-holding={holding === index ? "true" : undefined}
              >
                {shown ? (
                  <>
                    <div className="pack-slot__face">
                      <CardFace face={faceModel({ defId: slot.cardId, def: CATALOG[slot.cardId], radiant: false })} />
                    </div>
                    <p className="pack-slot__outcome" data-testid={packTestid.outcome(index)}>
                      {slotOutcome(slot)}
                    </p>
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
          {error}
        </p>
      )}

      <div className="pack-actions">
        {all ? null : (
          <button type="button" data-testid={packTestid.skipAll} disabled={busy} onClick={skipAll}>
            {t("Reveal all")}
          </button>
        )}
        {all && left > 0 ? (
          <button type="button" data-testid={packTestid.next} onClick={onNext}>
            {t("Next pack ({left} left)", { left })}
          </button>
        ) : null}
        <button type="button" data-testid={packTestid.close} disabled={busy} onClick={onClose}>
          {all ? t("Done") : t("Close")}
        </button>
      </div>

      {fxOn ? (
        <div className="fx-layer pack-fx" data-testid={packTestid.fx} data-fx="on" aria-hidden="true">
          <canvas ref={canvasRef} className="fx-canvas" />
          <div ref={domRef} className="fx-dom" />
        </div>
      ) : null}
    </div>,
    document.body,
  );
}

/** What a failed request says: the server's words, else the error's. */
function wordsOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

type SceneState = { pack: string; left: number };

/**
 * The panel. `revision` is how many times the page has asked it to read the packs again (another
 * panel bought one, #1309); `onChanged` tells the page an opening changed the account.
 */
export default function PackOpening({
  token,
  revision = 0,
  onChanged,
}: {
  token: string;
  revision?: number;
  onChanged?: () => void;
}): ReactElement {
  const [info, setInfo] = useState<PacksResponse | null>(null);
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

  useEffect(() => {
    let cancelled = false;
    setReading(true);
    getPacks(token).then(
      (response) => {
        if (!cancelled) {
          setInfo(response);
          setLoadError(null);
          setReading(false);
        }
      },
      (cause: unknown) => {
        if (!cancelled) {
          setLoadError(wordsOf(cause));
          setReading(false);
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, [token, revision, attempt]);

  const begin = (next: SceneState | null): void => {
    setScene(next);
    setOpened(null);
    setOpenError(null);
  };

  const start = (pack: string, count: number): void => {
    begin({ pack, left: count - 1 });
    setRound((before) => before + 1);
  };

  const tear = (): void => {
    if (scene === null || inFlight.current || opened !== null) return;
    inFlight.current = true;
    setBusy(true);
    setOpenError(null);
    openPack(token, scene.pack)
      .then(
        (next) => {
          setOpened(next);
          setInfo((before) => (before === null ? before : withOpening(before, next)));
          onChanged?.();
        },
        (cause: unknown) => {
          setOpenError(wordsOf(cause));
        },
      )
      .finally(() => {
        inFlight.current = false;
        setBusy(false);
      });
  };

  const left = scene === null ? 0 : opened === null ? scene.left : Math.min(scene.left, opened.unopened);

  const next = (): void => {
    if (scene === null || left < 1) return;
    begin({ pack: scene.pack, left: left - 1 });
    setRound((before) => before + 1);
  };

  const close = (): void => {
    if (!busy) begin(null);
  };

  return (
    <div className="form-card packs-panel" data-testid={packTestid.panel}>
      {loadError === null ? null : (
        <>
          <p className="notice" role="alert" data-testid={packTestid.loadError}>
            {loadError}
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
        <p className="notice" role="status" data-testid={packTestid.loading}>
          {t("Loading your packs…")}
        </p>
      ) : null}
      {info === null ? null : (
        <>
          <PackOdds info={info} />
          <PacksHeld info={info} onOpen={start} />
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

/** The odds per rarity, with and without pity, and the counters, all as `GET /api/packs` states them. */
function PackOdds({ info }: { info: PacksResponse }): ReactElement {
  const { odds, pity, thresholds } = info;
  return (
    <div data-testid={packTestid.odds}>
      <table className="packs-odds">
        <thead>
          <tr>
            <th scope="col">{t("Rarity")}</th>
            <th scope="col">{t("Chance per pack")}</th>
            <th scope="col">{t("With pity")}</th>
            <th scope="col">{t("Pity")}</th>
          </tr>
        </thead>
        <tbody>
          {TRACKED_RARITIES.map(({ key, rarity }) => (
            <tr key={key} data-testid={packTestid.oddsRow(key)}>
              <th scope="row">{t(rarity)}</th>
              <td>{percent(odds.base[key])}</td>
              <td>{percent(odds.target[key])}</td>
              <td data-testid={packTestid.pity(key)}>
                {t("{count} / {threshold} packs", { count: pity[key], threshold: thresholds[key] })}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <p data-testid={packTestid.filler}>
        {t("A slot that holds no Epic or better is Rare {rare} of the time, else Common.", {
          rare: percent(odds.fillerRare),
        })}
      </p>
      <p data-testid={packTestid.duplicate}>
        {t(
          "A duplicate becomes a cosmetic: Common {common}, Rare {rare}, Epic {epic}, Legendary {legendary}, Mythic {mythic}. With none of that rarity left to own, it fills both token bars.",
          {
            common: percent(odds.duplicateCosmetic.common),
            rare: percent(odds.duplicateCosmetic.rare),
            epic: percent(odds.duplicateCosmetic.epic),
            legendary: percent(odds.duplicateCosmetic.legendary),
            mythic: percent(odds.duplicateCosmetic.mythic),
          },
        )}
      </p>
    </div>
  );
}

/** The unopened packs the account holds, each opened alone or, from two, all in a row. */
function PacksHeld({ info, onOpen }: { info: PacksResponse; onOpen: (pack: string, count: number) => void }): ReactElement {
  if (info.unopened.length === 0) {
    return (
      <p data-testid={packTestid.none}>{t("No unopened packs.")}</p>
    );
  }
  return (
    <div data-testid={packTestid.held}>
      {info.unopened.map(({ pack, count }) => (
        <div key={pack} className="packs-held" data-testid={packTestid.heldRow(pack)}>
          <span>{t("{pack}: {count} unopened", { pack: packName(pack), count })}</span>
          <div className="account-actions">
            <button
              type="button"
              data-testid={packTestid.open(pack)}
              onClick={() => {
                onOpen(pack, 1);
              }}
            >
              {t("Open")}
            </button>
            {count > 1 ? (
              <button
                type="button"
                data-testid={packTestid.openAll(pack)}
                onClick={() => {
                  onOpen(pack, count);
                }}
              >
                {t("Open all {count}", { count })}
              </button>
            ) : null}
          </div>
        </div>
      ))}
    </div>
  );
}
