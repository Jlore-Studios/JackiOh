// R2800000 against the real engine (WebAssembly) and its `viewFor` redaction: Twisting Nether's hook
// streams its marker and its deaths as one run, which each seat plans as one wipe its set piece holds,
// on its base face and on both Radiant choices; the "enemy permanents" choice takes one side, the
// caster's opponent's, on both seats. Each seat is fed as Game feeds it: the new part of its view's
// window (`newEventsSince`), planned against the view before (`planEntries`, `setPieceEntries`, `planFx`).

import type { Action, ActionBody, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { devDeck, realPort } from "../audio/test/realGame.ts";
import { planFx } from "../fx/cues.ts";
import { createFxMemory } from "../fx/memory.ts";
import { hasSetPiece, wipeClaim } from "../fx/setPieces.ts";
import { SET_PIECE_HOLD_MS, newEventsSince, planEntries, setPieceEntries, type AnimationEntry } from "./animations.ts";
import { createPlayTracker } from "./runs.ts";

const SEATS = ["p1", "p2"] as const;
const NETHER = "core-088";
/** Core #4 Gary the Gambler (a 1/1), #53 (a 4/6) and #6 Mana Well (a Field Spell): nothing of theirs answers a death. */
const SMALL = "core-004";
const BIG = "core-053";
const WELL = "core-006";

const port = realPort();

/** A sandbox game at p1's main phase: a Unit of p1's, two of p2's and p2's Field Spell, and Nether in p1's hand. */
function netherGame(seed: string, radiant: boolean) {
  const cheap = devDeck("cheap20");
  let state = port.beginGame(port.createGame({ seed, decks: [cheap, cheap], sandbox: true } as Parameters<typeof port.createGame>[0])).state;
  let nonce = 0;
  const raw = (player: PlayerId, body: ActionBody): void => {
    nonce += 1;
    const result = port.reduce(state, { ...body, playerId: player, nonce: `r2800000-${String(nonce)}` } as Action);
    if (result.error !== undefined) throw new Error(`${player} ${body.type}: ${result.error}`);
    state = result.state;
  };
  const actor = (): PlayerId | undefined => SEATS.find((p) => port.legalActions(state, p).some((a) => a.type !== "concede" && a.type !== "offerDraw"));
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
  const sandbox = (op: unknown): void => raw("p1", { type: "sandbox", op } as unknown as ActionBody);
  sandbox({ kind: "setMana", player: "p1", current: 10, max: 10 });
  sandbox({ kind: "spawn", player: "p1", defId: SMALL, to: { zone: "field", row: "units", lane: 1 } });
  sandbox({ kind: "spawn", player: "p2", defId: SMALL, to: { zone: "field", row: "units", lane: 2 } });
  sandbox({ kind: "spawn", player: "p2", defId: BIG, to: { zone: "field", row: "units", lane: 4 } });
  sandbox({ kind: "spawn", player: "p2", defId: WELL, to: { zone: "field", row: "backrow", lane: 3 } });
  sandbox({ kind: "spawn", player: "p1", defId: NETHER, radiant, to: { zone: "hand" } });
  const hand = port.viewFor(state, "p1").you.hand;
  const nether = Array.isArray(hand) ? hand.find((c) => c.defId === NETHER) : undefined;
  return {
    /** Casts Nether with `modes` (none: its base face), and what each seat's board plans of it. */
    cast(modes?: readonly string[]): Record<PlayerId, { entries: AnimationEntry[]; before: PlayerView }> {
      const play = port
        .legalActions(state, "p1")
        .find((a) => a.type === "play" && a.instanceId === nether?.instanceId && JSON.stringify(a.modes ?? null) === JSON.stringify(modes ?? null));
      if (play === undefined) throw new Error(`no play of Nether with ${JSON.stringify(modes)}`);
      const before = { p1: port.viewFor(state, "p1"), p2: port.viewFor(state, "p2") };
      raw("p1", play);
      const plan = (seat: PlayerId) => {
        const fresh = newEventsSince(before[seat].events, port.viewFor(state, seat).events);
        const planned = planEntries(fresh, before[seat], false, createPlayTracker(), hasSetPiece);
        return { entries: setPieceEntries(planned, wipeClaim, { reduced: false, bigMoments: "full", speed: 1 }), before: before[seat] };
      };
      return { p1: plan("p1"), p2: plan("p2") };
    },
  };
}

/** The side of the board the caster's opponent's cards stand on, as `seat` sees it. */
const enemyOf = (seat: PlayerId): "you" | "opponent" => (seat === "p1" ? "opponent" : "you");

describe("R2800000 against the real engine: Twisting Nether's wipe", () => {
  it("R2800000 Nether plays one held wipe on both faces and seats; enemy-only names one side", () => {
    const casts = [
      { label: "base", heard: netherGame("r2800000-base", false).cast(), sides: () => ["you", "opponent"], taken: 4 },
      { label: "radiant all", heard: netherGame("r2800000-all", true).cast(["all"]), sides: () => ["you", "opponent"], taken: 4 },
      { label: "radiant enemy", heard: netherGame("r2800000-enemy", true).cast(["enemy"]), sides: (seat: PlayerId) => [enemyOf(seat)], taken: 3 },
    ];
    for (const { label, heard, sides, taken } of casts) {
      for (const seat of SEATS) {
        const where = `${label} on ${seat}`;
        const { entries, before } = heard[seat];
        const wipes = entries.filter((entry) => entry.wipe !== undefined);
        expect(wipes, where).toHaveLength(1);
        const [wipe] = wipes;
        expect(wipe?.events.map((e) => e.type), where).toEqual(Array.from({ length: taken }, () => "destroyed"));
        expect(wipe?.wipe?.by, where).toBe(NETHER);
        expect(wipe?.wipe?.sides, where).toEqual(sides(seat));
        expect(wipe?.setPiece, where).toEqual({ key: "nether", holdMs: SET_PIECE_HOLD_MS });
        // No removal plays apart from it, and the action holds once.
        expect(entries.filter((entry) => entry.wipe === undefined && entry.events.some((e) => e.type === "destroyed")), where).toEqual([]);
        expect(entries.filter((entry) => entry.setPiece !== undefined), where).toHaveLength(1);
        // The effects layer draws its vortex over the sides it takes, and each card it takes is drawn in.
        const cues = wipe === undefined ? [] : planFx(wipe, before, { intensity: 1, card: () => undefined, memory: createFxMemory() });
        const piece = cues.find((cue) => cue.kind === "setPiece");
        expect(piece, where).toMatchObject({ shape: "vortex", side: sides(seat).length === 1 ? sides(seat)[0] : null });
        expect(cues.filter((cue) => cue.kind === "pull"), where).toHaveLength(taken);
      }
    }
  });
});
