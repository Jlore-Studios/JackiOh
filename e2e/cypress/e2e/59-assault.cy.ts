// 59 — the Infinite Assault (issue #4585, part G of #4576; docs/infinite-assault.md IA7, IA10): on the e2e
// build with no server, open `/assault`, take the day's Assault deck, Start, read level 1's briefing and
// open its fight, win level 1, reach level 2's briefing, reload and find the day's record and the open
// attempt kept on the device, then abandon and find the attempt ended and the record standing.
//
// The win reaches the attempt the way a finished fight's report does (R4733000): the fight's
// `fightOver`, held beside the kept attempt keyed to its attempt and level, is applied once on the next
// load. Playing a whole game to a win against the AI would make the spec long and tie it to one deal;
// the fight screen's own report is covered by `FightScreen.test.tsx`. Daily only, as every spec outside
// scripts/shard-specs.mjs's SMOKE.

import {
  ASSAULT_ATTEMPT_KEY,
  ASSAULT_HELD_KEY,
  ASSAULT_RECORDS_KEY,
  ASSAULT_STORAGE_VERSION,
} from "../../../apps/web/src/assault/config.ts";
import { assaultFightTestid } from "../../../apps/web/src/assault/fight/testids.ts";
import { assaultTestid } from "../../../apps/web/src/assault/testids.ts";
import { ts } from "../../support/testids.ts";

/** Mirrors `paths.assault` in apps/web/src/net/navigate.ts. */
const ASSAULT = "/assault";

/** The worker loads the engine, the card scripts and the AI before the fight's first board. */
const BOOT_TIMEOUT = 60_000;

/** A fresh device's first attempt of the day. */
const FIRST_ATTEMPT = 1;
const FIRST_LEVEL = 1;
const SECOND_LEVEL = 2;

/** The turns the held report says level 1 took; any count the attempt accepts. */
const WON_IN_TURNS = 9;

/** The day screen's ladder row opens with `Level {n}: `, then the level's enemy. */
const LADDER_PREFIX = /^Level \d+: /;

type StoredAttempt = {
  v: number;
  file: { setup: { date: string; source: string; startLevel: number; attempt: number }; actions: { kind: string }[] };
};
type StoredRecord = { date: string; best: number; checkpoints: number[]; attempts: number };
type StoredRecords = { v: number; days: Record<string, StoredRecord> };

function stored<T>(key: string): Cypress.Chainable<T | null> {
  return cy.window({ log: false }).then((win) => {
    const raw = win.localStorage.getItem(key);
    return cy.wrap<T | null>(raw === null ? null : (JSON.parse(raw) as T), { log: false });
  });
}

function storedAttempt(): Cypress.Chainable<StoredAttempt | null> {
  return stored<StoredAttempt>(ASSAULT_ATTEMPT_KEY);
}

/** The one day this device has played. */
function storedRecord(date: string): Cypress.Chainable<StoredRecord | undefined> {
  return stored<StoredRecords>(ASSAULT_RECORDS_KEY).then((records) => {
    expect(records?.v).to.equal(ASSAULT_STORAGE_VERSION);
    return cy.wrap<StoredRecord | undefined>(records?.days[date], { log: false });
  });
}

function expectBriefing(level: number): void {
  cy.get(ts(assaultTestid.briefing), { timeout: BOOT_TIMEOUT })
    .should("be.visible")
    .and("contain.text", `Level ${String(level)}`);
  cy.get(ts(assaultTestid.fightStart)).should("be.visible").and("be.enabled");
}

describe("59 Infinite Assault — the day's deck through level 1, kept over a reload, then abandoned", () => {
  beforeEach(() => {
    cy.clearLocalStorage();
  });

  it("starts on the day's deck, wins level 1, keeps the attempt over a reload and ends it on Abandon", () => {
    cy.visit(ASSAULT);
    // The page is a `display: contents` wrapper, which Cypress never counts as visible.
    cy.get(ts(assaultTestid.page), { timeout: BOOT_TIMEOUT }).should("exist");

    // The day screen: a fresh record, the ladder, and the three deck sources.
    cy.get(ts(assaultTestid.record)).should("contain.text", "Best: level 0").and("contain.text", "Attempts: 0");
    cy.get(ts(assaultTestid.deckDay)).should("be.visible");
    cy.get(ts(assaultTestid.deckPreset)).should("exist");
    cy.get(ts(assaultTestid.deckSaved)).should("exist");
    cy.get(ts(assaultTestid.level(SECOND_LEVEL))).should("exist").and("not.have.attr", "data-beaten");
    // `have.attr` hands on the attribute, not the element, so the row is fetched again to read it.
    cy.get(ts(assaultTestid.level(FIRST_LEVEL))).should("not.have.attr", "data-beaten");
    cy.get(ts(assaultTestid.level(FIRST_LEVEL)))
      .find("span")
      .first()
      .invoke("text")
      .then((row) => {
        const enemy = row.replace(LADDER_PREFIX, "").trim();
        expect(enemy, `level 1's enemy in "${row}"`).to.not.equal("");
        cy.wrap(enemy).as("enemy");
      });

    // The day's Assault deck, from level 1.
    cy.get(ts(assaultTestid.option("day", 0))).check();
    cy.get(ts(assaultTestid.startLevel(FIRST_LEVEL))).should("be.checked");
    cy.get(ts(assaultTestid.start)).click();

    // Level 1's briefing names the enemy the ladder showed.
    expectBriefing(FIRST_LEVEL);
    cy.get<string>("@enemy").then((enemy) => {
      cy.get(ts(assaultTestid.briefing)).should("contain.text", enemy);
    });
    storedAttempt().then((attempt) => {
      expect(attempt?.v).to.equal(ASSAULT_STORAGE_VERSION);
      expect(attempt?.file.setup).to.include({ source: "dayDeck", startLevel: FIRST_LEVEL, attempt: FIRST_ATTEMPT });
      expect(attempt?.file.actions).to.deep.equal([]);
    });

    // The fight: the practice board against the level's enemy.
    cy.get(ts(assaultTestid.fightStart)).click();
    cy.get(ts(assaultFightTestid.screen), { timeout: BOOT_TIMEOUT }).should("be.visible");
    cy.get<string>("@enemy").then((enemy) => {
      cy.get(ts(assaultFightTestid.enemy), { timeout: BOOT_TIMEOUT }).should("contain.text", enemy);
    });
    cy.get(ts(assaultFightTestid.level)).should("contain.text", String(FIRST_LEVEL));
    storedAttempt().then((attempt) => {
      expect(attempt?.file.actions.map((action) => action.kind)).to.deep.equal(["fight"]);
    });

    // Level 1 won: its report, held beside the kept attempt (R4733000), is applied on the next load.
    cy.window().then((win) => {
      win.localStorage.setItem(
        ASSAULT_HELD_KEY,
        JSON.stringify({
          attempt: FIRST_ATTEMPT,
          level: FIRST_LEVEL,
          action: { kind: "fightOver", won: true, turns: WON_IN_TURNS },
        }),
      );
    });
    cy.reload();
    expectBriefing(SECOND_LEVEL);
    stored<unknown>(ASSAULT_HELD_KEY).should("equal", null);

    // Reloaded again: the open attempt still waits on level 2's briefing and the record holds level 1.
    cy.reload();
    expectBriefing(SECOND_LEVEL);
    storedAttempt().then((attempt) => {
      expect(attempt?.file.actions.map((action) => action.kind)).to.deep.equal(["fight", "fightOver"]);
      const date = attempt?.file.setup.date ?? "";
      storedRecord(date).should("deep.include", { best: FIRST_LEVEL, attempts: FIRST_ATTEMPT });
      cy.wrap(date).as("date");
    });

    // Abandon: the attempt ends, and the day's record stands.
    cy.get(ts(assaultTestid.briefing)).contains("button", "Abandon attempt").click();
    cy.get(ts(assaultTestid.briefing)).find('[role="alertdialog"]').contains("button", "Abandon attempt").click();
    cy.get(ts(assaultTestid.result)).should("be.visible").and("contain.text", "Attempt abandoned");
    cy.get(ts(assaultTestid.record))
      .should("contain.text", `Best: level ${String(FIRST_LEVEL)}`)
      .and("contain.text", `Attempts: ${String(FIRST_ATTEMPT)}`);
    storedAttempt().should("equal", null);
    cy.get<string>("@date").then((date) => {
      storedRecord(date).should("deep.include", { best: FIRST_LEVEL, attempts: FIRST_ATTEMPT });
    });

    // Back on the day screen, after a reload too: level 1 beaten, no attempt open, a new one may start.
    cy.get(ts(assaultTestid.result)).contains("button", "Back to today").click();
    cy.reload();
    cy.get(ts(assaultTestid.page), { timeout: BOOT_TIMEOUT }).should("exist");
    cy.get(ts(assaultTestid.record))
      .should("contain.text", `Best: level ${String(FIRST_LEVEL)}`)
      .and("contain.text", `Attempts: ${String(FIRST_ATTEMPT)}`);
    cy.get(ts(assaultTestid.level(FIRST_LEVEL))).should("have.attr", "data-beaten", "true");
    cy.get(ts(assaultTestid.level(FIRST_LEVEL))).should("contain.text", "Beaten");
    cy.get(ts(assaultTestid.level(SECOND_LEVEL))).should("not.have.attr", "data-beaten");
    cy.get(ts(assaultTestid.level(SECOND_LEVEL))).should("not.contain.text", "Beaten");
    cy.get(ts(assaultTestid.start)).should("be.visible");
    cy.get(ts(assaultTestid.resume)).should("not.exist");
  });
});
