// The serious and critical axe violations spec 38 accepts (issue #757), each with its reason.
//
// An exception is a known shortfall written down, never a way to quiet the check: every entry names
// the route it holds for, the axe rule, the elements it covers (a CSS selector each flagged element
// must match) and why it is not fixed yet. A flagged element no entry covers still fails, and spec 38
// fails on an entry that no longer fires, so the list only ever holds what is still true.
// `/accessibility` lists the same gaps for players (apps/web/src/routes/accessibility.tsx).

export type AxeException = {
  /** The route label spec 38 checks, as its tests name it (`"/practice board"`, `"/almanac"`). */
  route: string;
  /** The axe rule id (`label-content-name-mismatch`, …). */
  rule: string;
  /** A CSS selector every element this entry covers matches. */
  selector: string;
  /** Why it stands: what it would take to fix and who decides. */
  reason: string;
};

/**
 * WCAG 2.5.3 Label in Name. A card shown as a button (the landing page's fan, the card pool's tiles,
 * a prompt's card options) draws the whole card face, its cost, name, type, rules text and stats, and
 * axe counts all of that as the button's visible label. Its accessible name is a summary that names
 * the card ("Read Bigot"; the pool's R432 "Bigot, Unit, (2) Cost, Common. Show details"; a prompt's
 * "Bigot, costs (2)"), so a voice user who says the name they see reaches it. Reading the whole face as the name, or moving the face out of the button,
 * is a design decision about how a card is announced, left to a person.
 */
const CARD_FACE_LABEL =
  "A card button's visible text is its whole face; its name is a summary that names the card. Changing how a card is announced is a design decision.";

/**
 * WCAG 2.5.3 Label in Name, the deck builder's list. A card in the open deck is a strip that shows its
 * cost and name, and pressing it takes the card out, so its name says that ("Remove Bigot from Deck
 * 2"): it names the card, but not the cost drawn beside it. Whether the cost is read too is the same
 * decision about how a card is announced, left to a person.
 */
const DECK_TILE_LABEL =
  "A deck's card strip shows its cost and name; its name says what pressing it does and names the card. Reading the cost too is the same design decision.";

/**
 * WCAG 2.5.8 Target Size (Minimum). The game log's card names are buttons that open the card, one per
 * line, and with a mouse the lines are a dense list less than 24 px apart (B29,
 * apps/web/src/game/inspectable.css); a coarse pointer, a finger, gets a 44 px row. Spacing the mouse's
 * rows out to 24 px shows fewer lines in the same box, which is a design decision left to a person.
 */
const LOG_LINE_SIZE =
  "The game log's card buttons sit in a dense list with a mouse (B29); a finger gets 44 px rows. Spacing a mouse's rows out is a design decision.";

export const AXE_EXCEPTIONS: readonly AxeException[] = [
  { route: "/", rule: "label-content-name-mismatch", selector: ".landing-fan-card", reason: CARD_FACE_LABEL },
  { route: "/almanac", rule: "label-content-name-mismatch", selector: ".db-card", reason: CARD_FACE_LABEL },
  { route: "/decks", rule: "label-content-name-mismatch", selector: ".db-tile", reason: DECK_TILE_LABEL },
  { route: "/practice board", rule: "label-content-name-mismatch", selector: ".prompt-card", reason: CARD_FACE_LABEL },
  { route: "hotseat board, a unit in play", rule: "target-size", selector: ".log .log-card", reason: LOG_LINE_SIZE },
];
