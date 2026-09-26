// Spec 28 — the v0.1.1 patch's presentation (issue #27; SPEC §10.10, R370–R374), on /dev/hotseat and
// the landing page, against `build:e2e` with no server.
//
// What it asserts, each off the DOM the view drew (CLAUDE.md rule 7):
//
//   A. Your own face-down trap (R371), with spec 03's decks and seed: player 2 sets #41 Sheepish,
//      and on player 2's own device the card is its face with `data-unrevealed="true"` and a "Face
//      down" tag whose tooltip says "Face down — your opponent can't see this card"; its hover
//      preview carries the same line over the face. The deck pile reads "Deck" and its preview
//      "Your deck" (R373). After the hand-over, player 1 sees a back labelled "Face-down trap",
//      whose hover shows the face-down overlay and names nothing (R370). Where the view gives the
//      back a cost (the engine's half of v0.1.1), the gem, the label and the overlay all state it
//      as "Cost (1)", Sheepish's; this branch's view may not carry it yet, and then none is drawn.
//   B. Combo-Index by its letter (R372), with `28-combo-a` (spec 03's player-1 deck with #93 in it)
//      and seed 28-combo-16, which deals #93 into player 1's opening hand: played on player 1's
//      second turn, its grade badge reads E and its face "Grade {E}"; its own play meets E's
//      threshold of one, so after that turn the badge reads D, on the other seat's board too.
//   C. The homescreen fan (R374): four real faces and a back; two visits with two different sources
//      of randomness deal two different hands, each of four rarities.
//
// Screenshots: `--expose shots=1` adds shots of A's face-down treatment and B's Combo-Index at
// 1280x720 and 390x844, under `<E2E_ARTIFACTS>/screenshots/28-patch-011-ui.cy.ts/28-patch-011-ui/`.
// Headless Chrome crops a tall capture, so launch it big enough:
//
//   E2E_WINDOW_SIZE=1600,1200 pnpm exec cypress run --browser chrome \
//     --spec cypress/e2e/28-patch-011-ui.cy.ts --expose shots=1

import { landingFanCardTestid, landingTestid } from "../../../apps/web/src/auth/testids.ts";
import { seedFor } from "../../support/config.ts";
import {
  INSPECT_FACE,
  INSPECT_FACE_DOWN,
  INSPECT_FACE_DOWN_COST,
  INSPECT_HOVER,
  INSPECT_LIST_HOVER,
  INSPECT_NOTE,
  cardId,
  libraryId,
  ts,
  unrevealedId,
  zoneId,
} from "../../support/testids.ts";

const TRAP_SEED = seedFor("03-sheep-19");
const COMBO_SEED = seedFor("28-combo-16");

const SHEEPISH = "Sheepish";
/** SPEC §8 #41's cost, which a back that carries one must state. */
const SHEEPISH_COST = 1;
const COMBO_INDEX = "Combo-Index";

const UNREVEALED_NOTE = "Face down — your opponent can't see this card";

const SHOTS = ["1", "true"].includes(String(Cypress.expose("shots") ?? ""));
const VIEWPORTS = [
  { label: "desktop-1280x720", width: 1280, height: 720 },
  { label: "phone-390x844", width: 390, height: 844 },
] as const;
type Viewport = (typeof VIEWPORTS)[number];

function shoot(viewport: Viewport | null, name: string): void {
  if (viewport === null) return;
  cy.screenshot(`28-patch-011-ui/${viewport.label}/${name}`, { capture: "viewport", overwrite: true });
}

function hover(selector: string): void {
  cy.get(selector).trigger("pointerover", { pointerType: "mouse" });
}

function unhover(selector: string): void {
  cy.get(selector).trigger("pointerout", { pointerType: "mouse" });
}

/** A: the face-down treatment on both seats, and the deck pile's words. */
function faceDownTrap(viewport: Viewport | null): void {
  if (viewport !== null) cy.viewport(viewport.width, viewport.height);
  cy.seedGame({ seed: TRAP_SEED, a: "03-plays-a", b: "03-sheepish-b" });
  cy.advanceToTurn(2);

  // Player 2 sets Sheepish face-down in backrow lane 3; its own device reads it (R33).
  cy.playByName(SHEEPISH, { zone: { side: "you", row: "backrow", lane: 3 } });
  cy.instanceAt("p2", "backrow", 3).then((trap) => {
    const own = ts(cardId(trap));
    cy.get(ts(zoneId("you", "backrow", 3))).find(own).should("have.attr", "data-unrevealed", "true");
    cy.get(ts(unrevealedId(trap)))
      .should("be.visible")
      .and("contain.text", "Face down")
      .and("have.attr", "title", UNREVEALED_NOTE);
    cy.get(ts(unrevealedId(trap))).find("img").should("exist");

    // Its hover preview carries the note over its face.
    hover(own);
    cy.get(ts(INSPECT_HOVER)).should("be.visible").find(ts(INSPECT_NOTE)).should("have.text", UNREVEALED_NOTE);
    cy.get(ts(INSPECT_HOVER)).find(ts(INSPECT_FACE)).should("contain.text", SHEEPISH);
    shoot(viewport, "own-face-down-trap-hover");
    unhover(own);
    cy.get(ts(INSPECT_HOVER)).should("not.exist");
    shoot(viewport, "own-face-down-trap");

    // R373: the deck pile says Deck, and its list says "Your deck".
    cy.get(ts(libraryId("you"))).find(".pile-label").should("have.text", "Deck");
    hover(ts(libraryId("you")));
    cy.get(ts(INSPECT_LIST_HOVER)).should("be.visible").and("contain.text", "Your deck");
    unhover(ts(libraryId("you")));

    // Player 1's turn: the same zone is a back, and nothing on it names the card.
    cy.advanceToTurn(3);
    const back = `${ts(zoneId("opponent", "backrow", 3))} .card-back`;
    cy.get(ts(zoneId("opponent", "backrow", 3))).find(own).should("not.exist");
    cy.get(back).should("have.attr", "data-face-down", "true").and("not.have.attr", "data-def-id");
    cy.get(back).invoke("attr", "aria-label").should("match", /^Face-down trap/).and("not.contain", SHEEPISH);
    hover(back);
    cy.get(ts(INSPECT_FACE_DOWN)).should("be.visible").and("contain.text", "Face-down trap").and("not.contain.text", SHEEPISH);
    cy.get(ts(INSPECT_FACE_DOWN)).find(".cf").should("not.exist");

    // R370: where the view gives the back a cost, the gem, the label and the overlay state it.
    cy.get(back).then(($back) => {
      const cost = $back.attr("data-facedown-cost");
      if (cost === undefined) {
        cy.get(back).find(".facedown-cost").should("not.exist");
        cy.get(ts(INSPECT_FACE_DOWN)).find(ts(INSPECT_FACE_DOWN_COST)).should("not.exist");
        return;
      }
      expect(Number(cost), "the back's cost is Sheepish's").to.eq(SHEEPISH_COST);
      cy.get(back).find(".facedown-cost").should("have.text", String(SHEEPISH_COST));
      cy.get(back).should("have.attr", "aria-label", `Face-down trap, Cost (${String(SHEEPISH_COST)})`);
      cy.get(ts(INSPECT_FACE_DOWN)).find(ts(INSPECT_FACE_DOWN_COST)).should("have.text", `Cost (${String(SHEEPISH_COST)})`);
    });
    shoot(viewport, "opponent-face-down-trap-hover");
    unhover(back);
    cy.get(ts(INSPECT_FACE_DOWN)).should("not.exist");
  });
}

/** B: Combo-Index's grade badge and face by letter, E then D. */
function comboIndex(viewport: Viewport | null): void {
  if (viewport !== null) cy.viewport(viewport.width, viewport.height);
  cy.seedGame({ seed: COMBO_SEED, a: "28-combo-a", b: "03-sheepish-b" });
  cy.advanceToTurn(3);

  cy.playByName(COMBO_INDEX, { zone: { side: "you", row: "backrow", lane: 1 } });
  cy.instanceAt("p1", "backrow", 1).then((combo) => {
    const card = ts(cardId(combo));
    const badge = `${card} [data-counter="grade"]`;
    cy.get(badge).should("have.text", "E").and("have.attr", "data-grade-letter", "E").and("have.attr", "title", "Grade E");

    hover(card);
    cy.get(ts(INSPECT_HOVER)).find(ts(INSPECT_FACE)).should("contain.text", "Grade {E} (starts at E).");
    cy.get(ts(INSPECT_HOVER)).find('.cf-value[data-label="Grade"]').should("have.text", "{E}");
    shoot(viewport, "combo-index-grade-e-hover");
    unhover(card);
    shoot(viewport, "combo-index-grade-e");

    // Its own play meets grade E's threshold of one, so the end of the turn raises it to D.
    cy.endTurn();
    cy.get(`${ts(zoneId("opponent", "backrow", 1))} ${card} [data-counter="grade"]`)
      .should("have.text", "D")
      .and("have.attr", "data-grade-letter", "D");
    hover(card);
    cy.get(ts(INSPECT_HOVER)).find('.cf-value[data-label="Grade"]').should("have.text", "{D}");
    shoot(viewport, "combo-index-grade-d-hover");
    unhover(card);
  });
}

/** The four dealt faces' ids, left to right. */
function fanIds(): Cypress.Chainable<string[]> {
  return cy.get(ts(landingTestid.fan)).then(($fan) =>
    [0, 1, 2, 3].map((index) => $fan.find(ts(landingFanCardTestid(index))).attr("data-def-id") ?? ""),
  );
}

/** A seeded `Math.random` for the landing page (mulberry32). */
function seededRandom(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function visitLanding(seed: number): void {
  cy.visit("/", {
    onBeforeLoad(win) {
      win.Math.random = seededRandom(seed);
    },
  });
  cy.get(ts(landingTestid.fan)).should("be.visible");
}

describe("Spec 28 — the v0.1.1 patch's board and homescreen (R370–R374)", () => {
  beforeEach(() => {
    // No server: the landing page's account check gets a plain "signed out".
    cy.intercept({ url: /\/api\// }, { statusCode: 401, body: { code: "unauthenticated", message: "signed out" } });
  });

  it("R371 R370 R373 your face-down trap is marked Face down, the other seat sees a back, and the pile reads Deck", () => {
    faceDownTrap(null);
  });

  it("R372 Combo-Index shows its grade by letter, E and then D, on both seats", () => {
    comboIndex(null);
  });

  it("R374 the homescreen fan deals four real cards and a back, and a new visit deals a new hand", () => {
    visitLanding(11);
    for (const index of [0, 1, 2, 3]) {
      cy.get(ts(landingFanCardTestid(index)))
        .should("have.attr", "data-face", "up")
        .find(".card-name")
        .invoke("text")
        .should("not.be.empty");
    }
    cy.get(ts(landingFanCardTestid(4))).should("have.attr", "data-face", "down");
    cy.get(ts(landingFanCardTestid(2))).should("have.attr", "data-radiant", "true");
    cy.get(`${ts(landingTestid.fan)} .cf[data-rarity]`).then(($faces) => {
      const rarities = new Set([...$faces].map((face) => face.getAttribute("data-rarity")));
      expect(rarities.size, "four rarities").to.eq(4);
    });
    fanIds().then((first) => {
      expect(new Set(first).size, "four different cards").to.eq(4);
      visitLanding(12);
      fanIds().should((second) => {
        expect(second, "a new visit deals a new hand").to.not.deep.eq(first);
      });
    });
  });

  if (SHOTS) {
    for (const viewport of VIEWPORTS) {
      it(`screenshots at ${viewport.label}: your face-down trap, the other seat's back, and Combo-Index`, () => {
        faceDownTrap(viewport);
        comboIndex(viewport);
      });
    }
  }
});
