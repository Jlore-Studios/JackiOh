// Trio-code copy/import and capacity refusal (R339–R341).
// `e2e-p2` supplies one installed trio; `e2e-p1` starts empty, proving this import created every listing.
// Read the code from the output field because Cypress may deny clipboard access. See e2e/README.md.
// BUILD M6 supplies deck endpoints; L4 forbids cards shared between trio decks.

import { MAX_SAVED_DECKS, MAX_SAVED_TRIOS } from "../../../apps/web/src/wire/serverConfig.ts";
import { INSTALLED_DECK_NAMES, INSTALLED_TRIO_NAME, mintId, type InstalledLoadout } from "../../support/commands.ts";
import { accounts, constants, routes, server, timeouts, type E2EAccount } from "../../support/config.ts";
import {
  DECK_LIST,
  TRIO_CODE_OUTPUT,
  TRIO_COPY_CODE,
  TRIO_EDITOR,
  TRIO_IMPORT,
  TRIO_IMPORT_CAP_REASON,
  TRIO_IMPORT_INPUT,
  TRIO_IMPORT_OPEN,
  TRIO_IMPORT_PREVIEW,
  TRIO_IMPORT_SHARED,
  TRIO_IMPORT_SUBMIT,
  TRIO_NAME_INPUT,
  TRIO_VERDICT,
  WORKSHOP,
  deckRowId,
  trioImportSlotId,
  trioRowId,
  ts,
} from "../../support/testids.ts";

const DECK_SIZE = constants.DECK_SIZE;

const DECK_ROWS = `${ts(DECK_LIST)} [data-testid^="${deckRowId("")}"]`;

type ErrorBody = { error: { code: string; message: string; details?: unknown } };

function api(path: string): string {
  return `${server.http()}${path}`;
}

function bearer(account: E2EAccount): Record<string, string> {
  return { authorization: `Bearer ${account.token}` };
}

function openWorkshop(account: E2EAccount): void {
  cy.visitAs(account, routes.deckbuilder());
  cy.get(ts(WORKSHOP), { timeout: timeouts.view }).should("exist");
}

function copiedTrioCode(): Cypress.Chainable<{ code: string; installed: InstalledLoadout }> {
  const exporter = accounts.p2();
  return cy.installLoadout(exporter, "19-modes-b").then((installed) => {
    openWorkshop(exporter);
    cy.get(ts(trioRowId(installed.trioId)), { timeout: timeouts.view }).click();
    cy.get(ts(TRIO_EDITOR)).should("have.attr", "data-trio", installed.trioId);
    cy.get(ts(TRIO_COPY_CODE)).click();
    return cy
      .get(ts(TRIO_CODE_OUTPUT))
      .invoke("val")
      .then((value) => {
        const code = String(value);
        expect(code, "R339: a trio code").to.match(/^JKT\d+\.[A-Za-z0-9_-]+$/u);
        return { code, installed };
      });
  });
}

function pasteTrioCode(code: string): void {
  cy.get(ts(TRIO_IMPORT_OPEN), { timeout: timeouts.view }).click();
  cy.get(ts(TRIO_IMPORT)).should("be.visible");
  cy.get(ts(TRIO_IMPORT_INPUT)).type(code, { delay: 0 });
}

describe("26 trio codes — copy, import, and the caps (R339–R341)", () => {
  beforeEach(() => {
    cy.clearDecks(accounts.p1());
  });

  it("R339 R341 a copied trio code imports into another account as three new decks and a trio", () => {
    const importer = accounts.p1();
    copiedTrioCode().then(({ code, installed }) => {
      openWorkshop(importer);
      cy.get(DECK_ROWS).should("have.length", 0);
      pasteTrioCode(code);

      cy.get(ts(TRIO_IMPORT_PREVIEW)).should("have.attr", "data-ok", "true").and("contain.text", INSTALLED_TRIO_NAME);
      INSTALLED_DECK_NAMES.forEach((name, at) => {
        cy.get(ts(trioImportSlotId(at + 1)))
          .should("have.attr", "data-count", String(DECK_SIZE))
          .and("contain.text", name);
      });
      cy.get(ts(TRIO_IMPORT_SHARED)).should("not.exist");
      cy.get(ts(TRIO_IMPORT_SUBMIT)).should("not.be.disabled").click();

      cy.get(ts(TRIO_EDITOR), { timeout: timeouts.view }).should("be.visible");
      cy.get(ts(TRIO_NAME_INPUT)).should("have.value", INSTALLED_TRIO_NAME);
      cy.get(ts(TRIO_VERDICT)).should("have.attr", "data-ready", "true");
      cy.get(DECK_ROWS).should("have.length", constants.DECKS_PER_LOADOUT);

      cy.savedDecks(importer).should((saved) => {
        expect(saved.decks.map((deck) => deck.name), "the decks' names").to.deep.eq([...INSTALLED_DECK_NAMES]);
        expect(saved.decks.map((deck) => deck.cards), "the decks' cards, card for card").to.deep.eq(installed.decks);
        expect(saved.decks.map((deck) => deck.id), "new decks, not the exporter's").to.not.include.members(installed.deckIds);
        expect(saved.trios, "one trio").to.have.length(1);
        expect(saved.trios[0]?.name).to.eq(INSTALLED_TRIO_NAME);
        expect(saved.trios[0]?.deckIds, "the trio names the new decks in slot order").to.deep.eq(
          saved.decks.map((deck) => deck.id),
        );
      });
    });
  });

  it("R340 with too little room the import says how many slots it needs, the server refuses it too, and nothing is made", () => {
    const importer = accounts.p1();
    copiedTrioCode().then(({ code, installed }) => {
      // Leave one slot free; importing the trio needs three.
      cy.savedDecks(importer).then((saved) => {
        for (let at = 0; at < MAX_SAVED_DECKS - 1; at += 1) {
          cy.saveDeck(importer, { name: `Filler ${String(at + 1)}`, cards: [], catalogVersion: saved.catalogVersion });
        }
      });
      openWorkshop(importer);
      cy.get(DECK_ROWS).should("have.length", MAX_SAVED_DECKS - 1);
      pasteTrioCode(code);
      cy.get(ts(TRIO_IMPORT_PREVIEW)).should("have.attr", "data-ok", "true");
      cy.get(ts(TRIO_IMPORT_CAP_REASON))
        .should("have.attr", "data-decks-short", "2")
        .and("have.attr", "data-trios-short", "0")
        .and("contain.text", "needs 3 free deck slots")
        .and("contain.text", "Delete 2 decks");
      cy.get(ts(TRIO_IMPORT_SUBMIT)).should("be.disabled");

      // Server authority (CLAUDE.md rule 7): direct import has the same refusal and no writes.
      cy.savedDecks(importer).then((saved) => {
        cy.request<ErrorBody>({
          method: "POST",
          url: api("/api/trios/import"),
          headers: bearer(importer),
          body: {
            catalogVersion: saved.catalogVersion,
            trio: { id: mintId(), name: INSTALLED_TRIO_NAME },
            slots: installed.decks.map((cards, at) => ({ id: mintId(), name: INSTALLED_DECK_NAMES[at] ?? "Deck", cards })),
          },
          failOnStatusCode: false,
        }).should((response) => {
          expect(response.status, "R340: past the deck cap").to.eq(409);
          expect(response.body.error.code).to.eq("conflict");
          expect(response.body.error.details).to.deep.eq({
            decksShort: 2,
            triosShort: 0,
            limits: { decks: MAX_SAVED_DECKS, trios: MAX_SAVED_TRIOS },
          });
          expect(response.body.error.message).to.contain("Nothing was imported.");
        });
      });
      cy.savedDecks(importer).should((saved) => {
        expect(saved.decks, "no deck was made").to.have.length(MAX_SAVED_DECKS - 1);
        expect(saved.trios, "no trio was made").to.have.length(0);
      });
    });
  });
});
