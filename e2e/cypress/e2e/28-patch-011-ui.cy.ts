// Spec 28 checks the DOM only (CLAUDE.md rule 7; SPEC §10.10).
// Face-down cost is SPEC §8 #41 (R351, R370, R432); Combo-Index relies on R82 and R372.
// Face-down traps (R371, R373), Combo-Index, and the landing fan (R374) are covered here.
// `--expose shots=1` captures treatments; use 1600x1200 because Headless Chrome crops tall viewports.

import { landingFanCardTestid, landingTestid } from "../../../apps/web/src/auth/testids.ts";
import { FX_SETTINGS_KEY, seedFor } from "../../support/config.ts";
import {
  INSPECT_FACE,
  INSPECT_FACE_DOWN,
  INSPECT_FACE_DOWN_COST,
  INSPECT_HOVER,
  INSPECT_LIST_HOVER,
  INSPECT_NOTE,
  SHOWCASE,
  cardId,
  libraryId,
  ts,
  unrevealedId,
  zoneId,
} from "../../support/testids.ts";

const TRAP_SEED = seedFor("03-sheep-19");
const COMBO_SEED = seedFor("28-combo-16");

const SHEEPISH = "Sheepish";
const SHEEPISH_COST = 1;
const COMBO_INDEX = "Combo-Index";
const THRESHOLD_LABEL = "N = the grades from E to the current one";

const UNREVEALED_NOTE = "Face down — your opponent can't see this card";

const SHOTS = ["1", "true"].includes(String(Cypress.expose("shots") ?? ""));
const VIEWPORTS = [
  { label: "desktop-1280x720", width: 1280, height: 720 },
  { label: "phone-390x844", width: 390, height: 844 },
] as const;
type Viewport = (typeof VIEWPORTS)[number];

/** R200: screenshot passes turn effects off. */
const QUIET_FX = { speed: 1, intensity: "off", motion: "system" } as const;

function quiet(viewport: Viewport | null): { onBeforeLoad?: (win: Cypress.AUTWindow) => void } {
  return viewport === null ? {} : { onBeforeLoad: (win) => win.localStorage.setItem(FX_SETTINGS_KEY, JSON.stringify(QUIET_FX)) };
}

function shoot(viewport: Viewport | null, name: string): void {
  if (viewport === null) return;
  // Wait for the opponent's showcase animation.
  cy.get(ts(SHOWCASE)).should("not.exist");
  cy.screenshot(`28-patch-011-ui/${viewport.label}/${name}`, { capture: "viewport", overwrite: true });
}

function hover(selector: string): void {
  cy.get(selector).trigger("pointerover", { pointerType: "mouse" });
}

function unhover(selector: string): void {
  cy.get(selector).trigger("pointerout", { pointerType: "mouse" });
}

function faceDownTrap(viewport: Viewport | null): void {
  if (viewport !== null) cy.viewport(viewport.width, viewport.height);
  cy.seedGame({ seed: TRAP_SEED, a: "03-plays-a", b: "03-sheepish-b", ...quiet(viewport) });
  cy.advanceToTurn(2);

  // R33: the owner's device shows a set Sheepish.
  cy.playByName(SHEEPISH, { zone: { side: "you", row: "backrow", lane: 3 } });
  cy.instanceAt("p2", "backrow", 3).then((trap) => {
    const own = ts(cardId(trap));
    cy.get(ts(zoneId("you", "backrow", 3))).find(own).should("have.attr", "data-unrevealed", "true");
    cy.get(ts(unrevealedId(trap)))
      .should("be.visible")
      .and("contain.text", "Face down")
      .and("have.attr", "title", UNREVEALED_NOTE);
    cy.get(ts(unrevealedId(trap))).find("img").should("exist");

    hover(own);
    cy.get(ts(INSPECT_HOVER)).should("be.visible").find(ts(INSPECT_NOTE)).should("have.text", UNREVEALED_NOTE);
    cy.get(ts(INSPECT_HOVER)).find(ts(INSPECT_FACE)).should("contain.text", SHEEPISH);
    shoot(viewport, "own-face-down-trap-hover");
    unhover(own);
    cy.get(ts(INSPECT_HOVER)).should("not.exist");
    shoot(viewport, "own-face-down-trap");

    cy.get(ts(libraryId("you"))).find(".pile-label").should("have.text", "Deck");
    hover(ts(libraryId("you")));
    cy.get(ts(INSPECT_LIST_HOVER)).should("be.visible").and("contain.text", "Your deck");
    unhover(ts(libraryId("you")));

    cy.advanceToTurn(3);
    // R227: wait for the showcase.
    cy.get(ts(SHOWCASE)).should("not.exist");
    const back = `${ts(zoneId("opponent", "backrow", 3))} .card-back`;
    cy.get(ts(zoneId("opponent", "backrow", 3))).find(own).should("not.exist");
    cy.get(back).should("have.attr", "data-face-down", "true").and("not.have.attr", "data-def-id");
    cy.get(back).invoke("attr", "aria-label").should("match", /^Face-down trap/).and("not.contain", SHEEPISH);
    hover(back);
    cy.get(ts(INSPECT_FACE_DOWN)).should("be.visible").and("contain.text", "Face-down trap").and("not.contain.text", SHEEPISH);
    cy.get(ts(INSPECT_FACE_DOWN)).find(".cf").should("not.exist");

    cy.get(back).should("have.attr", "data-facedown-cost", String(SHEEPISH_COST));
    cy.get(back).find(".facedown-cost").should("have.text", String(SHEEPISH_COST));
    cy.get(back).should("have.attr", "aria-label", `Face-down trap, (${String(SHEEPISH_COST)}) Cost`);
    cy.get(ts(INSPECT_FACE_DOWN)).find(ts(INSPECT_FACE_DOWN_COST)).should("have.text", `(${String(SHEEPISH_COST)}) Cost`);
    shoot(viewport, "opponent-face-down-trap-hover");
    unhover(back);
    cy.get(ts(INSPECT_FACE_DOWN)).should("not.exist");
  });
}

function comboIndex(viewport: Viewport | null): void {
  if (viewport !== null) cy.viewport(viewport.width, viewport.height);
  cy.seedGame({ seed: COMBO_SEED, a: "28-combo-a", b: "03-sheepish-b", ...quiet(viewport) });
  cy.advanceToTurn(5);

  cy.playByName(COMBO_INDEX, { zone: { side: "you", row: "backrow", lane: 1 } });
  cy.instanceAt("p1", "backrow", 1).then((combo) => {
    const card = ts(cardId(combo));
    const badge = `${card} [data-counter="grade"]`;
    cy.get(badge).should("have.text", "E").and("have.attr", "data-grade-letter", "E").and("have.attr", "title", "Grade E");

    hover(card);
    cy.get(ts(INSPECT_HOVER)).find(ts(INSPECT_FACE)).should("contain.text", "Grade {E} (starts at E).");
    cy.get(ts(INSPECT_HOVER)).find('.cf-value[data-label="Grade"]').should("have.text", "{E}");
    cy.get(ts(INSPECT_HOVER)).find(`.cf-value[data-label="${THRESHOLD_LABEL}"]`).should("have.text", "{1}");
    shoot(viewport, "combo-index-grade-e-hover");
    unhover(card);
    shoot(viewport, "combo-index-grade-e");

    // Grade E's threshold is one, so end turn advances it to D.
    cy.endTurn();
    cy.jackioh().its("seat").should("eq", "p2");
    cy.get(ts(zoneId("opponent", "backrow", 1)))
      .find(`${card} [data-counter="grade"]`)
      .should("have.text", "D")
      .and("have.attr", "data-grade-letter", "D");
    hover(card);
    cy.get(ts(INSPECT_HOVER)).find('.cf-value[data-label="Grade"]').should("have.text", "{D}");
    shoot(viewport, "combo-index-grade-d-hover");
    unhover(card);
  });
}

function fanIds(): Cypress.Chainable<string[]> {
  return cy.get(ts(landingTestid.fan)).then(($fan) =>
    [0, 1, 2, 3].map((index) => $fan.find(ts(landingFanCardTestid(index))).attr("data-def-id") ?? ""),
  );
}

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
    // Without a server, the account check is signed out.
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
