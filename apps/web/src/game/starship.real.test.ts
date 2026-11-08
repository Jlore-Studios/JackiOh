// R2801000 against the real engine (WebAssembly) and its `viewFor` redaction: two Starship Pieces dying
// to one Twisting Nether fuse, one after the other, into the Dormant Starship the pass summons
// (R1852000), which each seat plans as one fuse run of two pieces; then the Starship's launch by its
// Activate (R1852003) is the Starship's set piece on both seats, held for its beat. Each seat is fed as
// Game feeds it: the new part of its view's window (`newEventsSince`), planned against the view before
// (`planEntries`, `setPieceEntries`, `planFx`).

import type { Action, ActionBody, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { devDeck, realPort } from "../audio/test/realGame.ts";
import { planFx } from "../fx/cues.ts";
import { createFxMemory } from "../fx/memory.ts";
import { hasSetPiece, wipeClaim } from "../fx/setPieces.ts";
import { starshipClaim } from "../fx/starship.ts";
import { STARSHIP_LAUNCH_HOLD_MS, STARSHIP_SET_PIECE, newEventsSince, planEntries, setPieceEntries, starshipFuseMs, type AnimationEntry } from "./animations.ts";
import { createPlayTracker } from "./runs.ts";

const SEATS = ["p1", "p2"] as const;
const NETHER = "core-088";
const RUSH_LAUNCHER = "stinkyscape-140";
const FRUIT_LAUNCHER = "stinkyscape-141";

const port = realPort();

/** A sandbox game at p1's main phase: two Launchers of p1's on the field and Nether in p1's hand. */
function starshipGame(seed: string) {
  const cheap = devDeck("cheap20");
  let state = port.beginGame(port.createGame({ seed, decks: [cheap, cheap], sandbox: true } as Parameters<typeof port.createGame>[0])).state;
  let nonce = 0;
  const raw = (player: PlayerId, body: ActionBody): void => {
    nonce += 1;
    const result = port.reduce(state, { ...body, playerId: player, nonce: `r2801000-${String(nonce)}` } as Action);
    if (result.error !== undefined) throw new Error(`${player} ${body.type}: ${result.error}`);
    state = result.state;
  };
  const actor = (): PlayerId | undefined => SEATS.find((p) => port.legalActions(state, p).some((a) => a.type !== "concede" && a.type !== "offerDraw"));
  /** Plays on (mulligans, answers, turn ends) until p1's main phase with nothing pending. */
  const toMain = (): void => {
    for (let step = 0; step < 40; step += 1) {
      const v = port.viewFor(state, "p1");
      if (v.active === "p1" && v.phase === "main" && v.pending === null) return;
      const who = actor();
      if (who === undefined) return;
      const legal = port.legalActions(state, who);
      const hand = port.viewFor(state, who).you.hand;
      if (legal.some((a) => a.type === "mulligan")) raw(who, { type: "mulligan", keep: Array.isArray(hand) ? hand.map((c) => c.instanceId) : [] });
      else raw(who, legal.find((a) => a.type === "answer") ?? { type: "endTurn" });
    }
  };
  toMain();
  const sandbox = (op: unknown): void => raw("p1", { type: "sandbox", op } as unknown as ActionBody);
  sandbox({ kind: "setMana", player: "p1", current: 10, max: 10 });
  sandbox({ kind: "spawn", player: "p1", defId: RUSH_LAUNCHER, to: { zone: "field", row: "units", lane: 1 } });
  sandbox({ kind: "spawn", player: "p1", defId: FRUIT_LAUNCHER, to: { zone: "field", row: "units", lane: 2 } });
  sandbox({ kind: "spawn", player: "p1", defId: NETHER, to: { zone: "hand" } });

  /** What each seat's board plans of `act`. */
  const heard = (act: () => void): Record<PlayerId, { entries: AnimationEntry[]; before: PlayerView }> => {
    const before = { p1: port.viewFor(state, "p1"), p2: port.viewFor(state, "p2") };
    act();
    const plan = (seat: PlayerId) => {
      const fresh = newEventsSince(before[seat].events, port.viewFor(state, seat).events);
      const planned = planEntries(fresh, before[seat], false, createPlayTracker(), hasSetPiece);
      const claimOf = (entry: AnimationEntry) => wipeClaim(entry) ?? starshipClaim(entry);
      return { entries: setPieceEntries(planned, claimOf, { reduced: false, bigMoments: "full", speed: 1 }), before: before[seat] };
    };
    return { p1: plan("p1"), p2: plan("p2") };
  };

  return {
    /** Casts Nether over the Launchers. */
    nether() {
      const hand = port.viewFor(state, "p1").you.hand;
      const card = Array.isArray(hand) ? hand.find((c) => c.defId === NETHER) : undefined;
      const play = port.legalActions(state, "p1").find((a) => a.type === "play" && a.instanceId === card?.instanceId);
      if (play === undefined) throw new Error("no play of Nether");
      return heard(() => {
        raw("p1", play);
      });
    },
    /** Launches p1's Starship by its Activate, passing both turns first if it may not launch yet. */
    launch() {
      const find = () => port.legalActions(state, "p1").find((a) => a.type === "activate" && a.ability === "launch");
      if (find() === undefined) {
        raw("p1", { type: "endTurn" });
        toMain();
      }
      sandbox({ kind: "setMana", player: "p1", current: 10, max: 10 });
      const activate = find();
      if (activate === undefined) throw new Error("no launch to activate");
      return heard(() => {
        raw("p1", activate);
      });
    },
  };
}

describe("R2801000 against the real engine: the Starship", () => {
  it("R2801000 two pieces dying at once are one fuse run on both seats; the launch holds as starship on both", () => {
    const game = starshipGame("r2801000-starship");
    const fused = game.nether();
    for (const seat of SEATS) {
      const { entries, before } = fused[seat];
      const runs = entries.filter((entry) => entry.fuse !== undefined);
      expect(runs, seat).toHaveLength(1);
      const [run] = runs;
      expect(run?.events.map((e) => e.type), seat).toEqual(["fused", "starshipFused", "fused", "starshipFused"]);
      expect(run?.fuse?.pieces, seat).toHaveLength(2);
      expect(run?.fuse?.count, seat).toBe(2);
      expect(run?.durationMs, seat).toBe(starshipFuseMs(2));
      // The pass summoned the Starship in this action: the run centres on the zone it entered.
      expect(run?.fuse?.anchor, seat).toMatch(/^zone-/u);
      // The wipe holds this action; the fuse run holds nothing.
      expect(run?.setPiece, seat).toBeUndefined();
      expect(entries.filter((entry) => entry.setPiece !== undefined), seat).toHaveLength(1);
      // R3334000: each piece's module flies to its hardpoint on the Starship and is welded on.
      const cues = run === undefined ? [] : planFx(run, before, { intensity: 1, card: () => undefined, memory: createFxMemory() });
      expect(cues.filter((cue) => cue.kind === "module" && cue.motion === "dock"), seat).toHaveLength(2);
      expect(cues.filter((cue) => cue.kind === "weld"), seat).toHaveLength(2);
    }

    const launched = game.launch();
    for (const seat of SEATS) {
      const { entries, before } = launched[seat];
      const launches = entries.filter((entry) => entry.launch !== undefined);
      expect(launches, seat).toHaveLength(1);
      const [launch] = launches;
      expect(launch?.launch?.pieces, seat).toBe(2);
      expect(launch?.setPiece, seat).toEqual({ key: STARSHIP_SET_PIECE, holdMs: STARSHIP_LAUNCH_HOLD_MS });
      expect(entries.filter((entry) => entry.setPiece !== undefined), seat).toHaveLength(1);
      // Its awakening, which breaks the seal (#2435), plays just before it.
      const at = launch === undefined ? -1 : entries.indexOf(launch);
      expect(entries[at - 1]?.type, seat).toBe("awakened");
      const cues = launch === undefined ? [] : planFx(launch, before, { intensity: 1, card: () => undefined, memory: createFxMemory() });
      // R3334001: no countdown; the cinematic shows the ship's picture and the name it launched under.
      expect(cues.find((cue) => cue.kind === "launch"), seat).toMatchObject({ size: "small", countsMs: [] });
      const launchedEvent = launch?.events.find((event) => event.type === "starshipLaunched");
      const name = launchedEvent?.type === "starshipLaunched" ? launchedEvent.name : undefined;
      expect(name, seat).toBeTruthy();
      expect(cues.find((cue) => cue.kind === "cinematic"), seat).toMatchObject({ name, size: "small" });
    }
  });
});
