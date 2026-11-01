// R2722002 against the real engine (WebAssembly) and its `viewFor` redaction: a hidden card makes its
// entrance (its intro, its sting, its animation and a Mythic's takeover) as it reveals and never as it
// is set, on both seats, and no cue keys on a card a seat's view hides at that moment. Every seat is
// fed the way Game feeds it: the new part of its view's window (`newEventsSince`), planned against the
// view before (`planEntries`, `entranceEntries`, `planFx`), the sound director over the entries, and
// the music director over the events the sound director resolves.

import type { Action, ActionBody, CardType, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { createMusicDirector } from "../audio/musicDirector.ts";
import { createSoundDirector } from "../audio/director.ts";
import { DEFAULT_AUDIO_SETTINGS } from "../audio/settings.ts";
import { devDeck, realPort } from "../audio/test/realGame.ts";
import type { SfxId, SfxParams, SoundSink } from "../audio/types.ts";
import { cueCard } from "../audio/useGameAudio.ts";
import { catalogFacts } from "../fx/FxLayer.tsx";
import { planFx } from "../fx/cues.ts";
import { createFxMemory } from "../fx/memory.ts";
import type { FxCue, FxRaysCue, FxTakeoverCue } from "../fx/types.ts";
import { entranceEntries, newEventsSince, planEntries, type AnimationEntry, type EntranceResolver } from "./animations.ts";
import { lookupFromDefs } from "./catalog.ts";
import { firstReveal, isFaceDownSet, otherSeatReveal } from "./entrance.ts";

const SEATS = ["p1", "p2"] as const;
/** Meditative #45 Knowledge Breaker (Legendary; its Aura lends Units Cloaked), #99 Paranoia (lends Spells Cloaked). */
const KNOWLEDGE_BREAKER = "meditative-045";
const PARANOIA = "meditative-099";
/** Meditative #8 Reach the Summit (a Mythic Spell), Classic #6 Cloaked Toe Cracker (a Common Unit that prints Cloaked). */
const SUMMIT = "meditative-008";
const TOE_CRACKER = "classic-006";
/** Core #85 Unlicensed Experimentation: a Legendary Trap that fires on a permanent of a type its controller has. */
const EXPERIMENT = "core-085";

type Heard = {
  sfx: { id: SfxId; params?: SfxParams }[];
  intros: string[];
  entries: AnimationEntry[];
  fx: FxCue[];
  fresh: GameEvent[];
  /** What the entries of the play itself (its announce, its play, its arrival) sounded and drew. */
  play: string[];
};
type Seat = { view: PlayerView; hear(next: PlayerView): Heard };

const port = realPort();
const lookup = lookupFromDefs(port.catalog?.() ?? {});
const resolve: EntranceResolver = (defId) => {
  const info = lookup(defId, false);
  return info === undefined ? null : { type: info.type as CardType, rarity: info.def?.printedRarity ?? info.rarity };
};

/** The entries of a play itself, for "a set sounds and looks the same whatever card it hides". */
const PLAY_TYPES = new Set<string>(["cardAnnounced", "cardPlayed", "summoned"]);

/** One seat's board: its directors, the cards its queue has entered, and what each new view made it play. */
function seat(first: PlayerView): Seat {
  const intros: string[] = [];
  const sfx: Heard["sfx"] = [];
  const sink: SoundSink = {
    playSfx: (id, params) => {
      sfx.push(params === undefined ? { id } : { id, params });
      return true;
    },
    playVoice: () => true,
    playEffect: () => true,
  };
  const music = createMusicDirector({
    sink: { request: () => {}, resetResume: () => {}, preload: () => {}, playIntro: (id) => intros.push(id), stopIntro: () => {}, preloadIntros: () => {} },
    settings: () => ({ ...DEFAULT_AUDIO_SETTINGS, dynamicMusic: true }),
    subscribeSettings: () => () => {},
    rotation: () => 0,
  });
  const sound = createSoundDirector(sink, undefined, (defId) => cueCard(lookup, defId), (event, view) => music.onEvent(event, view));
  const entered = new Set<string>();
  const memory = createFxMemory();
  const s: Seat = {
    view: first,
    hear(next) {
      sfx.length = 0;
      intros.length = 0;
      const fresh = newEventsSince(s.view.events, next.events);
      const entries = entranceEntries(planEntries(fresh, s.view, false), resolve, { reduced: false, speed: 1, seen: entered });
      sound.onView(next);
      music.onView(next);
      const env = { intensity: 1, card: (defId: string) => catalogFacts(lookup, defId), memory };
      const play: string[] = [];
      const fx = entries.flatMap((entry) => {
        const from = sfx.length;
        sound.onEntryStart(entry);
        memory.remember(entry.events);
        const cues = planFx(entry, entry.view, env);
        if (PLAY_TYPES.has(entry.type)) play.push(...sfx.slice(from).map((c) => `sfx:${c.id}`), ...cues.map((c) => `fx:${c.kind}`));
        return cues;
      });
      sound.onIdle();
      s.view = next;
      return { sfx: [...sfx], intros: [...intros], entries, fx, fresh, play: play.sort() };
    },
  };
  sound.onView(first);
  music.onView(first);
  return s;
}

/** A sandbox game, both seats' boards following it, reached to p1's main phase. */
function sandboxGame(seed: string) {
  const cheap = devDeck("cheap20");
  let state = port.beginGame(port.createGame({ seed, decks: [cheap, cheap], sandbox: true } as Parameters<typeof port.createGame>[0])).state;
  let nonce = 0;
  const raw = (player: PlayerId, body: ActionBody): void => {
    nonce += 1;
    const result = port.reduce(state, { ...body, playerId: player, nonce: `r2722002-${String(nonce)}` } as Action);
    if (result.error !== undefined) throw new Error(`${player} ${body.type}: ${result.error}`);
    state = result.state;
  };
  const actor = (): PlayerId | undefined => SEATS.find((p) => port.legalActions(state, p).some((a) => a.type !== "concede" && a.type !== "offerDraw"));
  // The mulligans, keeping everything, and whatever they open; then to p1's main phase.
  for (let step = 0; step < 40; step += 1) {
    const v = port.viewFor(state, "p1");
    if (v.active === "p1" && v.phase === "main" && v.pending === null) break;
    const who = actor();
    if (who === undefined) break;
    const legal = port.legalActions(state, who);
    const hand = port.viewFor(state, who).you.hand;
    if (legal.some((a) => a.type === "mulligan")) raw(who, { type: "mulligan", keep: Array.isArray(hand) ? hand.map((c) => c.instanceId) : [] });
    else raw(who, legal.find((a) => a.type === "answer") ?? { type: "endTurn" });
  }
  const seats = { p1: seat(port.viewFor(state, "p1")), p2: seat(port.viewFor(state, "p2")) };
  return {
    legal: (player: PlayerId) => port.legalActions(state, player),
    view: (player: PlayerId) => port.viewFor(state, player),
    hand: (player: PlayerId) => {
      const hand = port.viewFor(state, player).you.hand;
      return Array.isArray(hand) ? hand : [];
    },
    /** Acts, answering any prompt it opens with its first answer, and reports what each seat played. */
    act(player: PlayerId, body: ActionBody): Record<PlayerId, Heard> {
      raw(player, body);
      for (let i = 0; i < 6; i += 1) {
        const who = actor();
        const answer = who === undefined ? undefined : port.legalActions(state, who).find((a) => a.type === "answer");
        if (who === undefined || answer === undefined) break;
        raw(who, answer);
      }
      return { p1: seats.p1.hear(port.viewFor(state, "p1")), p2: seats.p2.hear(port.viewFor(state, "p2")) };
    },
    sandbox(player: PlayerId, op: unknown): void {
      raw(player, { type: "sandbox", op } as unknown as ActionBody);
      seats.p1.hear(port.viewFor(state, "p1"));
      seats.p2.hear(port.viewFor(state, "p2"));
    },
  };
}

const rarityOf = (defId: string): string | undefined => resolve(defId)?.rarity;
const isGrand = (defId: string): boolean => rarityOf(defId) === "Legendary" || rarityOf(defId) === "Mythic";

/**
 * The Legendary and Mythic cards a seat's own new events name at an entrance's moment: a face-up play
 * (no set), a Unit summoned face-up, a first firing it reads, or a Cloaked reveal by the other player.
 */
function namedGrand(fresh: readonly GameEvent[], viewer: PlayerId): string[] {
  return fresh.flatMap((event) => {
    if (event.type === "cardPlayed" && !isFaceDownSet(event) && resolve(event.defId)?.type !== "Trap" && resolve(event.defId)?.type !== "Field Trap") return [event.defId];
    if (event.type === "summoned" && event.row === "units" && event.defId !== "hidden") return [event.defId];
    if (firstReveal(event) || otherSeatReveal(event, viewer)) return [event.defId];
    return [];
  }).filter(isGrand);
}

/** A takeover, or the rays a Legendary's or a Mythic's entrance throws (a heal's holy rays are no entrance). */
const isEntranceFx = (cue: FxCue): cue is FxRaysCue | FxTakeoverCue =>
  cue.kind === "takeover" || (cue.kind === "rays" && (cue.tone === "legendary" || cue.tone === "mythic"));

/** Every cue an entrance or a rarity makes: the intros, the entrance and rarity stings, the entrances, the rays and the takeover. */
function grandCues(heard: Heard): string[] {
  return [
    ...heard.intros.map((id) => `intro:${id}`),
    ...heard.sfx.filter((s) => s.id === "entrance" || s.id === "sting").map((s) => `sfx:${s.id}${s.params?.mythic === true ? ":mythic" : ""}${s.params?.tier === undefined ? "" : `:${s.params.tier}`}`),
    ...heard.entries.flatMap((entry) => (entry.entrance === undefined ? [] : [`entrance:${entry.entrance.tier}:${String(entry.entrance.holdMs > 0)}`])),
    ...heard.fx.filter(isEntranceFx).map((cue) => (cue.kind === "rays" ? `rays:${cue.tone}` : `takeover:${cue.look}`)),
  ].sort();
}


/** The Cloaked game: three set, one destroyed unrevealed, and the rest revealed, every action from both seats. */
function cloakedGame() {
  const game = sandboxGame("r2722002");
  game.sandbox("p1", { kind: "setMana", player: "p1", current: 10, max: 10 });
  game.sandbox("p1", { kind: "spawn", player: "p1", defId: KNOWLEDGE_BREAKER, to: { zone: "field", row: "units", lane: 1 } });
  game.sandbox("p1", { kind: "spawn", player: "p1", defId: PARANOIA, to: { zone: "field", row: "backrow", lane: 1 } });
  for (const defId of [KNOWLEDGE_BREAKER, SUMMIT, TOE_CRACKER, TOE_CRACKER]) game.sandbox("p1", { kind: "spawn", player: "p1", defId, to: { zone: "hand" } });
  const sets: { defId: string; heard: Record<PlayerId, Heard>; id: string }[] = [];
  const taken = new Set<string>();
  for (const defId of [KNOWLEDGE_BREAKER, SUMMIT, TOE_CRACKER, TOE_CRACKER]) {
    const inHand = game.hand("p1").find((c) => c.defId === defId && !taken.has(c.instanceId));
    if (inHand === undefined) throw new Error(`no ${defId} in hand`);
    taken.add(inHand.instanceId);
    const play = game.legal("p1").find((a) => a.type === "play" && a.instanceId === inHand.instanceId && a.faceDown === "cloaked");
    if (play === undefined) throw new Error(`no face-down play of ${defId}`);
    const heard = game.act("p1", play);
    const set = heard.p1.fresh.find((e) => e.type === "cardPlayed" && e.formerId === inHand.instanceId);
    if (set === undefined || set.type !== "cardPlayed") throw new Error(`no set of ${defId}`);
    sets.push({ defId, heard, id: set.instanceId });
  }
  const [breaker, summit, cracker, doomed] = sets;
  if (breaker === undefined || summit === undefined || cracker === undefined || doomed === undefined) throw new Error("four sets");
  // A card destroyed before it reveals (R783008).
  const destroyed = game.act("p1", { type: "sandbox", op: { kind: "destroy", instanceId: doomed.id } } as unknown as ActionBody);
  const reveal = (id: string): Record<PlayerId, Heard> => {
    const action = game.legal("p1").find((a) => a.type === "activate" && a.instanceId === id && a.ability === "reveal");
    if (action === undefined) throw new Error(`no reveal of ${id}`);
    return game.act("p1", action);
  };
  return { sets, destroyed, reveals: { breaker: reveal(breaker.id), summit: reveal(summit.id), cracker: reveal(cracker.id) } };
}

/** The Trap game: p1 sets a Legendary Trap and p2's play of a Unit fires it. */
function trapGame() {
  const game = sandboxGame("r2722002-trap");
  game.sandbox("p1", { kind: "setMana", player: "p1", current: 10, max: 10 });
  game.sandbox("p1", { kind: "spawn", player: "p1", defId: "core-004", to: { zone: "field", row: "units", lane: 1 } });
  game.sandbox("p1", { kind: "spawn", player: "p1", defId: EXPERIMENT, to: { zone: "hand" } });
  const trap = game.hand("p1").find((c) => c.defId === EXPERIMENT);
  const setAction = game.legal("p1").find((a) => a.type === "play" && a.instanceId === trap?.instanceId);
  if (setAction === undefined) throw new Error("no set of the Trap");
  const set = game.act("p1", setAction);
  game.act("p1", { type: "endTurn" });
  game.sandbox("p2", { kind: "setMana", player: "p2", current: 10, max: 10 });
  game.sandbox("p2", { kind: "spawn", player: "p2", defId: "core-004", to: { zone: "hand" } });
  const unitCard = game.hand("p2").find((c) => c.defId === "core-004");
  const unitPlay = game.legal("p2").find((a) => a.type === "play" && a.instanceId === unitCard?.instanceId && a.faceDown === undefined);
  if (unitPlay === undefined) throw new Error("no play of p2's unit");
  const fire = game.act("p2", unitPlay);
  return { set, fire };
}

describe("R2722002 against the real engine: a hidden card's entrance is at its reveal", () => {
  const cloaked = cloakedGame();
  const trap = trapGame();
  const actions = [
    ...cloaked.sets.map((s) => ({ label: `set ${s.defId}`, heard: s.heard })),
    { label: "destroyed unrevealed", heard: cloaked.destroyed },
    { label: "reveal Knowledge Breaker", heard: cloaked.reveals.breaker },
    { label: "reveal Reach the Summit", heard: cloaked.reveals.summit },
    { label: "reveal Toe Cracker", heard: cloaked.reveals.cracker },
    { label: "set Unlicensed Experimentation", heard: trap.set },
    { label: "fire Unlicensed Experimentation", heard: trap.fire },
  ];

  it("R2722002 no cue keys on a card the seat's view hides", () => {
    for (const { label, heard } of actions) {
      for (const viewer of SEATS) {
        const seen = heard[viewer];
        const named = namedGrand(seen.fresh, viewer);
        const where = `${label} on ${viewer}`;
        // Every intro, entrance sting, entrance, ray and takeover is a card this seat's own events name.
        for (const id of seen.intros) expect(named.map((d) => `intro-${d}`), where).toContain(id);
        for (const entry of seen.entries) if (entry.entrance !== undefined) expect(named, where).toContain(entry.entrance.defId);
        if (named.length === 0) {
          expect(seen.sfx.filter((s) => s.id === "entrance"), where).toEqual([]);
          expect(seen.fx.filter(isEntranceFx), where).toEqual([]);
        }
      }
    }
    // Every set and the destroyed card: nothing that names or hints at the card, on either seat.
    for (const { label, heard } of [...cloaked.sets.map((s) => ({ label: s.defId, heard: s.heard })), { label: "destroyed", heard: cloaked.destroyed }, { label: "trap set", heard: trap.set }]) {
      for (const viewer of SEATS) expect(grandCues(heard[viewer]), `${label} on ${viewer}`).toEqual([]);
    }
    // The other seat hears and sees a Legendary's set and a Mythic's exactly as a Common's.
    const [breaker, summit, cracker] = cloaked.sets;
    expect(must(cracker).heard.p2.play.length).toBeGreaterThan(0);
    expect(must(breaker).heard.p2.play).toEqual(must(cracker).heard.p2.play);
    expect(must(summit).heard.p2.play).toEqual(must(cracker).heard.p2.play);
    // A real Trap's firing names it to its controller alone: the other seat's is rarity-neutral (the
    // sting is the Common Unit's own play that fires it).
    expect(grandCues(trap.fire.p2)).toEqual(["sfx:sting"]);
  });

  it("R2722002 the reveal plays the full entrance on both seats", () => {
    for (const viewer of SEATS) {
      expect(grandCues(cloaked.reveals.breaker[viewer]), `Knowledge Breaker on ${viewer}`).toEqual(
        ["intro:intro-meditative-045", "sfx:entrance", "entrance:legendary:false", "rays:legendary"].sort(),
      );
      const summit = grandCues(cloaked.reveals.summit[viewer]);
      expect(summit, `Reach the Summit on ${viewer}`).toEqual(
        ["intro:intro-meditative-008", "sfx:entrance:mythic", "entrance:mythic:true", "rays:mythic", "takeover:prismatic"].sort(),
      );
      expect(grandCues(cloaked.reveals.cracker[viewer]), `Toe Cracker on ${viewer}`).toEqual(["sfx:sting"]);
    }
    // The Legendary Trap's first firing on its controller's seat: its intro, its entrance and its rays.
    // (Its sting is the Common Unit's play that fires it.)
    expect(grandCues(trap.fire.p1)).toEqual(["intro:intro-core-085", "sfx:entrance", "entrance:legendary:false", "rays:legendary", "sfx:sting"].sort());
  });
});

function must<T>(value: T | undefined): T {
  if (value === undefined) throw new Error("missing");
  return value;
}
