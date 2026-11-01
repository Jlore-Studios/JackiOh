// The serious and critical axe violations spec 38 accepts (issue #757), each with its reason: a known
// shortfall written down, never a way to quiet the check. A flagged element no entry covers still
// fails, and spec 38 fails on an entry that no longer fires. `/accessibility` lists the same gaps for
// players (apps/web/src/routes/accessibility.tsx).

export type AxeException = {
  /** The route label spec 38 checks, as its tests name it (`"/almanac"`; at 390x844 `"/almanac (phone)"`). */
  route: string;
  /** The axe rule id. */
  rule: string;
  /** A CSS selector every element this entry covers matches. */
  selector: string;
  /** Why it stands: what it would take to fix and who decides. */
  reason: string;
};

/**
 * WCAG 2.5.3 Label in Name. A card shown as a button draws its whole face, which axe counts as its
 * visible label; its accessible name is a summary that names the card (R432), so a voice user who
 * says the name they see reaches it. How a card is announced is a design decision left to a person.
 */
const CARD_FACE_LABEL =
  "A card button's visible text is its whole face; its name is a summary that names the card. Changing how a card is announced is a design decision.";

/**
 * WCAG 2.5.3 Label in Name, the deck builder's list: an open deck's strip shows cost and name, and its
 * name says what pressing it does ("Remove Bigot from Deck 2"), naming the card but not the cost.
 */
const DECK_TILE_LABEL =
  "A deck's card strip shows its cost and name; its name says what pressing it does and names the card. Reading the cost too is the same design decision.";

/**
 * WCAG 2.5.8 Target Size (Minimum). With a mouse the game log's card buttons are a dense list less than
 * 24 px apart (B29, apps/web/src/game/inspectable.css); a finger gets a 44 px row. Spacing the mouse's
 * rows out shows fewer lines in the same box, a design decision left to a person.
 */
const LOG_LINE_SIZE =
  "The game log's card buttons sit in a dense list with a mouse (B29); a finger gets 44 px rows. Spacing a mouse's rows out is a design decision.";

export const AXE_EXCEPTIONS: readonly AxeException[] = [
  { route: "/", rule: "label-content-name-mismatch", selector: ".landing-fan-card", reason: CARD_FACE_LABEL },
  { route: "/almanac", rule: "label-content-name-mismatch", selector: ".db-card", reason: CARD_FACE_LABEL },
  { route: "/decks", rule: "label-content-name-mismatch", selector: ".db-card", reason: CARD_FACE_LABEL },
  { route: "/decks", rule: "label-content-name-mismatch", selector: ".db-tile", reason: DECK_TILE_LABEL },
  { route: "/practice board", rule: "label-content-name-mismatch", selector: ".prompt-card", reason: CARD_FACE_LABEL },
  { route: "hotseat board, a unit in play", rule: "target-size", selector: ".log .log-card", reason: LOG_LINE_SIZE },
  // At 390x844 (#1561) the same card buttons. The open deck's strips are not among them: a phone
  // folds the deck's list away while its pool is open.
  { route: "/ (phone)", rule: "label-content-name-mismatch", selector: ".landing-fan-card", reason: CARD_FACE_LABEL },
  { route: "/almanac (phone)", rule: "label-content-name-mismatch", selector: ".db-card", reason: CARD_FACE_LABEL },
  { route: "/decks (phone)", rule: "label-content-name-mismatch", selector: ".db-card", reason: CARD_FACE_LABEL },
  { route: "/practice board (phone)", rule: "label-content-name-mismatch", selector: ".prompt-card", reason: CARD_FACE_LABEL },
];
