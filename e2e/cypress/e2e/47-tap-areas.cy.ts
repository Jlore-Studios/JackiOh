// 47 — every control on a phone takes a tap across 44x44 px (issue #2122, part of #2036, R2122000).
// Each test opens one screen at 390x844 and runs `expectHitAreas`; two check the measure itself
// (index.css's `--hit-min` is `HIT_MIN_PX`, and a probe narrower than the minimum fails).
// Per pull request: SMOKE holds 47. Networked as 46.

import { consentTestid, inviteTestid, landingTestid, loginTestid, resetTestid, shellTestid } from "../../../apps/web/src/auth/testids.ts";
import { patchTestid } from "../../../apps/web/src/patches/testids.ts";
import { practiceTestid } from "../../../apps/web/src/practice/testids.ts";
import { statsTestid } from "../../../apps/web/src/stats/testids.ts";
import { SERIES_MAX_GAMES, SERIES_PICK_SECONDS, SERIES_WINS_NEEDED } from "../../../apps/web/src/wire/serverConfig.ts";
import { allCardIds, cardId } from "../../support/cards.ts";
import { accounts, constants, routes, seedFor, timeouts, viewports } from "../../support/config.ts";
import {
  BOARD,
  CARD_POOL,
  COLLECTION_JLORE,
  COLLECTION_SCREEN,
  CREATOR_SCREEN,
  PLAY_QUEUE,
  PLAY_SIGN_IN,
  PRACTICE_HUD,
  PROMPT_SUBMIT,
  REPLAYS_ROOT,
  SERIES_SCORE,
  SHOP_PANEL,
  TOGETHER_ROOT,
  WORKSHOP,
  deckRowId,
  promptOf,
  ts,
} from "../../support/testids.ts";
import { SETTINGS_OPEN_NAV, SETTINGS_PANEL } from "../../support/ux.ts";

/** R2122000: the least a control's hit area spans in either dimension; index.css's `--hit-min`. */
const HIT_MIN_PX = 44;
/** A box may fall this far short (a sub-pixel border, a rounded width) and still be at its mark. */
const SLACK_PX = 0.5;
/** A failure names a control with no testid by this much of its text, and sizes in this many decimals. */
const NAME_CHARS = 30;
const PX_DIGITS = 1;
/** The interactive elements #2122 names. */
const CONTROLS = 'button, a[href], [role="button"], input, select, [tabindex="0"]';
/** R2122000's exemptions, never measured but still neighbours: a link in running text, and a card. */
const EXEMPT = ":is(p, label) a, .card";
/** The practice worker loads the engine, the card scripts and the AI before its first answer. */
const BOOT_TIMEOUT = 60_000;
/** The card pool of a big list takes a while to draw its first cards on a cold browser. */
const CARD_POOL_TIMEOUT = 30_000;
/** One pass over a screen of several hundred controls, retried until the layout has settled. */
const MEASURE_TIMEOUT = 10_000;
const MS_PER_SECOND = 1000;
const SEED = seedFor("47-tap-areas");
/** The series the intercept answers, as spec 46's: a v4 UUID's shape, nothing the server holds. */
const SERIES_ID = "00000000-0000-4000-8000-000000002122";
/** p2, not 46's p1: the smoke shard runs 46 then 47 on one server, and both specs' menus in one minute pass R109's allowance for one account. */
const SIGNED_IN = accounts.p2;
/** The screens whose testids live in their route's component file, which this project does not type-check. */
const SCREEN = { accountRank: "account-rank", leaderboardSeason: "leaderboard-season", almanac: "almanac" } as const;
/** The measure's own probe: this much narrower than the minimum and taller, its ::before no height and this far past each side. */
const PROBE_PX = 14;
const PROBE_TESTID = "tap-probe";

type Box = { left: number; top: number; right: number; bottom: number };
type Control = { el: Element; name: string; own: Box; shown: Box; hit: Box; exempt: boolean };

const boxOf = (el: Element): Box => {
  const { left, top, right, bottom } = el.getBoundingClientRect();
  return { left, top, right, bottom };
};
const union = (a: Box, b: Box): Box => ({ left: Math.min(a.left, b.left), top: Math.min(a.top, b.top), right: Math.max(a.right, b.right), bottom: Math.max(a.bottom, b.bottom) });
/** How far two boxes overlap across and down (negative: the gap between them). */
const overlap = (a: Box, b: Box) => ({ x: Math.min(a.right, b.right) - Math.max(a.left, b.left), y: Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) });

/** A checkbox's or a radio's labels, which toggle it. The tag, not `instanceof`: the page is another window. */
function labelsOf(el: Element): Element[] {
  const input = el as HTMLInputElement;
  return el.tagName === "INPUT" && (input.type === "checkbox" || input.type === "radio") && input.labels !== null ? Array.from(input.labels) : [];
}

/** `el`'s box, its labels' and each ::before and ::after that takes taps where its overflow leaves room; one with no width or no height takes none. */
function hitBox(win: Window, el: Element): Box {
  const style = win.getComputedStyle(el);
  let hit = boxOf(el);
  const origin = { x: hit.left + parseFloat(style.borderLeftWidth), y: hit.top + parseFloat(style.borderTopWidth) };
  for (const which of ["::before", "::after"]) {
    const p = win.getComputedStyle(el, which);
    if (p.content === "none" || p.content === "normal" || p.position !== "absolute" || style.position === "static" || p.pointerEvents === "none") continue;
    // A centering translate (inspectable.css's pile band) shifts the box; the rest of a transform is ignored.
    const shift = new DOMMatrixReadOnly(p.transform === "none" ? undefined : p.transform);
    const [x, y, w, h] = [parseFloat(p.left) + shift.e, parseFloat(p.top) + shift.f, parseFloat(p.width), parseFloat(p.height)];
    if (![x, y, w, h].every(Number.isFinite) || w <= SLACK_PX || h <= SLACK_PX || style.clipPath !== "none") continue;
    if (style.overflowX === "visible") hit = { ...hit, left: Math.min(hit.left, origin.x + x), right: Math.max(hit.right, origin.x + x + w) };
    if (style.overflowY === "visible") hit = { ...hit, top: Math.min(hit.top, origin.y + y), bottom: Math.max(hit.bottom, origin.y + y + h) };
  }
  return labelsOf(el).reduce((sum, label) => union(sum, hitBox(win, label)), hit);
}

/** The part of `el` the window and its clipping ancestors show; null when none is (scrolled out of its region). */
function shownBox(win: Window, el: Element): Box | null {
  let shown = { ...boxOf(el) };
  shown = { left: Math.max(shown.left, 0), top: Math.max(shown.top, 0), right: Math.min(shown.right, win.innerWidth), bottom: Math.min(shown.bottom, win.innerHeight) };
  for (let up = el.parentElement; up !== null; up = up.parentElement) {
    const style = win.getComputedStyle(up);
    const clip = boxOf(up);
    if (style.overflowX !== "visible") shown = { ...shown, left: Math.max(shown.left, clip.left), right: Math.min(shown.right, clip.right) };
    if (style.overflowY !== "visible") shown = { ...shown, top: Math.max(shown.top, clip.top), bottom: Math.min(shown.bottom, clip.bottom) };
  }
  return shown.right - shown.left > SLACK_PX && shown.bottom - shown.top > SLACK_PX ? shown : null;
}

function nameOf(el: Element): string {
  const own = el.getAttribute("data-testid");
  if (own !== null) return own;
  const what = `<${el.tagName.toLowerCase()}> "${(el.getAttribute("aria-label") ?? el.textContent ?? "").trim().slice(0, NAME_CHARS)}"`;
  const holder = el.parentElement?.closest("[data-testid]")?.getAttribute("data-testid");
  return holder === undefined || holder === null ? what : `${what} in ${holder}`;
}

/** The controls a tap reaches: shown, taking pointer events, inside the topmost open modal if any. */
function controlsOf(win: Window): Control[] {
  const modal = Array.from(win.document.querySelectorAll('[aria-modal="true"]')).filter((m) => m.getClientRects().length > 0).at(-1);
  const controls: Control[] = [];
  for (const el of win.document.querySelectorAll(CONTROLS)) {
    const style = win.getComputedStyle(el);
    const shown = shownBox(win, el);
    if ((modal !== undefined && !modal.contains(el)) || style.visibility === "hidden" || style.pointerEvents === "none" || shown === null) continue;
    const own = labelsOf(el).reduce((sum, label) => union(sum, boxOf(label)), boxOf(el));
    controls.push({ el, name: nameOf(el), own, shown, hit: hitBox(win, el), exempt: el.matches(EXEMPT) });
  }
  return controls;
}

/**
 * R2122000: a hit area reaches `HIT_MIN_PX / 2` past its center each way, or halfway to a nearer
 * neighbour (another control, neither holding the other, not overlapping it), and covers no neighbour.
 */
function hitAreaProblems(win: Window): string[] {
  const controls = controlsOf(win);
  const problems: string[] = [];
  for (const c of controls) {
    const others = controls.filter((n) => {
      const o = overlap(c.own, n.own);
      return n !== c && !n.el.contains(c.el) && !c.el.contains(n.el) && (o.x <= SLACK_PX || o.y <= SLACK_PX);
    });
    const half = HIT_MIN_PX / 2;
    const [cx, cy] = [(c.own.left + c.own.right) / 2, (c.own.top + c.own.bottom) / 2];
    const reach = { left: cx - half, top: cy - half, right: cx + half, bottom: cy + half };
    for (const n of others) {
      const o = overlap(c.own, n.own);
      if (o.y > 0 && n.own.right <= c.own.left + SLACK_PX) reach.left = Math.max(reach.left, (n.own.right + c.own.left) / 2);
      if (o.y > 0 && n.own.left >= c.own.right - SLACK_PX) reach.right = Math.min(reach.right, (n.own.left + c.own.right) / 2);
      if (o.x > 0 && n.own.bottom <= c.own.top + SLACK_PX) reach.top = Math.max(reach.top, (n.own.bottom + c.own.top) / 2);
      if (o.x > 0 && n.own.top >= c.own.bottom - SLACK_PX) reach.bottom = Math.min(reach.bottom, (n.own.top + c.own.bottom) / 2);
      const covered = overlap(c.hit, n.shown);
      if (covered.x > SLACK_PX && covered.y > SLACK_PX) problems.push(`${c.name}: its hit area covers ${n.name}`);
    }
    const size = `${(c.hit.right - c.hit.left).toFixed(PX_DIGITS)}x${(c.hit.bottom - c.hit.top).toFixed(PX_DIGITS)} px`;
    const short = (axis: string) => problems.push(`${c.name}: its hit area is ${size}, short of ${String(HIT_MIN_PX)} px ${axis}`);
    if (!c.exempt && (c.hit.left > reach.left + SLACK_PX || c.hit.right < reach.right - SLACK_PX)) short("across");
    if (!c.exempt && (c.hit.top > reach.top + SLACK_PX || c.hit.bottom < reach.bottom - SLACK_PX)) short("down");
  }
  return problems;
}

/** R2122000 at the phone size, retried until the screen has settled. */
function expectHitAreas(): void {
  cy.window({ timeout: MEASURE_TIMEOUT }).should((win) => {
    expect([win.innerWidth, win.innerHeight], "the viewport asked for").to.deep.eq([viewports.phone.width, viewports.phone.height]);
    expect(hitAreaProblems(win), "controls whose hit area falls short or covers another").to.deep.eq([]);
  });
}

/**
 * A pending account's code and consent screens for `SIGNED_IN`. Spec 10 redeems the pending fixture until
 * the server restarts (R144) and may share this shard, so its own read says pending, plus `owes`: the
 * screens draw only from that read (CLAUDE.md rule 7).
 */
function asPending(owes: Record<string, boolean> = {}): void {
  cy.intercept("GET", "**/api/auth/me", (req) => {
    req.continue((res) => {
      const body = res.body as Record<string, unknown> & { profile: Record<string, unknown> };
      res.body = {
        ...body,
        profile: { ...body.profile, status: "pending" },
        needsInviteCode: true,
        currentMatchId: null,
        currentSeriesId: null,
        ...owes,
      };
    });
  });
}

/** A series picking among three decks, the clock running: its tallest screen (`SeriesView`, apps/web/src/net/api.ts); `now` stands for the server's clock (R333). */
function seriesPicking(now: number): Record<string, unknown> {
  return {
    id: SERIES_ID,
    status: "picking",
    gameNo: 1,
    winsNeeded: SERIES_WINS_NEEDED,
    maxGames: SERIES_MAX_GAMES,
    pickDeadline: now + SERIES_PICK_SECONDS * MS_PER_SECOND,
    now,
    currentMatchId: null,
    ranked: true,
    you: {
      seat: "p1",
      wins: 0,
      trioName: "Main trio",
      decks: ["Aggro", "Control", "Ramp"].map((name, slot) => ({
        slot,
        name,
        cards: [cardId(slot + 1)],
        won: false,
        games: 0,
      })),
      pick: null,
      autoPick: false,
    },
    opponent: {
      wins: 0,
      decks: [0, 1, 2].map((slot) => ({ slot, won: false })),
      picked: false,
    },
    games: [],
    result: null,
  };
}

describe(`47 — every control on a phone takes a tap across ${String(HIT_MIN_PX)}x${String(HIT_MIN_PX)} px (issue #2122)`, () => {
  beforeEach(() => {
    cy.viewport(viewports.phone.width, viewports.phone.height);
  });

  describe("signed out", () => {
    beforeEach(() => {
      cy.signOut();
    });

    it("R2122000 index.css's --hit-min is the spec's minimum", () => {
      cy.visit("/no-such-menu");
      cy.get(ts(shellTestid.notFound), { timeout: timeouts.view }).should("be.visible");
      cy.document().should((doc) => expect(getComputedStyle(doc.documentElement).getPropertyValue("--hit-min").trim(), "index.css's --hit-min").to.eq(`${String(HIT_MIN_PX)}px`));
    });

    it("R2122000 a band with no height widens nothing: a control narrower than the minimum fails, however tall", () => {
      cy.visit("/no-such-menu");
      cy.get(ts(shellTestid.notFound), { timeout: timeouts.view }).should("be.visible");
      cy.document().then((doc) => {
        const band = doc.createElement("style");
        band.textContent = `[data-testid="${PROBE_TESTID}"]::before { content: ""; position: absolute; top: 50%; bottom: 50%; left: -${String(PROBE_PX)}px; right: -${String(PROBE_PX)}px; }`;
        const probe = doc.createElement("button");
        probe.dataset.testid = PROBE_TESTID;
        Object.assign(probe.style, { position: "fixed", top: "50%", left: "50%", padding: "0", width: `${String(HIT_MIN_PX - PROBE_PX)}px`, height: `${String(HIT_MIN_PX + PROBE_PX)}px` });
        doc.head.append(band);
        doc.body.append(probe);
      });
      const size = `${(HIT_MIN_PX - PROBE_PX).toFixed(PX_DIGITS)}x${(HIT_MIN_PX + PROBE_PX).toFixed(PX_DIGITS)} px`;
      cy.window().should((win) => expect(hitAreaProblems(win), "what the measure says of the probe").to.include(`${PROBE_TESTID}: its hit area is ${size}, short of ${String(HIT_MIN_PX)} px across`));
    });

    it("R2122000 the board, a practice game: the mulligan, then the first turn", () => {
      cy.visit(`${routes.practice()}?${new URLSearchParams({ seed: SEED, difficulty: "easy", deck: "random", seat: "p1", pace: "fast" }).toString()}`);
      cy.get(ts(PRACTICE_HUD), { timeout: BOOT_TIMEOUT }).should("be.visible");
      cy.get(ts(BOARD), { timeout: BOOT_TIMEOUT }).should("be.visible");
      cy.get(promptOf("mulligan"), { timeout: BOOT_TIMEOUT }).should("be.visible");
      cy.settled();
      expectHitAreas();
      cy.get(ts(PROMPT_SUBMIT)).click();
      cy.get(promptOf("mulligan"), { timeout: BOOT_TIMEOUT }).should("not.exist");
      cy.settled();
      expectHitAreas();
    });

    it("R2122000 /", () => {
      cy.visit("/");
      cy.get(ts(landingTestid.root), { timeout: timeouts.view }).should("be.visible");
      // The fan's cards are dealt in (R704): measured once every one is drawn.
      cy.get(`${ts(landingTestid.fan)} .landing-fan-card--face`, { timeout: timeouts.view }).should(($cards) => {
        expect($cards.length, "the fan holds its cards").to.be.greaterThan(0);
        for (const card of $cards.toArray()) expect(getComputedStyle(card).opacity, "a dealt card").to.eq("1");
      });
      expectHitAreas();
    });

    it("R2122000 /, the settings open", () => {
      cy.visit("/");
      cy.get(ts(landingTestid.root), { timeout: timeouts.view }).should("be.visible");
      cy.get(ts(SETTINGS_OPEN_NAV)).click();
      cy.get(ts(SETTINGS_PANEL), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /login", () => {
      cy.visit(routes.login());
      cy.get(ts(loginTestid.form), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /login, creating an account", () => {
      cy.visit(routes.login());
      cy.get(ts(loginTestid.mode), { timeout: timeouts.view }).click();
      cy.get(ts(loginTestid.form)).should("have.attr", "data-mode", "signUp");
      cy.get(ts(consentTestid.birthYear)).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /reset-password", () => {
      cy.visit("/reset-password");
      cy.get(ts(resetTestid.screen), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /almanac", () => {
      cy.visit("/almanac");
      cy.get(ts(SCREEN.almanac), { timeout: timeouts.view }).should("be.visible");
      cy.get(ts(CARD_POOL), { timeout: CARD_POOL_TIMEOUT }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /patch-notes", () => {
      cy.visit("/patch-notes");
      cy.get(ts(patchTestid.screen), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /stats", () => {
      cy.visit("/stats");
      cy.get(ts(statsTestid.summaryTiles), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /practice", () => {
      cy.visit(routes.practice());
      cy.get(ts(practiceTestid.setup), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /play, signed out", () => {
      cy.visit(routes.play());
      cy.get(ts(PLAY_SIGN_IN), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /replays, signed out", () => {
      cy.visit(routes.replays());
      cy.get(ts(REPLAYS_ROOT), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 a path nothing is at", () => {
      cy.visit("/no-such-menu");
      cy.get(ts(shellTestid.notFound), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });
  });

  describe("signed in", () => {
    before(() => {
      cy.freeAccount(SIGNED_IN());
    });

    it("R2122000 /play", () => {
      cy.visitAs(SIGNED_IN(), routes.play());
      cy.get(ts(PLAY_QUEUE), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /decks", () => {
      cy.visitAs(SIGNED_IN(), routes.deckbuilder());
      cy.get(ts(WORKSHOP), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /decks, a deck open", () => {
      // One full deck, whatever earlier specs saved: the open deck's pool is this menu's big list.
      cy.clearDecks(SIGNED_IN());
      cy.saveDeck(SIGNED_IN(), { name: "Taps", cards: allCardIds().slice(0, constants.DECK_SIZE) }).then((deckId) => {
        cy.visitAs(SIGNED_IN(), routes.deckbuilder());
        cy.get(ts(WORKSHOP), { timeout: timeouts.view }).should("be.visible");
        // A phone shows the list alone until a deck is opened (a desktop opens it already).
        cy.get(ts(deckRowId(deckId))).click();
        cy.get(ts(CARD_POOL), { timeout: CARD_POOL_TIMEOUT }).should("be.visible");
        expectHitAreas();
      });
    });

    it("R2122000 /account", () => {
      cy.visitAs(SIGNED_IN(), "/account");
      cy.get(ts(SCREEN.accountRank), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /leaderboard", () => {
      cy.visitAs(SIGNED_IN(), "/leaderboard");
      cy.get(ts(SCREEN.leaderboardSeason), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /creator", () => {
      cy.visitAs(SIGNED_IN(), routes.creator());
      cy.get(ts(CREATOR_SCREEN), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /replays", () => {
      cy.visitAs(SIGNED_IN(), routes.replays());
      cy.get(ts(REPLAYS_ROOT), { timeout: timeouts.view }).find('[role="status"]').should("not.exist");
      expectHitAreas();
    });

    it("R2122000 /together", () => {
      cy.visitAs(SIGNED_IN(), "/together");
      cy.get(ts(TOGETHER_ROOT), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /collection", () => {
      cy.visitAs(SIGNED_IN(), routes.collection());
      cy.get(ts(COLLECTION_SCREEN), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /collection, Cards", () => {
      cy.visitAs(SIGNED_IN(), routes.collectionCards());
      cy.get(ts(COLLECTION_JLORE), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /collection, Cosmetics", () => {
      cy.visitAs(SIGNED_IN(), `${routes.collection()}?section=cosmetics`);
      cy.get(ts(COLLECTION_SCREEN), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /shop", () => {
      cy.visitAs(SIGNED_IN(), routes.shop());
      cy.get(ts(SHOP_PANEL), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /series, choosing a deck", () => {
      cy.intercept("GET", `**/api/series/${SERIES_ID}`, { statusCode: 200, body: seriesPicking(Date.now()) });
      cy.visitAs(SIGNED_IN(), routes.series(SERIES_ID));
      cy.get(ts(SERIES_SCORE), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /invite, the code screen", () => {
      asPending();
      cy.visitAs(SIGNED_IN(), routes.invite());
      cy.get(ts(inviteTestid.input), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });

    it("R2122000 /invite, the consent screen", () => {
      asPending({ ageCheckOwed: true, termsOwed: true });
      cy.visitAs(SIGNED_IN(), routes.invite());
      cy.get(ts(consentTestid.birthMonth), { timeout: timeouts.view }).should("be.visible");
      expectHitAreas();
    });
  });
});
