// Spec 35 — the settings dialog's tabs, its resets, the background-music switch, and the account's
// copy of the settings (issue #128, #129, R633, R634; #263).
//
// What it proves, against the `E2E=1` server and a `build:e2e` client:
//
//   * the dialog has four tabs, Gameplay, Visuals, Audio and Account, in that order; a click or the
//     arrow keys open one, only that tab's section is shown, and the dialog opens again on the tab
//     the player used last;
//   * Visuals' "Reduce motion" applies at once (`<html data-reduce-motion>`); "Reset this tab" puts
//     back only the open tab, "Reset all" every tab, and the Account tab has nothing to reset;
//   * the background-music switch is off by default, is kept on the device across a reload, and
//     Reset this tab turns it off again;
//   * signed out, the Account tab says the settings stay on this device and offers Sign in;
//   * signed in (`e2e-p1`, active), a change reaches the account (`PUT /api/settings`) and the tab
//     says so, and a fresh device (its `localStorage` empty) takes the account's copy on load
//     (`GET /api/settings`); a save that fails says so and is sent again by Try again.
//
// The E2E server keeps its store for its whole life and other specs sign in as `e2e-p1`, so the
// account is only ever asked to change the background-music switch, and the spec leaves it off
// (its default) on the account before and after, through the API, with the newest time.
//
// House rules (BUILD M8): no fixed waits — each account read and write the page makes is awaited as
// the intercepted request, and everything else is a retried assertion on the DOM; every selector
// comes from support/testids.ts and support/ux.ts. Nothing here starts a game, so there is no seed.

import { SESSION_STORAGE_KEY, accounts, routes, server } from "../../support/config.ts";
import { ts } from "../../support/testids.ts";
import {
  SETTINGS_CLOSE,
  SETTINGS_OPEN_NAV,
  SETTINGS_PANEL,
  SETTINGS_RESET,
  SETTINGS_RESET_TAB,
  SETTINGS_TABLIST,
  settingId,
  settingsSectionId,
  settingsTabId,
  type SettingsSection,
} from "../../support/ux.ts";

/** The tabs, in the order the dialog shows them (SECTIONS in apps/web/src/settings/SettingsPanel.tsx). */
const TABS: readonly SettingsSection[] = ["gameplay", "visuals", "audio", "account"];

/** The audio controls' background-music switch (MUSIC_FLAGS in apps/web/src/audio/AudioControls.tsx). */
const MUSIC_BACKGROUND = "audio-music-background";
/** The Account tab's status, its retry and its sign-in link (apps/web/src/settings/AccountSettings.tsx). */
const SETTINGS_ACCOUNT = "settings-account";
const SETTINGS_SYNC_RETRY = "settings-sync-retry";
const SETTINGS_SIGN_IN = "settings-sign-in";

/** Where the audio store keeps its settings (`AUDIO_SETTINGS_KEY` in apps/web/src/audio/constants.ts). */
const AUDIO_SETTINGS_KEY = "jackioh.audio.v1";

/** How long the page may take to read or write the account after it boots. */
const SYNC_TIMEOUT = 20_000;

type AccountCopy = { groups: Record<string, { at: number; values: Record<string, unknown> }> };

const account = accounts.p1;

function headers(): Record<string, string> {
  return { authorization: `Bearer ${account().token}` };
}

function readAccount(): Cypress.Chainable<AccountCopy> {
  return cy
    .request<{ settings: AccountCopy }>({ method: "GET", url: `${server.http()}/api/settings`, headers: headers() })
    .its("body.settings");
}

/**
 * The background-music switch off on the account, as the newest change, keeping the rest of its
 * audio group as it is. Nothing to do when the account holds no audio group (a fresh device then
 * keeps its own default, which is off).
 */
function backgroundOffOnTheAccount(): void {
  readAccount().then((copy) => {
    const audio = copy.groups.audio;
    if (audio === undefined) return;
    cy.request({
      method: "PUT",
      url: `${server.http()}/api/settings`,
      headers: headers(),
      body: { groups: { audio: { at: Date.now(), values: { ...audio.values, playMusicInBackground: false } } } },
    });
  });
}

/** The page's own requests to the account, spied on (never answered by the spec). */
function spyOnTheAccount(): void {
  cy.intercept("GET", "**/api/settings").as("load");
  cy.intercept("PUT", "**/api/settings").as("save");
}

function openSettings(): void {
  cy.get(ts(SETTINGS_OPEN_NAV)).click();
  cy.get(ts(SETTINGS_PANEL)).should("be.visible");
  cy.get(ts(SETTINGS_TABLIST)).should("be.visible");
}

function closeSettings(): void {
  cy.get(ts(SETTINGS_CLOSE)).click();
  cy.get(ts(SETTINGS_PANEL)).should("not.exist");
}

/** `tab` is selected and its section is the only one shown. */
function expectOpenTab(tab: SettingsSection): void {
  for (const other of TABS) {
    const open = other === tab;
    cy.get(ts(settingsTabId(other))).should("have.attr", "aria-selected", String(open));
    cy.get(ts(settingsSectionId(other))).should(open ? "be.visible" : "not.be.visible");
  }
}

function openTab(tab: SettingsSection): void {
  cy.get(ts(settingsTabId(tab))).click();
  expectOpenTab(tab);
}

function storedBackground(): Cypress.Chainable<unknown> {
  return cy.window().then((win) => {
    const raw = win.localStorage.getItem(AUDIO_SETTINGS_KEY);
    return raw === null ? undefined : (JSON.parse(raw) as { playMusicInBackground?: unknown }).playMusicInBackground;
  });
}

describe("35 — the settings dialog (#128, #129, R633, R634)", () => {
  describe("signed out, on the landing page", () => {
    beforeEach(() => {
      cy.visit("/");
    });

    it("has four tabs; a click or an arrow key opens one, alone, and it opens again on the last one used", () => {
      openSettings();
      cy.get(ts(SETTINGS_TABLIST))
        .find('[role="tab"]')
        .then(($tabs) => {
          expect(
            [...$tabs].map((tab) => tab.getAttribute("data-testid")),
            "the tabs, in order",
          ).to.deep.equal(TABS.map((tab) => settingsTabId(tab)));
        });
      // A device that never chose a tab opens on the first.
      expectOpenTab("gameplay");
      cy.get(ts(settingsSectionId("gameplay"))).find(ts(settingId("dragToPlay"))).should("exist");

      openTab("visuals");
      cy.get(ts(settingsSectionId("visuals"))).find(ts(settingId("reduceMotion"))).should("exist");
      openTab("audio");
      cy.get(ts(settingsSectionId("audio"))).find(ts(MUSIC_BACKGROUND)).should("exist");
      openTab("account");
      cy.get(ts(settingsSectionId("account"))).find(ts(SETTINGS_ACCOUNT)).should("exist");

      // The arrow keys walk the strip and wrap round its ends.
      cy.get(ts(settingsTabId("account"))).focus().type("{rightarrow}");
      expectOpenTab("gameplay");
      cy.focused().should("have.attr", "data-testid", settingsTabId("gameplay"));
      cy.focused().type("{leftarrow}{leftarrow}");
      expectOpenTab("audio");

      closeSettings();
      openSettings();
      expectOpenTab("audio");
    });

    it("applies Reduce motion at once; Reset this tab puts back only the open tab, and Reset all every tab", () => {
      openSettings();
      cy.get(ts(settingId("dragToPlay"))).should("be.checked").uncheck({ force: true });
      cy.get(ts(settingId("dragToPlay"))).should("not.be.checked");

      openTab("visuals");
      cy.get(ts(settingId("reduceMotion"))).should("not.be.checked").check({ force: true });
      cy.get("html").should("have.attr", "data-reduce-motion", "true");

      cy.get(ts(SETTINGS_RESET_TAB)).should("be.enabled").click();
      cy.get(ts(settingId("reduceMotion"))).should("not.be.checked");
      cy.get("html").should("not.have.attr", "data-reduce-motion");
      openTab("gameplay");
      cy.get(ts(settingId("dragToPlay"))).should("not.be.checked");

      // The Account tab keeps no store of its own, so there is nothing for it to reset.
      openTab("account");
      cy.get(ts(SETTINGS_RESET_TAB)).should("be.disabled");

      cy.get(ts(SETTINGS_RESET)).click();
      openTab("gameplay");
      cy.get(ts(settingId("dragToPlay"))).should("be.checked");
    });

    it("keeps the background-music switch, off by default, across a reload; Reset this tab turns it off", () => {
      openSettings();
      openTab("audio");
      cy.get(ts(MUSIC_BACKGROUND)).should("not.be.checked").check({ force: true });
      cy.get(ts(MUSIC_BACKGROUND)).should("be.checked");
      storedBackground().should("eq", true);

      cy.reload();
      openSettings();
      expectOpenTab("audio");
      cy.get(ts(MUSIC_BACKGROUND)).should("be.checked");

      cy.get(ts(SETTINGS_RESET_TAB)).click();
      cy.get(ts(MUSIC_BACKGROUND)).should("not.be.checked");
      storedBackground().should("eq", false);
    });

    it("says the settings stay on this device, and offers Sign in", () => {
      openSettings();
      openTab("account");
      cy.get(ts(SETTINGS_ACCOUNT)).should("have.attr", "data-phase", "signedOut").and("contain.text", "this device only");
      cy.get(ts(SETTINGS_SIGN_IN)).should("be.visible").click();
      cy.location("pathname").should("eq", routes.login());
    });
  });

  describe("signed in as an active account", () => {
    beforeEach(() => {
      backgroundOffOnTheAccount();
    });

    after(() => {
      backgroundOffOnTheAccount();
    });

    it("a change reaches the account, and a fresh device takes the account's copy", () => {
      spyOnTheAccount();
      cy.visitAs(account(), routes.deckbuilder());
      cy.wait("@load", { timeout: SYNC_TIMEOUT });

      openSettings();
      openTab("audio");
      cy.get(ts(MUSIC_BACKGROUND)).should("not.be.checked").check({ force: true });
      cy.wait("@save", { timeout: SYNC_TIMEOUT }).then((save) => {
        const sent = (save.request.body as { groups?: AccountCopy["groups"] }).groups;
        expect(sent?.audio?.values.playMusicInBackground, "the audio group goes up with the switch on").to.eq(true);
        expect(save.response?.statusCode, "PUT /api/settings answered").to.eq(200);
      });
      openTab("account");
      cy.get(ts(SETTINGS_ACCOUNT)).should("have.attr", "data-phase", "saved").and("contain.text", "Saved to your account");
      readAccount().its("groups.audio.values.playMusicInBackground").should("eq", true);
      closeSettings();

      // A fresh device: nothing in localStorage but the session the visit writes. Cleared as the new
      // page loads, not from here while the old one is still open: a page that sees its storage
      // emptied by another window (as `cy.clearLocalStorage` is) takes it for another tab putting
      // its settings back, stamps them as changed, and the next load keeps them over the account's.
      cy.visitAs(account(), routes.deckbuilder(), {
        onBeforeLoad(win) {
          for (const key of Object.keys(win.localStorage)) {
            if (key !== SESSION_STORAGE_KEY) win.localStorage.removeItem(key);
          }
        },
      });
      cy.wait("@load", { timeout: SYNC_TIMEOUT });
      openSettings();
      openTab("audio");
      cy.get(ts(MUSIC_BACKGROUND)).should("be.checked");
      storedBackground().should("eq", true);

      // Turned off here, the account follows.
      cy.get(ts(MUSIC_BACKGROUND)).uncheck({ force: true });
      cy.wait("@save", { timeout: SYNC_TIMEOUT })
        .its("request.body.groups.audio.values.playMusicInBackground")
        .should("eq", false);
      readAccount().its("groups.audio.values.playMusicInBackground").should("eq", false);
    });

    it("a save that fails says so on the Account tab, and Try again sends it", () => {
      spyOnTheAccount();
      // The newest intercept answers first: the page's first PUT never reaches the server.
      cy.intercept({ method: "PUT", url: "**/api/settings", times: 1 }, { forceNetworkError: true }).as("failed");
      cy.visitAs(account(), routes.deckbuilder());
      cy.wait("@load", { timeout: SYNC_TIMEOUT });

      openSettings();
      openTab("audio");
      cy.get(ts(MUSIC_BACKGROUND)).check({ force: true });
      cy.wait("@failed", { timeout: SYNC_TIMEOUT });
      openTab("account");
      cy.get(ts(SETTINGS_ACCOUNT)).should("have.attr", "data-phase", "error");
      cy.get(ts(SETTINGS_SYNC_RETRY)).should("be.visible").click();
      cy.wait("@save", { timeout: SYNC_TIMEOUT })
        .its("request.body.groups.audio.values.playMusicInBackground")
        .should("eq", true);
      cy.get(ts(SETTINGS_ACCOUNT)).should("have.attr", "data-phase", "saved");
      cy.get(ts(SETTINGS_SYNC_RETRY)).should("not.exist");
      readAccount().its("groups.audio.values.playMusicInBackground").should("eq", true);
    });
  });
});
