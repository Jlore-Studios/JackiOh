// Types the specs and support commands share: narrow structural subsets of the real types (SPEC §10.1,
// §10.2, §10.8), not @jackioh/shared, since `tsc -p e2e/tsconfig.json` has to pass on its own.

/** SPEC §10.1: seats. */
export type PlayerId = "p1" | "p2";

/** BUILD M5-T1 testid: `zone-<side>-<row>-<lane>`. `side` is view-relative (ASSUMPTION A3). */
export type Side = "you" | "opponent";

/** SPEC §3: the two field rows. */
export type Row = "units" | "backrow";

/** SPEC §3.1: lanes are 1-indexed, 1..5, exactly as `crates/engine/src/zones.rs` numbers them. */
export type Lane = 1 | 2 | 3 | 4 | 5;

export type ZoneRef = { side: Side; row: Row; lane: Lane };

/** SPEC §10.6 / crates/engine/src/wire/catalog_types.rs. */
export type PromptKind =
  | "discover"
  | "target"
  | "mode"
  | "mulligan"
  | "hand"
  | "zone"
  | "tribute"
  | "direction"
  | "x"
  | "embiggen"
  | "number"
  | "answer"
  | "cell"
  | "reward"
  | "pick"
  | "craft"
  | "market";

/** BUILD M5-T4: every animation sets `data-animating="<eventType>"` while it runs. */
export type EventType = string;

/** crates/engine/src/wire/actions.rs `Selection`. */
export type Selection =
  | { pick: "instance"; instanceId: string }
  | { pick: "hero"; player: PlayerId }
  | { pick: "zone"; player: PlayerId; row: Row; lane: number }
  | { pick: "mode"; option: string }
  | { pick: "none" };

/** crates/engine/src/wire/actions.rs `ActionBody`, client-sendable members only. */
export type ActionBody =
  | { type: "mulligan"; keep: string[] }
  | {
      type: "play";
      instanceId: string;
      zone?: { row: Row; lane: number };
      x?: number;
      embiggen?: boolean;
      tributes?: string[];
      targets?: Selection[];
      modes?: string[];
    }
  | { type: "attack"; attackerId: string; targetId: string }
  | { type: "switchPosition"; instanceId: string }
  | { type: "activatePower"; instanceId: string; targets?: Selection[] }
  | { type: "answer"; choiceId: string; selection: Selection[] }
  | { type: "offerDraw" }
  | { type: "answerDraw"; accept: boolean }
  | { type: "concede" }
  | { type: "endTurn" }
  // R1838000: a Trade of a Tradable hand card.
  | { type: "trade"; instanceId: string }
  // R345: a seat's own automatic turn end preference.
  | { type: "setAutoEndTurn"; enabled: boolean };

export type ActionInput = ActionBody & { playerId: PlayerId };
export type Action = ActionBody & { playerId: PlayerId; nonce: string };

/** The parts of `GameState` (SPEC §10.1) the specs read off `window.__jackioh.state`: loose, used only for the replay hash and seat/turn bookkeeping. */
export type GameStateLike = {
  seed: string;
  turn: number;
  active: PlayerId;
  phase: string;
  result: { winner: PlayerId | "draw"; reason: string } | null;
  /**
   * TWO SHAPES, ONE FIELD: the hotseat's raw `PendingChoice` names its seat `playerId` (`crates/engine/src/state.rs`);
   * a networked handle's (`apps/web/src/game/net.ts` `viewDerivedState`) spells it `player`. Declaring
   * only one let an assertion on the other read `undefined` past type-checking.
   */
  pending:
    | { id?: string; choiceId?: string; kind?: PromptKind; playerId?: PlayerId; player?: PlayerId }
    | null;
  /**
   * §2.1 step 3, R265: both seats' open mulligans, hotseat only (`crates/engine/src/state.rs`
   * `MulliganSeat`); `pending` is null meanwhile, and `keep` is a seat's sealed answer, null until given.
   */
  mulligan?: Record<PlayerId, { prompt?: { id?: string; kind?: PromptKind }; keep: string[] | null }>;
  players: Record<PlayerId, unknown>;
  [key: string]: unknown;
};

/**
 * BUILD M5-T3: `/dev/hotseat` exposes this outside production. `state`, `dispatch` and `seed` are the
 * fixed contract; `log` and `decks` are ASSUMPTION A2, for spec 01's replay hash.
 */
export type JackiOhDevHandle = {
  state: GameStateLike;
  dispatch: (action: ActionInput) => void;
  seed: string;
  /** ASSUMPTION A2: every action the hotseat loop has reduced, in order, with its nonce. */
  log?: Action[];
  /** ASSUMPTION A2: the two deck lists the route resolved, seat order [p1, p2]. */
  decks?: [string[], string[]];
  /** R180: the handicaps the hotseat game was created with, `{}` for none; `cy.replayCheck` folds with them. */
  handicaps?: Partial<Record<PlayerId, FixtureHandicap>>;
  /** Present in networked mode (apps/web/src/game/net.ts) when the route is not hotseat. */
  seat?: PlayerId;
  /** Hotseat only (routes/dev/hotseat.tsx), for spec 36's driver: the device seat's `viewFor`, its `legalActions`, and handing the device over. */
  view?: () => Record<string, unknown>;
  legal?: () => ActionBody[];
  setSeat?: (seat: PlayerId) => void;
};

/**
 * R180: one seat's handicap, a structural copy of `Handicap` in crates/engine/src/config.rs. Fields are
 * non-negative integers, `deckSize` is 1..LIBRARY_CAP (R184), and `heroHealth` is optional (R290).
 */
export type FixtureHandicap = {
  deckSize: number;
  manaBonus: number;
  manaCap: number;
  extraOpeningCards: number;
  extraDrawsPerTurn: number;
  heroHealth?: number;
};

/** A scenario deck under e2e/fixtures/decks. Shape is ASSUMPTION A1 (and A9). */
export type FixtureDeck = {
  /** The id passed as `a=` / `b=` on the hotseat route. Equals the filename without `.json`. */
  id: string;
  /** Which M8 spec owns this fixture. */
  spec: string;
  /** Why these cards: what the spec needs out of the deck. */
  description: string;
  /** Exactly DECK_SIZE distinct non-Token catalog ids, or `handicap.deckSize` with a handicap (R184). */
  cards: string[];
  /**
   * The seat's handicap (R180), for a scenario SPEC's own resources cannot reach in a few turns. A hotseat
   * fixture tuned before #1705 carries `TUTORIAL_HUMAN`'s, a human's resources with the earlier deck
   * size, to keep its cards (R1705005); a networked one carries none: the server refuses a handicap.
   */
  handicap?: FixtureHandicap;
};

/** What `seedGame` injects for the client to resolve `a=` / `b=` (ASSUMPTION A1). */
export type E2EDeckInjection = {
  decks: Record<string, string[]>;
  seed: string;
  /** R180: each seat's handicap, from the fixtures that carry one; absent when neither does. */
  handicaps?: Partial<Record<PlayerId, FixtureHandicap>>;
  /** R1420: the sets the hotseat engine previews before it creates the game, so a deck may hold an unshipped set (spec 36); a production build refuses `previewSets`. */
  preview?: string[];
};

/** How a prompt is answered. `answerPrompt` resolves these against the modal, then the board. */
export type PromptAnswer = {
  /** `PendingOption.key` values to pick (discover, mode, direction, embiggen, hand, mulligan). */
  options?: string[];
  /** The first N options, whatever their keys: the match rng draws Discover's (SPEC §10.6). Picked before `options`; the two combine. */
  first?: number;
  /** R81's embiggen price, as the boolean it is: the picker keys its two prices `"true"` and `"false"`. */
  embiggen?: boolean;
  /** Board instance ids to pick (target, tribute, hand pickers rendered on the cards). */
  cards?: string[];
  /** Board zones to pick (zone pickers). */
  zones?: ZoneRef[];
  /** A hero target (`target` prompts that may hit a hero). */
  hero?: Side;
  /** The value for an `x` prompt: its card when the picker shows a few (#492), else typed and confirmed. */
  x?: number;
  /** Press submit even for a single-pick kind (multi-select kinds always submit). */
  submit?: boolean;
};

declare global {
  interface Window {
    /** BUILD M5-T3. */
    __jackioh?: JackiOhDevHandle;
    /** ASSUMPTION A1: fixture decks handed to the E2E build of the hotseat route. */
    __jackiohE2E?: E2EDeckInjection;
  }
}
