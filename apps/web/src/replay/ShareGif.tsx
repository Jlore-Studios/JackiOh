// "Share as GIF" in the replay player (#513, R513000): the shown turn, or the range marked with the
// scrubber, drawn as a GIF from its steps' views alone and handed to the share sheet or downloaded.
// The state lives in `useGifExport`, which the player calls above its board so a fresh board (every
// jump gives `Game` a new key) keeps it; `ShareGif` draws it in the board's notice slot beneath the
// toolbar. The export itself, the raster and the encoder load on demand, with `import()`, the first
// time a GIF is made, so the main bundle does not carry them.

import { useCallback, useContext, useEffect, useRef, useState } from "react";
import type { ReactElement } from "react";

import { CatalogContext } from "../game/catalog.ts";
import { t } from "../i18n/i18n.ts";
import { REPLAY_GIF_SIZES } from "./config.ts";
import type { ReplayGifPreset } from "./config.ts";
import type * as Gif from "./gif/export.ts";
import { clampClip, loadClip, turnRange } from "./steps.ts";
import type { StepCache } from "./steps.ts";

export const gifTestid = {
  toggle: "replay-gif",
  panel: "replay-gif-panel",
  clip: "replay-gif-clip",
  start: "replay-gif-start",
  end: "replay-gif-end",
  turn: "replay-gif-turn",
  shape: "replay-gif-shape",
  make: "replay-gif-make",
  progress: "replay-gif-progress",
  cancel: "replay-gif-cancel",
  tooBig: "replay-gif-too-big",
  failed: "replay-gif-failed",
  share: "replay-gif-share",
} as const;

export type GifPhase =
  | { kind: "idle" }
  | { kind: "working"; frame: number; frames: number; attempt: number; attempts: number }
  | { kind: "ready"; file: File; share: boolean }
  | { kind: "tooBig" }
  | { kind: "failed"; message: string };

export type GifExport = {
  open: boolean;
  /** The marked ends, null while unmarked; one mark alone is a clip of that step. */
  from: number | null;
  to: number | null;
  /** The shown step. */
  current: number;
  preset: ReplayGifPreset;
  phase: GifPhase;
  toggle(): void;
  markStart(): void;
  markEnd(): void;
  wholeTurn(): void;
  setPreset(preset: ReplayGifPreset): void;
  make(): void;
  cancel(): void;
  send(): void;
};

const IDLE: GifPhase = { kind: "idle" };

function isAbort(cause: unknown): boolean {
  return cause instanceof DOMException && cause.name === "AbortError";
}

/** The clip marked on the scrubber, clamped to its cap, or null while nothing is marked. */
export function markedClip(from: number | null, to: number | null): [number, number] | null {
  if (from === null && to === null) return null;
  const a = from ?? to ?? 0;
  return clampClip(a, to ?? from ?? a);
}

export function useGifExport(cache: StepCache, current: number): GifExport {
  const lookup = useContext(CatalogContext);
  const [open, setOpen] = useState(false);
  const [from, setFrom] = useState<number | null>(null);
  const [to, setTo] = useState<number | null>(null);
  const [preset, setPresetState] = useState<ReplayGifPreset>("landscape");
  const [phase, setPhase] = useState<GifPhase>(IDLE);
  /** The export in flight, which Cancel and a leaving player abort. */
  const running = useRef<AbortController | null>(null);
  /** The export module once a GIF has been made: Share calls it without waiting, inside the click's gesture. */
  const loaded = useRef<typeof Gif | null>(null);

  useEffect(
    () => () => {
      running.current?.abort();
    },
    [],
  );

  const forget = useCallback((): void => {
    setPhase(IDLE);
  }, []);

  const make = useCallback((): void => {
    running.current?.abort();
    const controller = new AbortController();
    running.current = controller;
    setPhase({ kind: "working", frame: 0, frames: 0, attempt: 1, attempts: 1 });
    void (async () => {
      try {
        const [first, last] = markedClip(from, to) ?? (await turnRange(cache, current));
        const clip = await loadClip(cache, first, last);
        const gif = await import("./gif/export.ts");
        loaded.current = gif;
        if (controller.signal.aborted) return;
        const result = await gif.exportGif(
          { clip, preset, lookup },
          {
            signal: controller.signal,
            onProgress: (progress) => {
              if (!controller.signal.aborted) setPhase({ kind: "working", ...progress });
            },
          },
        );
        if (controller.signal.aborted || result.kind === "cancelled") return;
        setPhase(result.kind === "ready" ? { kind: "ready", file: result.file, share: gif.canShareFile(result.file) } : { kind: "tooBig" });
      } catch (cause) {
        if (controller.signal.aborted || isAbort(cause)) return;
        setPhase({ kind: "failed", message: cause instanceof Error ? cause.message : String(cause) });
      }
    })();
  }, [cache, current, from, to, preset, lookup]);

  return {
    open,
    from,
    to,
    current,
    preset,
    phase,
    toggle: () => {
      setOpen(!open);
    },
    markStart: () => {
      setFrom(current);
      forget();
    },
    markEnd: () => {
      setTo(current);
      forget();
    },
    wholeTurn: () => {
      setFrom(null);
      setTo(null);
      forget();
    },
    setPreset: (next) => {
      setPresetState(next);
      forget();
    },
    make,
    cancel: () => {
      running.current?.abort();
      running.current = null;
      setPhase(IDLE);
    },
    send: () => {
      if (phase.kind !== "ready" || loaded.current === null) return;
      loaded.current.shareGif(phase.file).catch((cause: unknown) => {
        // A dismissed share sheet is the viewer's answer, not a failure.
        if (!isAbort(cause)) setPhase({ kind: "failed", message: cause instanceof Error ? cause.message : String(cause) });
      });
    },
  };
}

const PRESETS = Object.keys(REPLAY_GIF_SIZES) as ReplayGifPreset[];

function presetLabel(preset: ReplayGifPreset): string {
  return preset === "landscape" ? t("Landscape") : t("Square");
}

export function ShareGif({ gif }: { gif: GifExport }): ReactElement {
  const { phase } = gif;
  const marked = markedClip(gif.from, gif.to);
  const working = phase.kind === "working";
  return (
    <div className="replay-gif-panel" data-testid={gifTestid.panel}>
      <button type="button" data-testid={gifTestid.toggle} aria-expanded={gif.open} onClick={gif.toggle}>
        {t("Share as GIF")}
      </button>
      {gif.open ? (
        <>
          <span data-testid={gifTestid.clip}>
            {marked === null ? t("Clip: this turn") : t("Clip: steps {from} to {to}", { from: marked[0] + 1, to: marked[1] + 1 })}
          </span>
          <button type="button" data-testid={gifTestid.start} onClick={gif.markStart} disabled={working}>
            {t("Start here")}
          </button>
          <button type="button" data-testid={gifTestid.end} onClick={gif.markEnd} disabled={working}>
            {t("End here")}
          </button>
          <button type="button" data-testid={gifTestid.turn} onClick={gif.wholeTurn} disabled={working || marked === null}>
            {t("This turn")}
          </button>
          <select
            data-testid={gifTestid.shape}
            aria-label={t("Shape")}
            value={gif.preset}
            disabled={working}
            onChange={(event) => {
              gif.setPreset(event.currentTarget.value as ReplayGifPreset);
            }}
          >
            {PRESETS.map((preset) => (
              <option key={preset} value={preset}>
                {presetLabel(preset)}
              </option>
            ))}
          </select>
          <button type="button" data-testid={gifTestid.make} onClick={gif.make} disabled={working}>
            {t("Make GIF")}
          </button>
          {phase.kind === "working" ? (
            <>
              <progress
                data-testid={gifTestid.progress}
                max={phase.frames > 0 ? phase.frames : undefined}
                value={phase.frames > 0 ? phase.frame : undefined}
              />
              {phase.frames > 0 ? (
                <span role="status">
                  {phase.attempt > 1
                    ? t("Drawing frame {frame} of {frames} (try {attempt} of {attempts})", {
                        frame: phase.frame,
                        frames: phase.frames,
                        attempt: phase.attempt,
                        attempts: phase.attempts,
                      })
                    : t("Drawing frame {frame} of {frames}", { frame: phase.frame, frames: phase.frames })}
                </span>
              ) : null}
              <button type="button" data-testid={gifTestid.cancel} onClick={gif.cancel}>
                {t("Cancel")}
              </button>
            </>
          ) : null}
          {phase.kind === "tooBig" ? (
            <span role="alert" data-testid={gifTestid.tooBig}>
              {t("Pick a shorter clip")}
            </span>
          ) : null}
          {phase.kind === "failed" ? (
            <span role="alert" data-testid={gifTestid.failed}>
              {phase.message}
            </span>
          ) : null}
          {phase.kind === "ready" ? (
            <button type="button" data-testid={gifTestid.share} onClick={gif.send}>
              {phase.share ? t("Share") : t("Download")}
            </button>
          ) : null}
        </>
      ) : null}
    </div>
  );
}
