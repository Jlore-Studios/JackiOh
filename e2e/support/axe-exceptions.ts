// The serious and critical axe violations spec 38 accepts (issue #757), each with its reason: a known
// shortfall written down, never a way to quiet the check. A flagged element no entry covers still
// fails, and spec 38 fails on an entry that no longer fires. The gaps below are the known design
// decisions (card-face naming, deck-tile naming, phone pile targets).

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
 * name says what pressing it does ("Remove Speciesist from Deck 2"), naming the card but not the cost.
 */
const DECK_TILE_LABEL =
  "A deck's card strip shows its cost and name; its name says what pressing it does and names the card. Reading the cost too is the same design decision.";

/**
 * WCAG 2.5.8 Target Size (Minimum), on a phone. A pile that opens is a 16 px pill in the thinner seat; a
 * finger gets the 44 px band its `::after` draws (B29, apps/web/src/game/inspectable.css), proven in a
 * browser by mobile-ux.cy.tsx. axe measures the element's box, not its pseudo-element, so it sees the pill.
 */
const PILE_BAND_SIZE =
  "A browsable pile is a 16 px pill on a phone; a finger gets its 44 px ::after band (B29), which axe does not measure. A taller pill takes height back from the board, a design decision.";

export const AXE_EXCEPTIONS: readonly AxeException[] = [
  { route: "/", rule: "label-content-name-mismatch", selector: ".landing-fan-card", reason: CARD_FACE_LABEL },
  { route: "/almanac", rule: "label-content-name-mismatch", selector: ".db-card", reason: CARD_FACE_LABEL },
  { route: "/decks", rule: "label-content-name-mismatch", selector: ".db-card", reason: CARD_FACE_LABEL },
  { route: "/decks", rule: "label-content-name-mismatch", selector: ".db-tile", reason: DECK_TILE_LABEL },
  { route: "/collection", rule: "label-content-name-mismatch", selector: ".collection-card", reason: CARD_FACE_LABEL },
  { route: "/practice board", rule: "label-content-name-mismatch", selector: ".prompt-card", reason: CARD_FACE_LABEL },
  // At 390x844 (#1561) the same card buttons. The open deck's strips are not among them: a phone
  // folds the deck's list away while its pool is open.
  { route: "/ (phone)", rule: "label-content-name-mismatch", selector: ".landing-fan-card", reason: CARD_FACE_LABEL },
  { route: "/almanac (phone)", rule: "label-content-name-mismatch", selector: ".db-card", reason: CARD_FACE_LABEL },
  { route: "/decks (phone)", rule: "label-content-name-mismatch", selector: ".db-card", reason: CARD_FACE_LABEL },
  { route: "/collection (phone)", rule: "label-content-name-mismatch", selector: ".collection-card", reason: CARD_FACE_LABEL },
  { route: "/practice board (phone)", rule: "label-content-name-mismatch", selector: ".prompt-card", reason: CARD_FACE_LABEL },
  { route: "/practice board (phone)", rule: "target-size", selector: ".pile--browsable", reason: PILE_BAND_SIZE },
  { route: "hotseat board (phone)", rule: "target-size", selector: ".pile--browsable", reason: PILE_BAND_SIZE },
];
