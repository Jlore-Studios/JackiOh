// The one hook `Game` calls for sound (SPEC §10.11). It feeds the director every view, tells it
// (and the engine) when the animation runner starts an entry or goes idle, installs the gesture
// unlock, the UI ticks and the debug handle for the component's lifetime, and preloads the voice
// lines the view makes likely. It also gives the board the music (R631): a music director that
// hears every view, every event the sound director resolves, and each idle, for as long as the
// board is mounted. The same events reach the haptics (R669), so a buzz lands with its sound.
//
// ORDER MATTERS. `Game` calls this directly after `const runner = queue.current;`, before its own
// layout effects, so the director's `onView` runs before Game's enqueue layout effect: the events
// of a view are owed by the time the runner starts their first entry, and a reduced-motion burst
// (which drains inside `enqueue`) is flushed with the view that carried it.
//
// It never throws. With no AudioContext (jsdom) every call inside it is a no-op, and a failure in
// the sound path is swallowed rather than taking the board down with it.

import { useContext, useEffect, useLayoutEffect, useRef } from "react";

import type { PlayerView } from "@jackioh/shared";

import type { AnimationQueue } from "../game/animations.ts";
import { CatalogContext, type CardLookup } from "../game/catalog.ts";
import type { CueCard } from "./cues.ts";
import { retainAppAudio } from "./appAudio.ts";
import { exposeAudioDebug } from "./debug.ts";
import { createHaptics, type Haptics } from "../haptics/haptics.ts";
import { createSoundDirector, type SoundDirector } from "./director.ts";
import { getAudioEngine } from "./engine.ts";
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

/**
 * The catalog facts a cue may use, from the board's lookup (base face; "hidden" never reaches it):
 * type, tags, rarity and, for a token that prints one, its printed rarity (B2.5, R506).
 */
export function cueCard(lookup: CardLookup | null, defId: string): CueCard | undefined {
  const info = lookup?.(defId, false);
  if (info === undefined) return undefined;
  const card: CueCard = { type: info.type, tags: info.tags };
  if (info.rarity !== undefined) card.rarity = info.rarity;
  const printed = info.def?.printedRarity;
  if (printed !== undefined) card.printedRarity = printed;
  return card;
}

export function useGameAudio(runner: AnimationQueue, view: PlayerView): void {
  // 0. The public catalog the board renders with (routes put `CatalogContext` above Game), read
  //    through a ref so the director made once still sees a catalog that arrives later.
  const lookup = useContext(CatalogContext);
  const lookupRef = useRef(lookup);
  lookupRef.current = lookup;

  // 1. The music director (R631), made when the board mounts (effect 7) and dropped when it leaves;
  //    the sound director below reaches it through the ref, so it only ever hears this board's.
  const musicRef = useRef<MusicDirector | null>(null);

  // 1b. The sound director, created once per mounted Game against the singleton engine, and the
  //     haptics (R669), which hear every event it resolves in the same step as its cues.
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
    },
  );
  const director = directorRef.current;

  // 2. Every view, before Game's enqueue layout effect sees it.
  useLayoutEffect(() => {
    quietly(() => director.onView(view));
    quietly(() => musicRef.current?.onView(view));
  }, [director, view]);

  // 3. Entry starts and idles, straight from the runner's notifications. The engine is busy while
  //    an entry is in flight, so no background voice work competes with the runner's timers (B58):
  //    busy before the entry's cues, free after the idle flush's, so a line the flush asks for is
  //    fetched ahead of the held preload and the prefetch. A Game unmounted mid-burst frees it.
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

  // 4. After every layout effect: covers a view that produced no entry the runner could start.
  useEffect(() => {
    if (runner.idle()) {
      quietly(() => director.onIdle());
      quietly(() => musicRef.current?.settle());
    }
  }, [director, view, runner]);

  // 5. Gesture unlock and UI ticks (shared with the app root, appAudio.ts) and the debug handle,
  //    for the component's lifetime.
  useEffect(() => {
    const engine = getAudioEngine();
    const removers = [retainAppAudio(), exposeAudioDebug(engine)];
    return () => {
      for (const remove of removers) remove();
    };
  }, []);

  // 7. The board's music: entered on mount, with the view it mounted on, and handed back to the menu
  //    on unmount. A layout effect, so the first view reaches it before anything is drawn.
  const viewRef = useRef(view);
  viewRef.current = view;
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

  // 6. Preload the lines this view makes likely, once the context runs. A view that starts a burst
  //    reaches here with the engine already busy, so the engine holds it until the burst is over.
  useEffect(() => {
    quietly(() => {
      const engine = getAudioEngine();
      if (engine.state() === "running") engine.preloadVoices(voiceKeysForView(view, CARD_AUDIO));
    });
  }, [view]);
}
