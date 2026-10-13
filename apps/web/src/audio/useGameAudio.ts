// Game's audio hook (SPEC §10.11): feeds views, entry starts and idles to the sound director;
// installs the gesture unlock, UI ticks and debug handle; preloads likely voice lines; and shares
// events with music (R631) and haptics (R669).
//
// ORDER MATTERS: call after `const runner = queue.current;` but before Game's layout effects, so
// cues are owed by the runner's first entry and a reduced-motion burst flushes with its own view.
//
// No AudioContext (jsdom) or sound failure is a no-op.

import { useContext, useEffect, useLayoutEffect, useRef } from "react";

import type { PlayerView } from "@jackioh/shared";

import { landingOf, type AnimationQueue } from "../game/animations.ts";
import { slamTierFor } from "../game/slamResolver.ts";
import { CatalogContext, type CardLookup } from "../game/catalog.ts";
import type { CueCard } from "./cues.ts";
import { retainAppAudio } from "./appAudio.ts";
import { exposeAudioDebug } from "./debug.ts";
import { createHaptics, type Haptics } from "../haptics/haptics.ts";
import { createSoundDirector, type SoundDirector } from "./director.ts";
import { getAudioEngine } from "./engine.ts";
import { createCrowdDirector, type CrowdDirector } from "./crowd.ts";
import { getMusicPlayer } from "./music.ts";
import { createMusicDirector, type MusicDirector } from "./musicDirector.ts";
import { enterGameMusic } from "./musicScene.ts";
import { CARD_AUDIO, voiceKeysForView } from "./voiceData.ts";

function quietly(run: () => void): void {
  try {
    run();
  } catch {
    // Sound is never a rule and never worth an error boundary: drop the cue, keep the game.
  }
}

/** Cue facts from the public base face, including printed token rarity (B2.5, R506). */
export function cueCard(lookup: CardLookup | null, defId: string): CueCard | undefined {
  const info = lookup?.(defId, false);
  if (info === undefined) return undefined;
  const card: CueCard = { type: info.type, tags: info.tags };
  if (info.rarity !== undefined) card.rarity = info.rarity;
  const printed = info.def?.printedRarity;
  if (printed !== undefined) card.printedRarity = printed;
  if (info.text !== "") card.text = info.text;
  return card;
}

export function useGameAudio(runner: AnimationQueue, view: PlayerView): void {
  // Use a ref so the persistent director sees a catalog that arrives after it.
  const lookup = useContext(CatalogContext);
  const lookupRef = useRef(lookup);
  lookupRef.current = lookup;
  const viewRef = useRef(view);
  viewRef.current = view;

  // The board's music director (R631), reached through this ref by the sound director.
  const musicRef = useRef<MusicDirector | null>(null);
  const crowdRef = useRef<CrowdDirector | null>(null);

  // Haptics (R669) hear each event when the singleton sound director resolves its cues.
  const hapticsRef = useRef<Haptics | null>(null);
  hapticsRef.current ??= createHaptics();
  const haptics = hapticsRef.current;
  const directorRef = useRef<SoundDirector | null>(null);
  directorRef.current ??= createSoundDirector(
    getAudioEngine(),
    CARD_AUDIO,
    (defId) => cueCard(lookupRef.current, defId),
    (event, planned) => {
      quietly(() => musicRef.current?.onEvent(event, planned));
      quietly(() => haptics.onEvent(event, planned));
      if (event.type === "damage") quietly(() => crowdRef.current?.observeDamage(event.amount));
      // A Unit lands at the queue's newest-view slam tier, under a hit's same quiet period.
      const landing = landingOf(event);
      if (landing !== null) quietly(() => crowdRef.current?.observeSlam(slamTierFor(landing, viewRef.current, lookupRef.current)));
    },
  );
  const director = directorRef.current;

  // Every view, before Game's enqueue layout effect.
  useLayoutEffect(() => {
    quietly(() => director.onView(view));
    quietly(() => musicRef.current?.onView(view));
  }, [director, view]);

  // Keep engine and music busy during entries (B58), so the idle flush preempts preload/prefetch.
  // Clear both if Game unmounts mid-burst.
  useLayoutEffect(() => {
    const engine = getAudioEngine();
    const music = getMusicPlayer();
    let last = runner.inFlight();
    quietly(() => engine.setBusy(last !== null));
    quietly(() => music.setBusy(last !== null));
    const stop = runner.subscribe(() => {
      const entry = runner.inFlight();
      if (entry !== null) {
        quietly(() => engine.setBusy(true));
        quietly(() => music.setBusy(true));
        if (entry !== last) quietly(() => director.onEntryStart(entry));
      } else {
        quietly(() => director.onIdle());
        quietly(() => musicRef.current?.settle());
        quietly(() => engine.setBusy(false));
        quietly(() => music.setBusy(false));
      }
      last = entry;
    });
    return () => {
      stop();
      quietly(() => engine.setBusy(false));
      quietly(() => music.setBusy(false));
    };
  }, [director, runner]);

  // Cover a view for which the runner starts no entry.
  useEffect(() => {
    if (runner.idle()) {
      quietly(() => director.onIdle());
      quietly(() => musicRef.current?.settle());
    }
  }, [director, view, runner]);

  // Keep app-level gesture unlock, UI ticks and debug handle for this component's lifetime.
  useEffect(() => {
    const engine = getAudioEngine();
    const removers = [retainAppAudio(), exposeAudioDebug(engine)];
    return () => {
      for (const remove of removers) remove();
    };
  }, []);

  // Crowd audio belongs to this match, not menu audio; release it on route leave.
  useEffect(() => {
    const crowd = createCrowdDirector({ engine: getAudioEngine() });
    crowdRef.current = crowd;
    crowd.start();
    return () => {
      crowd.dispose();
      if (crowdRef.current === crowd) crowdRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (view.result !== null) crowdRef.current?.end();
  }, [view.result]);

  // Enter board music before the first draw, then return control to the menu on unmount.
  useLayoutEffect(() => {
    let leave: (() => void) | null = null;
    quietly(() => {
      const sink = enterGameMusic();
      leave = () => sink.leave();
      musicRef.current = createMusicDirector({ sink });
      musicRef.current.onView(viewRef.current);
    });
    return () => {
      quietly(() => musicRef.current?.dispose());
      musicRef.current = null;
      quietly(() => leave?.());
    };
  }, []);

  // Busy bursts hold preloads until the context runs and the burst ends.
  useEffect(() => {
    quietly(() => {
      const engine = getAudioEngine();
      if (engine.state() === "running") engine.preloadVoices(voiceKeysForView(view, CARD_AUDIO));
    });
  }, [view]);
}
