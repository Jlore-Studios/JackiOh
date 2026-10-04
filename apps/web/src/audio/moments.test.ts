// R506 through the director: the plays in progress a cue may answer, and a card cast as it is
// drawn, followed through the stream the way the runner plays it; then the same against real engine
// games, both seats, with `viewFor`'s own redaction (R97, R203): #21 Hinder cast on draw cracks its
// victim's mana, and #27 Blood Ridden Glowy Jelly Bean's Radiant card bursts gold on the seat that
// cannot read it too.

import type { ActionBody, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { BLOOD_BEAN_DEF_ID, GOLD_BURST_DELAY_MS, HIDDEN_DEF_ID, HINDER_DEF_ID, NEXT_REFRESH_MODIFIER_ID } from "./constants.ts";
import { createSoundDirector, type SoundDirector } from "./director.ts";
import { answerPrompts, devDeck, realGame, type RealGame } from "./test/realGame.ts";
import type { SfxId, SfxParams, SoundSink } from "./types.ts";
import { castOnDrawAt } from "../fx/castOnDraw.ts";
import { newEventsSince, planEntries } from "../game/animations.ts";
import { baseView, emptySide, withEvents } from "../test/fixtures.ts";

/* --------------------------------------------------------------------------------------------- *
 * Rig
 * --------------------------------------------------------------------------------------------- */

type Sent = { id: SfxId; params: SfxParams | undefined; delayMs: number };
type Recorder = SoundSink & { sfx: Sent[] };

function recorder(): Recorder {
  const sfx: Sent[] = [];
  return {
    sfx,
    playSfx(id, params, delayMs) {
      sfx.push({ id, params, delayMs: delayMs ?? 0 });
      return true;
    },
    playVoice() {
      return true;
    },
    playEffect() {
      return true;
    },
  };
}

const ids = (sink: Recorder): SfxId[] => sink.sfx.map((s) => s.id);

/** Feeds `before`, then `after`, and plays every new entry the way Game does, then goes idle. */
function playLikeGame(director: SoundDirector, before: PlayerView, after: PlayerView): void {
  director.onView(after);
  for (const entry of planEntries(newEventsSince(before.events, after.events), before, false)) director.onEntryStart(entry);
  director.onIdle();
}

/** A fresh director on `first`, then one view carrying `events`, played entry by entry. */
function hear(events: GameEvent[], first: PlayerView = baseView()): Recorder {
  const sink = recorder();
  const director = createSoundDirector(sink);
  director.onView(first);
  playLikeGame(director, first, withEvents(first, events));
  return sink;
}

const drawn = (instanceId: string, defId: string, player: PlayerId = "p2"): GameEvent => ({ type: "drawn", player, instanceId, defId });
const played = (instanceId: string, defId: string, player: PlayerId = "p2"): GameEvent => ({
  type: "cardPlayed",
  player,
  instanceId,
  defId,
  costPaid: 0,
});
const resolved = (instanceId: string, defId: string, player: PlayerId = "p2"): GameEvent => ({
  type: "cardResolved",
  player,
  instanceId,
  defId,
  permanent: false,
  costPaid: 0,
});
const rider = (player: PlayerId = "p1"): GameEvent => ({ type: "modifierChanged", player, modifierId: NEXT_REFRESH_MODIFIER_ID, added: true });
const radiantInHand = (instanceId: string, defId: string, player: PlayerId = "p2"): GameEvent => ({
  type: "radiantSet",
  instanceId,
  defId,
  zone: { z: "hand", player },
});
const healthLost = (player: PlayerId, amount: number): GameEvent => ({ type: "healthLost", player, amount });
const SPELL = "core-005";

/* --------------------------------------------------------------------------------------------- *
 * The stream, synthetic
 * --------------------------------------------------------------------------------------------- */

describe("R506 the director follows the plays in progress", () => {
  it("R506 Hinder drawn and cast: the cast-on-draw sting, then the victim's rider cracks", () => {
    const sink = hear([drawn("c7", HINDER_DEF_ID), played("c7", HINDER_DEF_ID), rider("p1"), resolved("c7", HINDER_DEF_ID)]);
    expect(ids(sink)).toContain("castOnDraw");
    expect(ids(sink)).not.toContain("play");
    expect(ids(sink)).toContain("manaCrack");
    expect(ids(sink)).not.toContain("notify");
  });

  it("R506 once Hinder's play has resolved, a later rider is an ordinary notice", () => {
    const sink = hear([played("c7", HINDER_DEF_ID), resolved("c7", HINDER_DEF_ID), rider("p1")]);
    expect(ids(sink)).toContain("notify");
    expect(ids(sink)).not.toContain("manaCrack");
  });

  it("R506 a play the stream never closes is forgotten when a turn starts", () => {
    const sink = hear([played("c7", HINDER_DEF_ID), { type: "turnStarted", player: "p1", turn: 3 }, rider("p1")]);
    expect(ids(sink)).not.toContain("manaCrack");
  });

  it("R506 the innermost play is the one a cue answers", () => {
    const nested = hear([played("c7", BLOOD_BEAN_DEF_ID), played("c8", SPELL), radiantInHand("c3", "core-004")]);
    expect(ids(nested)).toContain("radiant");
    expect(ids(nested)).not.toContain("goldBurst");

    const back = hear([
      played("c7", BLOOD_BEAN_DEF_ID),
      played("c8", SPELL),
      resolved("c8", SPELL),
      radiantInHand("c3", "core-004"),
    ]);
    expect(ids(back)).toContain("goldBurst");
  });

  it("R506 #27's Radiant card bursts gold behind its blood drain, and its health loss drains as ever", () => {
    const sink = hear([
      drawn("c7", BLOOD_BEAN_DEF_ID),
      played("c7", BLOOD_BEAN_DEF_ID),
      radiantInHand("c3", "core-004"),
      healthLost("p2", 5),
      resolved("c7", BLOOD_BEAN_DEF_ID),
    ]);
    const drain = sink.sfx.find((s) => s.id === "bloodDrain");
    const burst = sink.sfx.find((s) => s.id === "goldBurst");
    expect(drain).toBeDefined();
    expect((burst?.delayMs ?? 0) - (drain?.delayMs ?? 0)).toBe(GOLD_BURST_DELAY_MS);
    expect(ids(sink)).toContain("drain");
  });

  it("R506 a card drawn earlier and played from hand later is no cast on draw", () => {
    const sink = hear([drawn("c7", SPELL), { type: "manaChanged", player: "p2", current: 3, max: 3 }, played("c7", SPELL)]);
    expect(ids(sink)).toContain("play");
    expect(ids(sink)).not.toContain("castOnDraw");
  });

  it("R506 the drawn card's own announcement or Gifted Program's Radiant may come between the draw and the cast", () => {
    const announced: GameEvent = { type: "cardAnnounced", player: "p2", instanceId: "c7", defId: SPELL, cardType: "Spell", costPaid: 0, targets: [] };
    const sink = hear([drawn("c7", SPELL), announced, radiantInHand("c7", SPELL), played("c7", SPELL)]);
    expect(ids(sink)).toContain("castOnDraw");
  });

  it("R506 a draw and a play of different cards, or by different players, are no cast on draw", () => {
    expect(ids(hear([drawn("c7", SPELL), played("c8", SPELL)]))).not.toContain("castOnDraw");
    expect(ids(hear([drawn("c7", SPELL, "p1"), played("c7", SPELL, "p2")]))).not.toContain("castOnDraw");
  });

  it("R203 a draw behind the sentinel marks nothing, and a play behind it is no Hinder", () => {
    const sink = hear([drawn(HIDDEN_DEF_ID, HIDDEN_DEF_ID), played(HIDDEN_DEF_ID, HIDDEN_DEF_ID), rider("p1")]);
    expect(ids(sink)).toContain("play");
    expect(ids(sink)).not.toContain("castOnDraw");
    expect(ids(sink)).not.toContain("manaCrack");
  });

  it("R506 R203 a cast on draw paused by its own prompt still stings, on the seat that could not read the draw", () => {
    const sink = recorder();
    const director = createSoundDirector(sink);
    const first = baseView();
    director.onView(first);
    const asked: GameEvent = { type: "promptOpened", player: "p2", choiceId: "q1", kind: "hand" };
    const view1 = withEvents(first, [drawn(HIDDEN_DEF_ID, HIDDEN_DEF_ID), asked]);
    playLikeGame(director, first, view1);
    const announced: GameEvent = {
      type: "cardAnnounced",
      player: "p2",
      instanceId: "c7",
      defId: HINDER_DEF_ID,
      cardType: "Spell",
      costPaid: 0,
      targets: [],
    };
    const view2 = withEvents(view1, [{ type: "promptAnswered", player: "p2", choiceId: "q1" }, announced, played("c7", HINDER_DEF_ID)]);
    playLikeGame(director, view1, view2);
    expect(ids(sink)).toContain("castOnDraw");
  });

  it("R506 a reduced-motion burst, flushed at idle, follows the plays the same way", () => {
    const sink = recorder();
    const director = createSoundDirector(sink);
    const first = baseView();
    director.onView(first);
    director.onView(withEvents(first, [drawn("c7", HINDER_DEF_ID), played("c7", HINDER_DEF_ID), rider("p1")]));
    director.onIdle();
    expect(ids(sink)).toEqual(expect.arrayContaining(["castOnDraw", "manaCrack"]));
  });

  it("R203 a hand-over to the other seat forgets the plays in progress", () => {
    const sink = recorder();
    const director = createSoundDirector(sink);
    const first = baseView();
    director.onView(first);
    playLikeGame(director, first, withEvents(first, [played("c7", HINDER_DEF_ID)]));
    const other = baseView({ viewer: "p2", active: "p2", you: emptySide("p2"), opponent: emptySide("p1") });
    director.onView(other);
    playLikeGame(director, other, withEvents(other, [rider("p2")]));
    expect(ids(sink)).not.toContain("manaCrack");
  });
});

/* --------------------------------------------------------------------------------------------- *
 * Against real engine games and viewFor's redaction
 * --------------------------------------------------------------------------------------------- */

type Seats = { p1: Recorder; p2: Recorder };
/** `fresh`: each seat's new events, as its view redacts them. */
type Heard = { seats: Seats; events: GameEvent[]; fresh: { p1: GameEvent[]; p2: GameEvent[] } };

/** Both seats' directors, fed `before` then `after` the way Game feeds them; returns what each heard. */
function hearAction(game: RealGame, player: PlayerId, body: ActionBody, settle = false): Heard {
  const seats: Seats = { p1: recorder(), p2: recorder() };
  const first = { p1: game.view("p1"), p2: game.view("p2") };
  const directors = { p1: createSoundDirector(seats.p1), p2: createSoundDirector(seats.p2) };
  directors.p1.onView(first.p1);
  directors.p2.onView(first.p2);
  const events: GameEvent[] = [];
  const fresh: { p1: GameEvent[]; p2: GameEvent[] } = { p1: [], p2: [] };
  let next: [PlayerId, ActionBody] | null = [player, body];
  // `settle`: then answer whatever the action asked (the cast's own prompts, #21's discard) with the
  // first listed answer, each answer a view of its own, as Game feeds the same directors.
  while (next !== null) {
    const before = { p1: game.view("p1"), p2: game.view("p2") };
    events.push(...game.act(next[0], next[1]));
    const after = { p1: game.view("p1"), p2: game.view("p2") };
    playLikeGame(directors.p1, before.p1, after.p1);
    playLikeGame(directors.p2, before.p2, after.p2);
    fresh.p1.push(...newEventsSince(before.p1.events, after.p1.events));
    fresh.p2.push(...newEventsSince(before.p2.events, after.p2.events));
    const who = settle ? game.actor() : null;
    const answer = who === null ? undefined : game.legal(who).find((a) => a.type === "answer");
    next = who === null || answer === undefined ? null : [who, answer];
  }
  return { seats, events, fresh };
}

/** Whether `events` hold a draw of `defId` by p2 that is cast at once. */
function castOnDrawOf(events: readonly GameEvent[], defId: string): boolean {
  return events.some((event, i) => event.type === "cardPlayed" && event.defId === defId && castOnDrawAt(events, i));
}

/**
 * A real game (p1 on cheap20; p2 on cheap20 with `defId` in place of a card) played until p1
 * ends a turn and p2's turn-start draw casts `defId` as it is drawn, with `also` holding too.
 * Deterministic: the same seeds in the same order, the first that produces it wins.
 */
function castOnDrawGame(defId: string, also: (events: readonly GameEvent[]) => boolean = () => true): Heard | null {
  const cheap = devDeck("cheap20");
  const p2Deck = [defId, ...cheap.filter((id) => id !== defId)].slice(0, cheap.length);
  for (let seed = 1; seed <= 80; seed += 1) {
    const game = realGame(`audio-moments-${defId}-${String(seed)}`, [cheap, p2Deck]);
    for (let step = 0; step < 16; step += 1) {
      answerPrompts(game);
      const who = game.actor();
      if (who === null) break;
      if (who === "p1") {
        const heard = hearAction(game, "p1", { type: "endTurn" }, true);
        if (castOnDrawOf(heard.events, defId) && also(heard.events)) return heard;
        continue;
      }
      game.act(who, { type: "endTurn" });
    }
  }
  return null;
}

const must = <T,>(value: T | null, what: string): T => {
  if (value === null) throw new Error(`expected ${what}`);
  return value;
};

describe("R506 against real games: #21 Hinder cast on draw", () => {
  const scenario = castOnDrawGame(HINDER_DEF_ID);

  it("R506 a real seed casts Hinder as p2 draws it", () => {
    expect(scenario, "a seed where p2's turn-start draw casts Hinder").not.toBeNull();
  });

  it("R506 both seats hear the cast-on-draw sting and the victim's mana crack", () => {
    const { seats } = must(scenario, "the scenario");
    for (const seat of [seats.p1, seats.p2]) {
      expect(ids(seat)).toContain("castOnDraw");
      expect(ids(seat)).toContain("manaCrack");
    }
  });

  it("R506 the crack answers the rider the engine puts on p1's next refresh", () => {
    const { fresh } = must(scenario, "the scenario");
    const rid = fresh.p1.find((e) => e.type === "modifierChanged" && e.modifierId === NEXT_REFRESH_MODIFIER_ID);
    expect(rid).toMatchObject({ player: "p1" });
  });
});

describe("R506 against real games: #27 Blood Ridden Glowy Jelly Bean cast on draw", () => {
  const scenario = castOnDrawGame(BLOOD_BEAN_DEF_ID, (events) => events.some((e) => e.type === "radiantSet" && e.zone.z === "hand"));

  it("R506 a real seed casts #27 as p2 draws it, and it turns a card in p2's hand Radiant", () => {
    expect(scenario, "a seed where p2's turn-start draw casts #27 onto a hand with a card in it").not.toBeNull();
  });

  it("R203 p1 cannot read the card that turned Radiant, and hears the same blood drain and gold burst as p2", () => {
    const { seats, fresh } = must(scenario, "the scenario");
    const hidden = fresh.p1.find((e) => e.type === "radiantSet" && e.zone.z === "hand");
    expect(hidden).toMatchObject({ instanceId: HIDDEN_DEF_ID, defId: HIDDEN_DEF_ID });
    const named = fresh.p2.find((e) => e.type === "radiantSet" && e.zone.z === "hand");
    expect(named?.type === "radiantSet" ? named.defId : HIDDEN_DEF_ID).not.toBe(HIDDEN_DEF_ID);

    const moment = (seat: Recorder): SfxId[] => ids(seat).filter((id) => id === "bloodDrain" || id === "goldBurst");
    expect(moment(seats.p1)).toEqual(expect.arrayContaining(["bloodDrain", "goldBurst"]));
    expect(moment(seats.p1)).toEqual(moment(seats.p2));
    expect(ids(seats.p1)).toContain("castOnDraw");
  });
});
