// B38 and B39 audio on a Chrome hotseat board (docs/polish/2-sound.md).
// Assert voice requests, not playback: headless audio outputs differ, but accepted cues are stable.
// BUILD M8: seed the game, avoid fixed waits, and use `ts()` selectors.
// The choice-free 01-aggro decks let the test end turns until a unit is offered.

import { seedFor } from "../../support/config.ts";
import { BOARD, LEGAL, cardId, handCardId, ts, zoneId } from "../../support/testids.ts";
import type { GameStateLike, Lane, PlayerId } from "../../support/types.ts";

const SEED = seedFor("15-audio");
const DECK_A = "01-aggro-a";
const DECK_B = "01-aggro-b";

const AUDIO_TOGGLE = "audio-toggle";
const AUDIO_SETTINGS_KEY = "jackioh.audio.v1";
const CORE_004_PLAY_URL = "/audio/voice/core-004-play.m4a";

/** Unit ids in the 01-aggro decks; spells use a cast line rather than a play line. */
const UNIT_DEF_IDS: ReadonlySet<string> = new Set([
  "core-001", "core-003", "core-008", "core-011", "core-015", "core-019", "core-020", "core-025",
  "core-032", "core-045", "core-053", "core-056", "core-077", "core-081", "core-089", "core-091",
  "core-092",
]);

/** Player-turn budget for finding an offered unit. */
const TURN_BUDGET = 8;

const LANES: readonly Lane[] = [1, 2, 3, 4, 5];

// Locally declared `window.__jackiohAudio` (`e2e/` does not import `apps/`)

type VoiceLineKind = "play" | "attack" | "death" | "cast";

type PlayedCueLike =
  | { kind: "sfx"; id: string; params?: { amount?: number; mine?: boolean }; delayMs: number; atMs: number }
  | { kind: "voice"; defId: string; line: VoiceLineKind; delayMs: number; atMs: number; outcome: string }
  // R655: a card's own effect on one of its hooks.
  | { kind: "effect"; defId: string; hook: VoiceLineKind; effect: string; delayMs: number; atMs: number };

type AudioDebugHandleLike = {
  state(): string;
  log(): readonly PlayedCueLike[];
  clearLog(): void;
  contextsCreated(): number;
};

function audioHandle(win: Cypress.AUTWindow): AudioDebugHandleLike | undefined {
  return (win as unknown as { __jackiohAudio?: AudioDebugHandleLike }).__jackiohAudio;
}

/** Re-read the handle on retry so a hotseat remount cannot leave a stale assertion. */
function expectAudio(check: (audio: AudioDebugHandleLike) => void): void {
  cy.window({ log: false }).should((win) => {
    const audio = audioHandle(win);
    expect(audio, "window.__jackiohAudio (outside production builds)").to.not.eq(undefined);
    if (audio !== undefined) check(audio);
  });
}

function clearAudioLog(): void {
  cy.window({ log: false }).then((win) => {
    const audio = audioHandle(win);
    expect(audio, "window.__jackiohAudio (outside production builds)").to.not.eq(undefined);
    audio?.clearLog();
  });
}

function voiceCues(audio: AudioDebugHandleLike): Extract<PlayedCueLike, { kind: "voice" }>[] {
  return audio.log().filter((cue): cue is Extract<PlayedCueLike, { kind: "voice" }> => cue.kind === "voice");
}

// Playing a unit through the UI

type HandCard = { id: string; defId: string };

function handOf(state: GameStateLike, player: PlayerId): HandCard[] {
  const side = state.players[player] as { hand?: HandCard[] };
  return side.hand ?? [];
}

/** The first target the client marks legal. */
function firstLegal(testids: readonly string[]): Cypress.Chainable<string | null> {
  return cy.get("body", { log: false }).then(($body) => {
    const found = testids.find((testid) => $body.find(`${ts(testid)}${LEGAL}`).length > 0);
    return cy.wrap(found ?? null, { log: false });
  });
}

/** BUILD M5-T3: hand the device to the acting seat. */
function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    if (handle.seat !== undefined && handle.seat !== player) cy.handOver();
  });
}

type Played = { instanceId: string; defId: string };

/** Play an offered unit, ending turns until one appears; clear the log immediately before its click. */
function playOfferedUnit(played: Played, beforePlay: () => void, turnsLeft = TURN_BUDGET): void {
  cy.gameState().then((state) => {
    expect(turnsLeft, "the client offered a unit inside the turn budget").to.be.greaterThan(0);
    expect(state.result, "the game is still running").to.eq(null);
    const seat = state.active;
    ensureSeat(seat);
    const units = handOf(state, seat).filter((card) => UNIT_DEF_IDS.has(card.defId));

    firstLegal(units.map((card) => handCardId(card.id))).then((testid) => {
      if (testid === null) {
        // R82: a turn with nothing left to do may already have ended itself.
        cy.gameState().then((now) => {
          if (now.active === seat) cy.endTurn();
          else cy.handOver();
        });
        playOfferedUnit(played, beforePlay, turnsLeft - 1);
        return;
      }

      const card = units.find((unit) => handCardId(unit.id) === testid);
      expect(card, `the offered hand card ${testid}`).to.not.eq(undefined);
      played.instanceId = card?.id ?? "";
      played.defId = card?.defId ?? "";

      beforePlay();
      cy.get(ts(testid)).click();
      cy.settled();
      // R81: the zone travels in the `play` action and a board click finishes it (BUILD M5-T2).
      firstLegal(LANES.map((lane) => zoneId("you", "units", lane))).then((zone) => {
        if (zone !== null) {
          cy.get(ts(zone)).click();
          cy.settled();
        }
      });
      cy.get(ts(testid)).should("not.exist");
      cy.then(() => {
        cy.get(ts(cardId(played.instanceId))).should("exist");
      });
    });
  });
}

describe("polish 2 — audio on the hotseat board", () => {
  it("B38 no AudioContext exists before a gesture, and one click on the board makes exactly one", () => {
    // `manual` avoids an unlocking gesture before the assertion.
    cy.seedGame({ seed: SEED, a: DECK_A, b: DECK_B, mulligan: "manual" });

    expectAudio((audio) => {
      expect(audio.contextsCreated(), "no AudioContext before any gesture").to.eq(0);
      expect(audio.state(), "the engine waits for a gesture").to.eq("locked");
      expect(audio.log(), "nothing is accepted before a gesture").to.have.length(0);
    });

    // Board padding avoids the mulligan panel and toggle; the prompt scrim is click-through.
    cy.get(ts(BOARD)).click(4, 4);

    expectAudio((audio) => {
      expect(audio.contextsCreated(), "the first gesture constructs one AudioContext").to.eq(1);
      expect(audio.state(), "the engine has left locked").to.not.be.oneOf(["locked", "unsupported"]);
    });

    cy.keepMulligans();
    expectAudio((audio) => {
      expect(audio.contextsCreated(), "still exactly one AudioContext").to.eq(1);
    });
  });

  it("B38 playing a unit from hand through the UI logs a voice request for its play line", () => {
    // `keep` answers through the UI, supplying the unlocking gesture.
    cy.seedGame({ seed: SEED, a: DECK_A, b: DECK_B });
    expectAudio((audio) => {
      expect(audio.contextsCreated(), "the mulligan clicks unlocked audio").to.eq(1);
      expect(audio.state()).to.not.be.oneOf(["locked", "unsupported"]);
    });

    const played: Played = { instanceId: "", defId: "" };
    playOfferedUnit(played, clearAudioLog);

    expectAudio((audio) => {
      const lines = voiceCues(audio).filter((cue) => cue.defId === played.defId && cue.line === "play");
      expect(played.defId, "a unit was played").to.not.eq("");
      expect(
        lines,
        `one voice request for ${played.defId}'s play line (log: ${JSON.stringify(audio.log())})`,
      ).to.have.length(1);
    });
  });

  it("B39 a mute survives a reload, and a unit played while muted logs nothing", () => {
    cy.seedGame({ seed: SEED, a: DECK_A, b: DECK_B });

    cy.get(ts(AUDIO_TOGGLE)).should("have.attr", "aria-pressed", "false");
    cy.get(ts(AUDIO_TOGGLE)).click();
    cy.get(ts(AUDIO_TOGGLE)).should("have.attr", "aria-pressed", "true");
    cy.window({ log: false }).should((win) => {
      const stored = JSON.parse(win.localStorage.getItem(AUDIO_SETTINGS_KEY) ?? "null") as { muted?: unknown } | null;
      expect(stored?.muted, `localStorage["${AUDIO_SETTINGS_KEY}"].muted`).to.eq(true);
    });

    // The hotseat route restores decks from `seedGame`'s localStorage copy.
    cy.reload();
    cy.jackioh().should((handle) => {
      expect(handle.seed, "the same seeded game after the reload").to.eq(SEED);
    });
    cy.settled();
    cy.get(ts(AUDIO_TOGGLE)).should("have.attr", "aria-pressed", "true");

    // Gestures unlock muted audio, so an empty log proves mute rather than lock.
    cy.keepMulligans();
    expectAudio((audio) => {
      expect(audio.contextsCreated(), "the mulligan clicks unlocked audio").to.eq(1);
      expect(audio.state()).to.not.be.oneOf(["locked", "unsupported"]);
      expect(audio.log(), "muted: not even the mulligan clicks are accepted").to.have.length(0);
    });

    const played: Played = { instanceId: "", defId: "" };
    playOfferedUnit(played, clearAudioLog);

    cy.get(ts(AUDIO_TOGGLE)).should("have.attr", "aria-pressed", "true");
    expectAudio((audio) => {
      expect(played.defId, "a unit was played").to.not.eq("");
      expect(audio.log(), `muted: playing ${played.defId} appends nothing`).to.deep.eq([]);
    });
  });

  it("B39 GET /audio/voice/core-004-play.m4a answers 200 with an audio content type", () => {
    cy.request({ url: CORE_004_PLAY_URL, encoding: "binary" }).then((response) => {
      expect(response.status, `GET ${CORE_004_PLAY_URL}`).to.eq(200);
      const header: unknown = response.headers["content-type"];
      const contentType = Array.isArray(header) ? header.join(", ") : String(header ?? "");
      expect(contentType, "content-type").to.match(/^audio\//);
      expect(String(response.body).length, "a non-empty body").to.be.greaterThan(0);
    });
  });
});
