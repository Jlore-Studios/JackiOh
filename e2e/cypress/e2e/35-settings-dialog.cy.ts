// Settings-dialog coverage (R633, R634). The shared account changes only background music and ends off.
// BUILD M8: await account requests and use retried DOM assertions; selectors come from support/testids.ts and support/ux.ts.

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
const TABS: readonly SettingsSection[] = ["gameplay", "visuals", "audio"];

/** The audio controls' background-music switch (MUSIC_FLAGS in apps/web/src/audio/AudioControls.tsx). */
const MUSIC_BACKGROUND = "audio-music-background";

/** Where the audio store keeps its settings (`AUDIO_SETTINGS_KEY` in apps/web/src/audio/constants.ts). */
const AUDIO_SETTINGS_KEY = "jackioh.audio.v1";

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

/** Keeps background music off without overwriting existing audio settings. */
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

/** Spies on, rather than answers, the page's account requests. */
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

    it("has three tabs; a click or an arrow key opens one, alone, and it opens again on the last one used", () => {
      openSettings();
      cy.get(ts(SETTINGS_TABLIST))
        .find('[role="tab"]')
        .then(($tabs) => {
          expect(
            [...$tabs].map((tab) => tab.getAttribute("data-testid")),
            "the tabs, in order",
          ).to.deep.equal(TABS.map((tab) => settingsTabId(tab)));
        });
      expectOpenTab("gameplay");
      cy.get(ts(settingsSectionId("gameplay"))).find(ts(settingId("dragToPlay"))).should("exist");

      openTab("visuals");
      cy.get(ts(settingsSectionId("visuals"))).find(ts(settingId("reduceMotion"))).should("exist");
      openTab("audio");
      cy.get(ts(settingsSectionId("audio"))).find(ts(MUSIC_BACKGROUND)).should("exist");

      cy.get(ts(settingsTabId("audio"))).focus().type("{rightarrow}");
      expectOpenTab("gameplay");
      cy.focused().should("have.attr", "data-testid", settingsTabId("gameplay"));
      cy.focused().type("{leftarrow}");
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
      readAccount().its("groups.audio.values.playMusicInBackground").should("eq", true);
      closeSettings();

      // Clear storage on the new page: another window's clear writes settings that eclipse the account copy.
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

      cy.get(ts(MUSIC_BACKGROUND)).uncheck({ force: true });
      cy.wait("@save", { timeout: SYNC_TIMEOUT })
        .its("request.body.groups.audio.values.playMusicInBackground")
        .should("eq", false);
      readAccount().its("groups.audio.values.playMusicInBackground").should("eq", false);
    });
  });
});
