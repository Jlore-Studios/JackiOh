// 45 — the economy loop: a new account starts, earns, spends and collects (#1310; SPEC §14, §14.12;
// R1310000, R1310003, R1310004).
//
// Key assertion (SPEC §14.12): "Practice and hotseat let a player use every card and every cosmetic
// whatever the ledger holds, and so do the end-to-end tests, but the one that walks the economy
// itself." This is that one (R1310004). It walks the fixture account `e2e-econ` through the loop,
// each step asserted on what the server answers and what the page shows:
//
// 1. R1310000: the good code, redeemed, grants exactly `STARTER_CARDS`, one copy each, and pays the
//    newest set's track through the level it starts at.
// 2. §14.8, R1302001: a Best-of-1 ladder game on a deck of starter cards, won when seat two
//    concedes, raises the wallet by at least `JLORE_LADDER_WIN` (a level the XP reaches may pay
//    more) and the newest track by at least `XP_WIN_BONUS`, and `/collection` shows that track's
//    level and XP as `/api/tracks` answers them.
// 3. §14.1, §14.3: `/account`'s shop sells a global pack for `PACK_PRICE_JLORE`, and the pack opens.
// 4. §14.3: each slot the server granted as a new card reads "New card", and the collection grows
//    by exactly those cards.
// 5. R1310003, §14.5: the Dev tools fill the card token bar, which pays one card token, and the
//    token picker's Pick unlocks the card it names.
// 6. R1304000, §14.9: no quest has arrived on the account's first visits; the Dev tools give one,
//    and `/collection` lists it.
// 7. R1313000, R1310004: with the full-art gate asked for (`FULL_ART_GATED_KEY` before the page
//    loads), a full-art card the account owns prints its normal face in the deck builder; two
//    cosmetic bar fills pay a Rare token and then an Epic one (R1310003), the Epic token buys that
//    card's full art (R1305003), and on the next load the face prints full art while another
//    full-art card the account owns, whose full art it does not, still prints its normal face.
//
// One test, after a reseed (R144) in `beforeEach`: each step spends what the one before it earned,
// so a retry walks the loop again from the start instead of from wherever the first attempt stopped.
// The reseed also puts back `e2e-p1`, `e2e-p2`, `e2e-pending` and the good code; spec 10 and spec 44,
// which read them, reseed first too.
//
// Networked: it needs `E2E=1 cargo run --release -p jackioh-server` beside a development-mode client
// (`vite dev` or `build:e2e`, the only builds that read `FULL_ART_GATED_KEY`). Daily only, as every
// spec outside scripts/shard-specs.mjs's SMOKE.

import { newestShippedSet } from "../../../apps/web/src/wire/catalog.ts";
import { COSMETICS } from "../../../apps/web/src/wire/generated/cosmetics.ts";
import {
  COSMETIC_TOKEN_CYCLE,
  GLOBAL_PACK_NAME,
  JLORE_LADDER_WIN,
  MATCHMAKER_SWEEP_INTERVAL_SECONDS,
  PACK_PRICE_JLORE,
  PACK_SLOTS,
  RATING_WINDOW_FULL_AFTER_SECONDS,
  SERIES_POLL_SECONDS,
  STARTER_CARDS,
  XP_WIN_BONUS,
} from "../../../apps/web/src/wire/serverConfig.ts";
import {
  E2E_RESEED_PATH,
  FULL_ART_GATED_KEY,
  accounts,
  constants,
  inviteCodes,
  routes,
  seedFor,
  server,
  timeouts,
  type E2EAccount,
} from "../../support/config.ts";
import {
  ACCOUNT_DEV_TOGGLE,
  ACCOUNT_PACKS_TOGGLE,
  CARD_FACE,
  COLLECTION_JLORE,
  COLLECTION_SCREEN,
  DEV_TOOLS,
  DEV_TOOLS_FILL_CARD_BAR,
  DEV_TOOLS_FILL_COSMETIC_BAR,
  DEV_TOOLS_GIVE_QUEST,
  DEV_TOOLS_STATUS,
  PACK_CLOSE,
  PACK_SCENE,
  PACK_SKIP_ALL,
  PACK_SLOTS as PACK_SLOTS_ID,
  PLAY_DECK_SELECT,
  PLAY_QUEUE,
  PLAY_STATUS,
  QUESTS_QUEST,
  SHOP_WALLET,
  TOKEN_CARD_PICKS,
  TOKEN_HELD_CARD,
  collectionTrackId,
  collectionTrackLevelId,
  collectionTrackXpId,
  deckRowId,
  heroId,
  packSlotId,
  packSlotOutcomeId,
  packsOpenId,
  playModeId,
  poolCardId,
  shopBuyId,
  tokenCardPickId,
  ts,
} from "../../support/testids.ts";

// Constants

/** Seconds to milliseconds. */
const MS_PER_SECOND = 1000;

/** Pairing waits out the rating window's widening to MAX_FAIR_GAP (R640001), a sweep, a poll and a render, as in spec 37. */
const PAIRING_TIMEOUT_MS =
  (RATING_WINDOW_FULL_AFTER_SECONDS + MATCHMAKER_SWEEP_INTERVAL_SECONDS + SERIES_POLL_SECONDS) *
    MS_PER_SECOND +
  timeouts.view;

/** The Node second-player client, as in specs 06 and 37. */
const SEAT_TWO = "seat-two";

/** A match page, capturing its id. */
const MATCH_PATH = /^\/match\/([^/]+)$/;

/** installLoadout's default deck index, the deck seat two queues. */
const FIXTURE_DECK = 0;

/** What the Dev tools' status line says once the server has answered a tool with 200 (DevTools.tsx). */
const DEV_TOOL_DONE = "Done.";

/** A slot's outcome for a new card (packs.ts `slotOutcome`). */
const NEW_CARD_WORDS = "New card";

/** R1310003: what two cosmetic bar fills from the cycle's start pay, in order. */
const TWO_FILLS_PAY = ["rare", "epic"] as const;

/** The cosmetic token tier a cosmetic token redemption names (`POST /api/tokens/redeem-cosmetic`'s `token`). */
const EPIC_TOKEN = "Epic";

const SEED = seedFor("45-economy-loop");

// HTTP contract, as far as this spec reads it

type MeBody = { profile: { id: string; status: string }; needsInviteCode: boolean; isDev?: boolean };

type RedeemBody = { status?: string; needsInviteCode?: boolean };

type CollectionBody = { catalogVersion: string; entries: { cardId: string; quantity: number }[] };

type WalletBody = { jlore: number };

type TrackBody = {
  setId: string;
  level: number;
  xp: number;
  levelXp: number | null;
  xpToNext: number | null;
  rewardsClaimed: number;
  unlocked: boolean;
};

type TracksBody = { tracks: TrackBody[] };

type CosmeticStep = "rare" | "epic" | "cosmeticPack";

type TokensBody = {
  nextStep: CosmeticStep;
  tokens: { card: number; cosmeticRare: number; cosmeticEpic: number; cosmeticPack: number };
};

type OwnedCosmeticsBody = { cards: { cardId: string; cosmeticId: string }[]; account: string[] };

type QuestsBody = { quests: { questId: string }[]; nextDueAt: number | null };

type BoughtBody = { pack: string; jlore: number; unopened: number };

type OpenedBody = {
  pack: string;
  slots: { rarity: string; cardId: string; grant: { kind: "card" | "cosmetic" | "bars" } }[];
  unopened: number;
};

type FullArt = Extract<(typeof COSMETICS)[number], { kind: "fullArt" }>;

/** What the loop carries from one step to the next. */
type Loop = {
  deckId: string;
  jlore: number;
  owned: Set<string>;
  opened: OpenedBody | null;
};

// Helpers

function econ(): E2EAccount {
  return accounts.econ();
}

function api(path: string): string {
  return `${server.http()}${path}`;
}

function bearer(account: E2EAccount): Record<string, string> {
  return { authorization: `Bearer ${account.token}` };
}

/** `GET path` as `account`, which must answer 200; yields the body. */
function read<T extends object>(account: E2EAccount, path: string): Cypress.Chainable<T> {
  return cy
    .request<T>({ method: "GET", url: api(path), headers: bearer(account) })
    .then((response) => {
      expect(response.status, `GET ${path}`).to.eq(200);
    })
    .its("body");
}

function ownedIds(collection: CollectionBody): Set<string> {
  return new Set(collection.entries.filter((entry) => entry.quantity > 0).map((entry) => entry.cardId));
}

/** The newest shipped set's track, the one active from the start (§14.7). */
function newestTrack(body: TracksBody): TrackBody {
  const track = body.tracks.find((entry) => entry.setId === newestShippedSet());
  expect(track, `GET /api/tracks lists the newest set's (${newestShippedSet()}) track`).to.not.eq(undefined);
  return track as TrackBody;
}

/** A number as the page formats it (i18n.ts `formatNumber`, the browser's own locale in English). */
function shown(value: number): string {
  return new Intl.NumberFormat().format(value);
}

/** The cosmetic tokens of one cycle step that `tokens` holds. */
function heldOf(tokens: TokensBody, step: CosmeticStep): number {
  switch (step) {
    case "rare":
      return tokens.tokens.cosmeticRare;
    case "epic":
      return tokens.tokens.cosmeticEpic;
    case "cosmeticPack":
      return tokens.tokens.cosmeticPack;
  }
}

function landedOn(pattern: RegExp, timeout: number): Cypress.Chainable<string> {
  return cy
    .location("pathname", { timeout })
    .should("match", pattern)
    .then((pathname) => pattern.exec(pathname)?.[1] ?? "");
}

/** Seat two concedes over its own socket (§2.5 stamps the socket's seat; `playerId` is only action-shape data). */
function seatTwoConcedes(): void {
  cy.wsPlayer({ action: "send", name: SEAT_TWO, body: { type: "concede", playerId: "p2" } });
}

/** R1310004: the full-art gate, asked for before the page's scripts run (fullArt.ts reads it at load). */
function gateFullArt(win: Cypress.AUTWindow): void {
  win.localStorage.setItem(FULL_ART_GATED_KEY, "1");
}

/** `/account` with its Dev tools open (R1300000: shown because `/api/auth/me` says `isDev`). */
function openDevTools(): void {
  cy.visitAs(econ(), routes.account());
  cy.get(ts(ACCOUNT_DEV_TOGGLE), { timeout: timeouts.view }).click();
  cy.get(ts(DEV_TOOLS), { timeout: timeouts.view }).should("be.visible");
}

/** Press a Dev tools button whose request `alias` spies on, and see the server answer it and the panel say so. */
function pressDevTool(testid: string, alias: string): Cypress.Chainable<unknown> {
  cy.get(ts(testid)).should("not.be.disabled").click();
  return cy.wait(`@${alias}`).then((interception) => {
    expect(interception.response?.statusCode, `${testid}: the server's answer`).to.eq(200);
    cy.get(ts(DEV_TOOLS_STATUS), { timeout: timeouts.view }).should("have.text", DEV_TOOL_DONE);
    // A callback that ran a command yields through a chain, never a bare value (Cypress refuses the mix).
    return cy.wrap(interception.response?.body as unknown, { log: false });
  });
}

/** The deck builder with the loop's deck open, its full-art ownership read (R1313000). */
function openDeckGated(deckId: string): void {
  cy.intercept("GET", "**/api/cosmetics").as("ownedCosmetics");
  cy.visitAs(econ(), routes.deckbuilder(), { onBeforeLoad: gateFullArt });
  // The read happens only while ownership decides (ownedFullArt.ts), so its answer is the gate at work.
  cy.wait("@ownedCosmetics").its("response.statusCode").should("eq", 200);
  cy.get(ts(deckRowId(deckId)), { timeout: timeouts.view }).click();
}

// The steps

/** 1. R1310000: a new account's start. */
function redeemTheGoodCode(loop: Loop): void {
  read<MeBody>(econ(), "/api/auth/me").then((me) => {
    expect(me.profile.status, "R144: the reseed leaves e2e-econ waiting at the invite gate").to.eq("pending");
    expect(me.needsInviteCode).to.eq(true);
    expect(me.isDev, "R1310004: the E2E server counts e2e-econ as a dev account").to.eq(true);
  });
  cy.request<RedeemBody>({
    method: "POST",
    url: api("/api/codes/redeem"),
    headers: bearer(econ()),
    body: { code: inviteCodes.good() },
  }).then((response) => {
    expect(response.status, "§9.4: the good code activates the account").to.eq(200);
    expect(response.body.status).to.eq("active");
  });
  read<CollectionBody>(econ(), "/api/collection").then((collection) => {
    expect(
      collection.entries.map((entry) => entry.cardId).sort(),
      `R1310000: a new account owns the ${String(STARTER_CARDS.length)} starter cards and nothing else`,
    ).to.deep.eq([...STARTER_CARDS].sort());
    for (const entry of collection.entries) {
      expect(entry.quantity, `R1310000: one copy of ${entry.cardId}`).to.eq(constants.MAX_COPIES);
    }
    loop.owned = ownedIds(collection);
  });
  read<TracksBody>(econ(), "/api/tracks").then((body) => {
    const track = newestTrack(body);
    expect(track.unlocked, "§14.7: the newest set's track is active from the start").to.eq(true);
    expect(
      track.rewardsClaimed,
      "R1310000: the redemption paid the newest track through the level it starts at",
    ).to.eq(track.level);
  });
}

/** 2. §14.8: a ladder game won pays Jlore and XP, and `/collection` shows the track. */
function winALadderGame(loop: Loop): void {
  let jloreBefore = 0;
  let xpBefore = 0;
  let theirDeck = "";
  read<WalletBody>(econ(), "/api/wallet").then((wallet) => {
    jloreBefore = wallet.jlore;
  });
  read<TracksBody>(econ(), "/api/tracks").then((body) => {
    xpBefore = newestTrack(body).xp;
  });
  cy.saveDeck(econ(), { name: "Starter deck", cards: STARTER_CARDS.slice(0, constants.DECK_SIZE) }).then((id) => {
    loop.deckId = id;
  });
  cy.installLoadout(accounts.p2(), "19-modes-b").then((installed) => {
    theirDeck = installed.deckIds[FIXTURE_DECK];
  });

  cy.visitAs(econ(), routes.play());
  cy.get(ts(playModeId("bo1")), { timeout: timeouts.view }).check();
  cy.get(ts(playModeId("bo1"))).should("be.checked");
  cy.then(() => {
    cy.get(ts(PLAY_DECK_SELECT), { timeout: timeouts.view }).select(loop.deckId);
    cy.get(ts(PLAY_DECK_SELECT)).should("have.value", loop.deckId);
  });
  // R253: the queue checks L5, so this is the starter cards being enough to play the ladder.
  cy.get(ts(PLAY_QUEUE), { timeout: timeouts.view }).should("not.be.disabled").click();
  cy.get(ts(PLAY_STATUS), { timeout: timeouts.view }).should("be.visible");
  cy.then(() => {
    cy.request({
      method: "POST",
      url: api("/api/queue"),
      headers: bearer(accounts.p2()),
      body: { mode: "bo1", deckId: theirDeck, seed: SEED },
    }).should((response) => {
      expect(response.status, "R143: the queue takes an end-to-end seed").to.eq(200);
    });
    // R897000: seat two is a queued client, so it polls like one while the rating window widens.
    cy.stayQueued(accounts.p2(), SERIES_POLL_SECONDS * MS_PER_SECOND);
  });
  let matchId = "";
  landedOn(MATCH_PATH, PAIRING_TIMEOUT_MS).then((id) => {
    matchId = id;
  });
  cy.then(() => {
    cy.wsPlayer({ action: "connect", name: SEAT_TWO, url: server.ws(), token: accounts.p2().token, matchId });
  });
  cy.get(ts(heroId("you")), { timeout: timeouts.view }).should("be.visible");
  cy.then(() => {
    seatTwoConcedes();
  });
  cy.expectResult("Win");

  // The result's transaction pays (api/results.rs), so the next screen reads what it paid.
  cy.visitAs(econ(), routes.collection());
  cy.get(ts(COLLECTION_SCREEN), { timeout: timeouts.view }).should("be.visible");
  cy.then(() => {
    read<WalletBody>(econ(), "/api/wallet").then((wallet) => {
      expect(
        wallet.jlore - jloreBefore,
        `§14.8, R1302001: a ladder win pays at least JLORE_LADDER_WIN (${String(JLORE_LADDER_WIN)})`,
      ).to.be.at.least(JLORE_LADDER_WIN);
      loop.jlore = wallet.jlore;
      cy.get(ts(COLLECTION_JLORE), { timeout: timeouts.view }).should("have.text", `${shown(wallet.jlore)} Jlore`);
    });
    read<TracksBody>(econ(), "/api/tracks").then((body) => {
      const track = newestTrack(body);
      expect(
        track.xp - xpBefore,
        `§14.8, R1302001: a ladder win gives at least XP_WIN_BONUS (${String(XP_WIN_BONUS)}) to the newest track`,
      ).to.be.at.least(XP_WIN_BONUS);
      expect(track.levelXp, "the track is short of its last level").to.not.eq(null);
      const span = track.levelXp ?? 0;
      const left = track.xpToNext ?? 0;
      cy.get(ts(collectionTrackLevelId(track.setId)), { timeout: timeouts.view }).should("contain.text", shown(track.level));
      cy.get(ts(collectionTrackId(track.setId))).find("progress").should("have.attr", "max", String(span)).and("have.attr", "value", String(span - left));
      cy.get(ts(collectionTrackXpId(track.setId))).should("contain.text", shown(left));
    });
  });
}

/** 3. §14.1, §14.3: the shop sells a global pack, and it opens. */
function buyAndOpenAGlobalPack(loop: Loop): void {
  cy.then(() => {
    expect(loop.jlore, `§14.3: what the account earned covers a pack (PACK_PRICE_JLORE ${String(PACK_PRICE_JLORE)})`).to.be.at.least(
      PACK_PRICE_JLORE,
    );
  });
  read<CollectionBody>(econ(), "/api/collection").then((collection) => {
    loop.owned = ownedIds(collection);
  });
  cy.intercept("POST", "**/api/packs/buy").as("buy");
  cy.intercept("POST", "**/api/packs/open").as("open");
  cy.visitAs(econ(), routes.account());
  cy.get(ts(ACCOUNT_PACKS_TOGGLE), { timeout: timeouts.view }).click();
  cy.then(() => {
    cy.get(ts(SHOP_WALLET), { timeout: timeouts.view }).should("have.text", `${String(loop.jlore)} Jlore`);
  });
  cy.get(ts(shopBuyId(GLOBAL_PACK_NAME))).should("not.be.disabled").click();
  cy.wait("@buy").then((interception) => {
    expect(interception.response?.statusCode, "POST /api/packs/buy").to.eq(200);
    const bought = interception.response?.body as BoughtBody;
    expect(bought.pack).to.eq(GLOBAL_PACK_NAME);
    expect(bought.jlore, "§14.3: the pack costs PACK_PRICE_JLORE").to.eq(loop.jlore - PACK_PRICE_JLORE);
    loop.jlore = bought.jlore;
  });
  cy.then(() => {
    read<WalletBody>(econ(), "/api/wallet").its("jlore").should("eq", loop.jlore);
  });

  cy.get(ts(packsOpenId(GLOBAL_PACK_NAME)), { timeout: timeouts.view }).click();
  cy.get(ts(PACK_SCENE), { timeout: timeouts.view }).should("be.visible");
  // Reveal all tears the sealed pack too: the one request is the opening.
  cy.get(ts(PACK_SKIP_ALL)).should("not.be.disabled").click();
  cy.wait("@open").then((interception) => {
    expect(interception.response?.statusCode, "POST /api/packs/open").to.eq(200);
    const opened = interception.response?.body as OpenedBody;
    expect(opened.pack).to.eq(GLOBAL_PACK_NAME);
    expect(opened.slots, "§14.3: a card pack holds PACK_SLOTS slots").to.have.length(PACK_SLOTS);
    loop.opened = opened;
  });
  cy.get(ts(PACK_SLOTS_ID), { timeout: timeouts.view }).should("be.visible");
}

/** 4. §14.3: a new card's slot says so, and the collection holds it. */
function collectTheNewCards(loop: Loop): void {
  cy.then(() => {
    const slots = loop.opened?.slots ?? [];
    const fresh = slots.flatMap((slot, index) => (slot.grant.kind === "card" ? [{ index, cardId: slot.cardId }] : []));
    expect(
      fresh.length,
      "§14.3, PACK_UNOWNED_WEIGHTS: a pack opened on the starter cards unlocks a new card",
    ).to.be.greaterThan(0);
    for (const { index, cardId } of fresh) {
      expect(loop.owned.has(cardId), `${cardId} was not owned before the pack`).to.eq(false);
      cy.get(ts(packSlotId(index))).should("have.attr", "data-grant", "card").and("have.attr", "data-revealed", "true");
      cy.get(ts(packSlotOutcomeId(index))).should("have.text", NEW_CARD_WORDS);
    }
    const expected = new Set([...loop.owned, ...fresh.map((slot) => slot.cardId)]);
    read<CollectionBody>(econ(), "/api/collection").then((collection) => {
      const owned = ownedIds(collection);
      expect([...owned].sort(), "§14.3: the collection grew by exactly the pack's new cards").to.deep.eq([...expected].sort());
      loop.owned = owned;
    });
  });
  cy.get(ts(PACK_CLOSE)).click();
  cy.get(ts(PACK_SCENE)).should("not.exist");
}

/** 5. R1310003, §14.5: a full card token bar pays a token, which unlocks the card picked. */
function redeemACardToken(loop: Loop): void {
  let held = 0;
  read<TokensBody>(econ(), "/api/tokens").then((tokens) => {
    held = tokens.tokens.card;
  });
  cy.intercept("POST", "**/api/dev/bar/card").as("fillCardBar");
  openDevTools();
  pressDevTool(DEV_TOOLS_FILL_CARD_BAR, "fillCardBar");
  cy.then(() => {
    read<TokensBody>(econ(), "/api/tokens").then((tokens) => {
      expect(tokens.tokens.card, "R1310003: the full card token bar paid one card token").to.eq(held + 1);
      held = tokens.tokens.card;
    });
  });

  // The token picker reads its tokens when it mounts, and the Dev tools tell no other panel, so read it again.
  cy.reload();
  cy.get(ts(ACCOUNT_PACKS_TOGGLE), { timeout: timeouts.view }).click();
  cy.then(() => {
    cy.get(ts(TOKEN_HELD_CARD), { timeout: timeouts.view }).should("have.text", `Card tokens held: ${String(held)}`);
  });
  cy.intercept("POST", "**/api/tokens/redeem-card").as("redeemCard");
  let picked = "";
  cy.get(TOKEN_CARD_PICKS)
    .first()
    .should("not.be.disabled")
    .invoke("attr", "data-testid")
    .then((testid) => {
      picked = String(testid).slice(tokenCardPickId("").length);
      expect(loop.owned.has(picked), `§14.5: the picker offers ${picked}, a card not owned`).to.eq(false);
    });
  cy.then(() => {
    cy.get(ts(tokenCardPickId(picked))).click();
  });
  cy.wait("@redeemCard").then((interception) => {
    expect(interception.response?.statusCode, "POST /api/tokens/redeem-card").to.eq(200);
    expect((interception.request.body as { cardId?: string }).cardId, "the Pick sends its own card").to.eq(picked);
    expect((interception.response?.body as TokensBody).tokens.card, "§14.5: the redemption spent the token").to.eq(held - 1);
  });
  cy.then(() => {
    read<CollectionBody>(econ(), "/api/collection").then((collection) => {
      const owned = ownedIds(collection);
      expect(owned.has(picked), `§14.5: the card token unlocked ${picked}`).to.eq(true);
      expect(owned.size, "and nothing else").to.eq(loop.owned.size + 1);
      loop.owned = owned;
    });
    cy.get(ts(tokenCardPickId(picked)), { timeout: timeouts.view }).should("not.exist");
  });
}

/** 6. R1304000, §14.9: a quest given at once shows on `/collection`. */
function receiveAQuest(): void {
  read<QuestsBody>(econ(), "/api/quests").then((body) => {
    expect(body.quests, "R1304000: an account's first visits start the clock and bring no quest").to.deep.eq([]);
  });
  cy.intercept("POST", "**/api/dev/quests").as("giveQuest");
  openDevTools();
  let given = "";
  pressDevTool(DEV_TOOLS_GIVE_QUEST, "giveQuest").then((body) => {
    given = (body as { quest: { questId: string } }).quest.questId;
  });
  cy.then(() => {
    read<QuestsBody>(econ(), "/api/quests").then((body) => {
      expect(body.quests.map((quest) => quest.questId), "§14.10: the dev tool gave the account that quest").to.deep.eq([given]);
    });
    cy.visitAs(econ(), routes.collection());
    cy.get(ts(QUESTS_QUEST), { timeout: timeouts.view }).should("have.length", [given].length).and("have.attr", "data-quest-id", given);
  });
}

/** 7. R1313000: a full art prints on its owner's own copy once owned, and on no other. */
function earnAFullArt(loop: Loop): void {
  let earned: FullArt | null = null;
  let other: FullArt | null = null;
  read<OwnedCosmeticsBody>(econ(), "/api/cosmetics").then((owned) => {
    const fullArts = COSMETICS.filter((cosmetic): cosmetic is FullArt => cosmetic.kind === "fullArt").filter(
      (cosmetic) =>
        loop.owned.has(cosmetic.card) &&
        !owned.cards.some((entry) => entry.cardId === cosmetic.card && entry.cosmeticId === cosmetic.id),
    );
    [earned = null, other = null] = fullArts;
    expect(earned, "a full art to earn, on a card the account owns").to.not.eq(null);
    expect(other, "and another, to compare").to.not.eq(null);
  });
  const fullArt = (): FullArt => earned as unknown as FullArt;
  const unearned = (): FullArt => other as unknown as FullArt;

  // Gated, an owned card whose full art the account lacks prints its normal face.
  cy.then(() => {
    openDeckGated(loop.deckId);
    cy.get(ts(poolCardId(fullArt().card)), { timeout: timeouts.view }).find(CARD_FACE).should("exist").and("not.have.attr", "data-full-art");
  });

  // R1310003: two fills of the cosmetic token bar, from the cycle's start, pay a Rare token then an Epic one.
  const paid: CosmeticStep[] = [];
  cy.intercept("POST", "**/api/dev/bar/cosmetic").as("fillCosmeticBar");
  openDevTools();
  for (const fill of TWO_FILLS_PAY) {
    read<TokensBody>(econ(), "/api/tokens").then((before) => {
      pressDevTool(DEV_TOOLS_FILL_COSMETIC_BAR, "fillCosmeticBar");
      read<TokensBody>(econ(), "/api/tokens").then((after) => {
        expect(heldOf(after, before.nextStep), `the fill paid the cycle's next step, ${before.nextStep} (for the ${fill} fill)`).to.eq(
          heldOf(before, before.nextStep) + 1,
        );
        paid.push(before.nextStep);
      });
    });
  }
  cy.then(() => {
    expect(paid, "R1310003: from the cycle's start, a Rare token then an Epic one").to.deep.eq([...TWO_FILLS_PAY]);
    expect(COSMETIC_TOKEN_CYCLE.slice(0, TWO_FILLS_PAY.length), "COSMETIC_TOKEN_CYCLE starts as R1310003 says").to.deep.eq([
      ...TWO_FILLS_PAY,
    ]);
  });

  // R1305003, R1313000: the Epic token buys the full art, for the one card it names.
  read<TokensBody>(econ(), "/api/tokens").then((before) => {
    cy.request<TokensBody>({
      method: "POST",
      url: api("/api/tokens/redeem-cosmetic"),
      headers: bearer(econ()),
      body: { token: EPIC_TOKEN, cosmeticId: fullArt().id, cardId: fullArt().card },
    }).then((response) => {
      expect(response.status, `POST /api/tokens/redeem-cosmetic for ${fullArt().id}`).to.eq(200);
      expect(response.body.tokens.cosmeticEpic, "§14.5: it spent the Epic token").to.eq(before.tokens.cosmeticEpic - 1);
    });
  });
  read<OwnedCosmeticsBody>(econ(), "/api/cosmetics").then((owned) => {
    expect(owned.cards, "R1313000: the account owns the full art, on its card").to.deep.include({
      cardId: fullArt().card,
      cosmeticId: fullArt().id,
    });
  });

  // The next load reads the ownership again: the owned full art prints, the other still does not.
  cy.then(() => {
    openDeckGated(loop.deckId);
    cy.get(ts(poolCardId(fullArt().card)), { timeout: timeouts.view }).find(CARD_FACE).should("have.attr", "data-full-art", "true");
    cy.get(ts(poolCardId(unearned().card))).find(CARD_FACE).should("exist").and("not.have.attr", "data-full-art");
  });
}

// ---------------------------------------------------------------------------------------------

describe("R1310000 R1310003 the economy loop", () => {
  beforeEach(() => {
    expect(SEED, "BUILD M8: every spec sets a seed").to.be.a("string").and.not.eq("");
    // R144: every fixture as a boot leaves it, so e2e-econ is pending and the good code unspent.
    cy.request({ method: "POST", url: api(E2E_RESEED_PATH) }).its("status").should("eq", 200);
  });

  it("a new account starts with the starter cards, wins on the ladder, opens a pack, redeems its tokens, is given a quest and earns a full art", () => {
    const loop: Loop = { deckId: "", jlore: 0, owned: new Set(), opened: null };
    redeemTheGoodCode(loop);
    winALadderGame(loop);
    buyAndOpenAGlobalPack(loop);
    collectTheNewCards(loop);
    redeemACardToken(loop);
    receiveAQuest();
    earnAFullArt(loop);
  });
});
