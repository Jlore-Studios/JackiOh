// Effects that ask the controller something: Choose one, a target, a card in hand, and Discover
// (SPEC §6.3, §10.6). Each opens a prompt and hands the answer to a named resume step.

import type { CardType, PlayerId, Row, Selection, Tag, ZoneRef } from "@jackioh/shared";
import { PLAYER_IDS, opponentOf } from "@jackioh/shared";
import { defOf, excludingDefId, query, type CatalogQueryArgs } from "../catalog";
import { cardTypeOf } from "../faces";
import { ANSWER_KEY, answerKeyOf, openPrompt, resumeSelf } from "../prompts";
import type { Effect, EffectContext } from "../script";
import { effectiveCost } from "../mana";
import type { CardInstance, GameState, PromptOption } from "../state";
import { activeUnitsOf, cardAt, isUnitToken, rowSize, slotsOf } from "../zones";
import { playerOf, type PlayerSpec } from "./targets";

/** Which cards a `target` prompt may offer. */
export type TargetScope = {
  side?: "ally" | "enemy" | "any";
  of?: ("unit" | "hero" | "backrow")[];
  type?: CardType | CardType[];
  excludeSelf?: boolean;
};

function sidesOf(ctx: EffectContext, side: TargetScope["side"]): PlayerId[] {
  if (side === "ally") return [ctx.controller];
  if (side === "enemy") return [opponentOf(ctx.controller)];
  return [...PLAYER_IDS];
}

/** Every card and hero a scope allows, in a deterministic order (active side first, lane order). */
export function targetsInScope(ctx: EffectContext, scope: TargetScope = {}): Selection[] {
  const kinds = scope.of ?? ["unit"];
  const out: Selection[] = [];

  for (const player of sidesOf(ctx, scope.side)) {
    if (kinds.includes("unit")) {
      for (const unit of activeUnitsOf(ctx.state, player)) {
        if (scope.excludeSelf === true && unit.id === ctx.self?.id) continue;
        out.push({ pick: "instance", instanceId: unit.id });
      }
    }
    if (kinds.includes("backrow")) {
      for (const ref of slotsOf(player, "backrow")) {
        const card = cardAt(ctx.state, ref);
        if (card === null) continue;
        if (scope.excludeSelf === true && card.id === ctx.self?.id) continue;
        out.push({ pick: "instance", instanceId: card.id });
      }
    }
    if (kinds.includes("hero")) out.push({ pick: "hero", player });
  }

  return out;
}

/**
 * §10.6: an answered prompt's selection arrives in `ctx.targets`, so a Discover's pick — a `mode`
 * option carrying a def id — is read from there. Play-time modes (R81) arrive in `ctx.modes`, and a
 * resume step may be reached either way, so both are offered in order.
 */
export function chosenOptions(ctx: EffectContext): string[] {
  const picked = ctx.targets.flatMap((selection) =>
    selection.pick === "mode" ? [selection.option] : [],
  );
  return [...picked, ...ctx.modes];
}

function label(ctx: EffectContext, selection: Selection): string {
  if (selection.pick === "hero") return `${selection.player}'s hero`;
  if (selection.pick === "instance") {
    const card = findOnBoard(ctx, selection.instanceId);
    return card === null ? selection.instanceId : defOf(ctx.state, card.defId).name;
  }
  if (selection.pick === "mode") return selection.option;
  return "nothing";
}

/**
 * §10.6, §10.8: an option's key is what the client sends back, so one key names one option — two
 * Duplicating Felinors in reach are two options, and a key built from the label (the card's name)
 * gave both the same one. The key is built from the selection itself, which is unique among the
 * options by construction, and never from a name (R177 keeps names off what a view may not read).
 */
function keyOf(selection: Selection): string {
  switch (selection.pick) {
    case "instance":
      return `instance:${selection.instanceId}`;
    case "hero":
      return `hero:${selection.player}`;
    case "mode":
      return `mode:${selection.option}`;
    case "zone":
      return `zone:${selection.player}:${selection.row}:${selection.lane}`;
    default:
      return "none";
  }
}

function findOnBoard(ctx: EffectContext, instanceId: string): CardInstance | null {
  for (const player of PLAYER_IDS) {
    const side = ctx.state.players[player];
    const found = [
      ...side.hand,
      ...side.graveyard,
      ...side.units.flatMap((pile) => pile ?? []),
      ...side.backrow.flatMap((card) => (card === null ? [] : [card])),
    ].find((card) => card.id === instanceId);
    if (found !== undefined) return found;
  }
  return null;
}

/**
 * §6.3 Choose one: a mode prompt whose answer resumes the script at `step`.
 *
 * B5 E18: `by: "enemy"` hands the prompt to the other player (Classic #8 Pickle: "your opponent
 * chooses"). The other player sees the options and answers; the answered step still runs as this
 * card's controller (`prompts.PROMPT_OWNER_KEY`), so "you draw" is yours. `labels` gives an option a
 * caption other than its own string (a quest reward's text, a difficulty's rolled reward).
 */
export function chooseMode(args: {
  options: string[];
  step: string;
  prompt?: string;
  data?: Record<string, unknown>;
  /** B5 E18: who answers. Default the card's controller. */
  by?: PlayerSpec;
  /** B5 E18: the caption of each option, by the option; absent is the option itself. */
  labels?: Record<string, string>;
}): Effect {
  return {
    kind: "chooseMode",
    apply(ctx): void {
      openPrompt(ctx, {
        player: playerOf(ctx, args.by ?? "self"),
        owner: ctx.controller,
        kind: "mode",
        prompt: args.prompt ?? "Choose one",
        options: args.options.map((option) => modeOption(option, args.labels?.[option] ?? option)),
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}

/** One fixed option: the string is what the answer carries back (`chosenOptions`). */
function modeOption(option: string, label: string): PromptOption {
  return { key: `mode:${option}`, label, selection: { pick: "mode", option } };
}

/** A target prompt (§10.6). With no legal target the effect fizzles and the card still resolves. */
export function chooseTarget(args: {
  step: string;
  scope?: TargetScope;
  prompt?: string;
  data?: Record<string, unknown>;
}): Effect {
  return {
    kind: "chooseTarget",
    apply(ctx): void {
      const options = targetsInScope(ctx, args.scope);
      if (options.length === 0) return;
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "target",
        prompt: args.prompt ?? "Choose a target",
        options: options.map((selection) => ({
          key: keyOf(selection),
          label: label(ctx, selection),
          selection,
        })),
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}

/**
 * A pick from a hand: your own by default (#26, #80).
 *
 * B5 E16, E17, E18: `of` names whose hand and `by` who picks. `of: "enemy", by: "enemy"` is the
 * other player's own hand pick — Classic #8's "they discard", Classic #9's "they keep one card of
 * their choice" — answered by them, and continued as this card's controller. `of: "enemy"` with the
 * default `by` is Classic #11 Mind Melt's "look at your opponent's hand": their hand cards are the
 * options, which only the chooser is shown (§10.8, R81); the hand's owner sees that a prompt is open
 * and nothing more. `min` and `max` default to `count` picks, fewer when the hand is shorter.
 */
export function chooseFromHand(args: {
  step: string;
  count?: number;
  min?: number;
  max?: number;
  prompt?: string;
  data?: Record<string, unknown>;
  /** Whose hand. Default the card's controller's. */
  of?: PlayerSpec;
  /** Who picks. Default the card's controller. */
  by?: PlayerSpec;
}): Effect {
  return {
    kind: "chooseFromHand",
    apply(ctx): void {
      const hand = ctx.state.players[playerOf(ctx, args.of ?? "self")].hand;
      if (hand.length === 0) return;
      const count = Math.min(args.count ?? 1, hand.length);
      openPrompt(ctx, {
        player: playerOf(ctx, args.by ?? "self"),
        owner: ctx.controller,
        kind: "hand",
        prompt: args.prompt ?? "Choose a card in your hand",
        options: hand.map((card) => ({
          key: `instance:${card.id}`,
          label: defOf(ctx.state, card.defId).name,
          selection: { pick: "instance", instanceId: card.id },
          ...(card.radiant ? { radiant: true as const } : {}),
        })),
        min: Math.min(args.min ?? count, hand.length),
        max: Math.min(args.max ?? count, hand.length),
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}

// ---------------------------------------------------------------------------
// B5 E17, E18: the new prompt kinds (docs/classic-sets.md B5)
// ---------------------------------------------------------------------------

/**
 * B5 E17: the costs present in a hand, each an option of a `number` prompt (Classic #11 Mind Melt's
 * Radiant: "choose a cost; exile every card of that cost from it"). A cost is the one the card would
 * be played for now (R65, `effectiveCost`), which is also what `exileMatching`'s `cost` reads, so the
 * group chosen is the group exiled. Each option's caption names the cards of that cost, which only
 * the chooser is shown; `chosenNumber` reads the answer. An empty hand asks nothing.
 */
export function chooseCostInHand(args: {
  step: string;
  of?: PlayerSpec;
  by?: PlayerSpec;
  prompt?: string;
  data?: Record<string, unknown>;
}): Effect {
  return {
    kind: "chooseCostInHand",
    apply(ctx): void {
      const hand = ctx.state.players[playerOf(ctx, args.of ?? "enemy")].hand;
      const groups = new Map<number, string[]>();
      for (const card of hand) {
        const cost = effectiveCost(ctx.state, card);
        groups.set(cost, [...(groups.get(cost) ?? []), defOf(ctx.state, card.defId).name]);
      }
      const costs = [...groups.keys()].sort((a, b) => a - b);
      openPrompt(ctx, {
        player: playerOf(ctx, args.by ?? "self"),
        owner: ctx.controller,
        kind: "number",
        prompt: args.prompt ?? "Choose a cost",
        options: costs.map((cost) => modeOption(String(cost), `(${cost}) ${(groups.get(cost) ?? []).join(", ")}`)),
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}

/**
 * B5 E18: a number from a fixed range, `from` to `to` inclusive (Classic #18 Glitch in the System
 * declares its 0 to 10 with the play instead, as a `ModeDecl` of kind `number`, R81). The options are
 * the numbers, so they reveal nothing. `chosenNumber` reads the answer.
 */
export function chooseNumber(args: {
  step: string;
  from: number;
  to: number;
  by?: PlayerSpec;
  prompt?: string;
  data?: Record<string, unknown>;
}): Effect {
  return {
    kind: "chooseNumber",
    apply(ctx): void {
      const low = Math.trunc(Math.min(args.from, args.to));
      const high = Math.trunc(Math.max(args.from, args.to));
      const numbers = Array.from({ length: high - low + 1 }, (_, at) => String(low + at));
      openPrompt(ctx, {
        player: playerOf(ctx, args.by ?? "self"),
        owner: ctx.controller,
        kind: "number",
        prompt: args.prompt ?? "Choose a number",
        options: numbers.map((number) => modeOption(number, number)),
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}

/** B5 E18: the number an answered `number` prompt (or a play's `number` mode) carries, or null. */
export function chosenNumber(ctx: EffectContext): number | null {
  for (const option of chosenOptions(ctx)) {
    const value = Number(option);
    if (option.trim() !== "" && Number.isInteger(value)) return value;
  }
  return null;
}

/** R465: the ids the options of an `answer` prompt carry, in the order they are shown. */
export const ANSWER_OPTION_IDS: readonly string[] = ["A", "B", "C", "D", "E", "F", "G", "H"];

/**
 * B5 E18, R465: a multiple-choice problem (Classic+ #42 KY's Test). `statement` is the prompt's text
 * and `options` the answers, `correct` the index of the right one among them. The options are shown
 * in an order the match rng shuffles (unless `shuffle` is false), each under a letter
 * (`ANSWER_OPTION_IDS`) that says nothing about which is right, and the right letter goes into the
 * prompt's resume data under `prompts.ANSWER_KEY` — which `viewFor` never sends and the AI's
 * redaction strips (R185) — so the key never leaves the engine. The answered step reads the verdict
 * with `answeredCorrectly`. At most `ANSWER_OPTION_IDS.length` options; fewer than two asks nothing.
 */
export function chooseAnswer(args: {
  step: string;
  statement: string;
  options: readonly string[];
  correct: number;
  shuffle?: boolean;
  by?: PlayerSpec;
  data?: Record<string, unknown>;
}): Effect {
  return {
    kind: "chooseAnswer",
    apply(ctx): void {
      const given = args.options.slice(0, ANSWER_OPTION_IDS.length).map((text, index) => ({ text, index }));
      if (given.length < 2 || args.correct < 0 || args.correct >= given.length) return;
      const shown = args.shuffle === false ? given : ctx.rng.shuffle(given);
      const at = shown.findIndex((option) => option.index === args.correct);
      const key = ANSWER_OPTION_IDS[at] ?? "";
      openPrompt(ctx, {
        player: playerOf(ctx, args.by ?? "self"),
        owner: ctx.controller,
        kind: "answer",
        prompt: args.statement,
        options: shown.map((option, index) => modeOption(ANSWER_OPTION_IDS[index] ?? String(index), option.text)),
        resume: resumeSelf(ctx, args.step, { ...(args.data ?? {}), [ANSWER_KEY]: key }),
      });
    },
  };
}

/**
 * R465: whether the answer to the step's `answer` prompt was the right one. False with no key in the
 * data: a state whose key was stripped (the AI's redacted copy) judges no answer right, so what the
 * AI simulates never tells one option from another and it answers from what the prompt shows.
 */
export function answeredCorrectly(ctx: EffectContext): boolean {
  const key = answerKeyOf(ctx.data);
  if (key === null) return false;
  const picked = ctx.targets.find((selection) => selection.pick === "mode");
  return picked !== undefined && picked.pick === "mode" && picked.option === key;
}

/** Which board cells a `cell` prompt offers (B5 E18). */
export type CellScope = {
  /** Sides, relative to the card's controller. Default both. */
  side?: "any" | "self" | "enemy";
  /** Rows. Default both. */
  rows?: Row[];
  /** Lanes to leave out (Classic+ #62's lanes already used). */
  exceptLanes?: number[];
};

/**
 * B5 E18: a board cell — a zone of either side and either row, empty or not — as a `zone` selection
 * (Classic+ #62 KY's Papaya's points, asked one at a time). The options run the chooser's own side
 * first, backrow then units, lane 1 upward, then the other side's units then backrow: the rows in the
 * order they lie on the board from the chooser's hero outward. `done` adds an option that picks no
 * cell (`{ pick: "none" }`), for a question a player may stop answering. `chosenCells` reads it.
 */
export function chooseCell(args: {
  step: string;
  cells?: CellScope;
  done?: boolean;
  by?: PlayerSpec;
  prompt?: string;
  data?: Record<string, unknown>;
}): Effect {
  return {
    kind: "chooseCell",
    apply(ctx): void {
      const chooser = playerOf(ctx, args.by ?? "self");
      const scope = args.cells ?? {};
      const rows = scope.rows ?? ["units", "backrow"];
      const except = new Set(scope.exceptLanes ?? []);
      const sides: { player: PlayerId; order: Row[] }[] = [];
      if (scope.side !== "enemy") sides.push({ player: ctx.controller, order: ["backrow", "units"] });
      if (scope.side !== "self") sides.push({ player: opponentOf(ctx.controller), order: ["units", "backrow"] });
      const options: PromptOption[] = [];
      for (const { player, order } of sides) {
        for (const row of order) {
          if (!rows.includes(row)) continue;
          for (let lane = 1; lane <= rowSize(row); lane += 1) {
            if (except.has(lane)) continue;
            const selection: Selection = { pick: "zone", player, row, lane };
            options.push({ key: keyOf(selection), label: `${player} ${row} ${lane}`, selection });
          }
        }
      }
      if (options.length === 0) return;
      if (args.done === true) options.push({ key: "none", label: "Done", selection: { pick: "none" } });
      openPrompt(ctx, {
        player: chooser,
        owner: ctx.controller,
        kind: "cell",
        prompt: args.prompt ?? "Choose a cell",
        options,
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}

/** B5 E18: the cells an answered `cell` prompt picked, in offered order; empty for "done". */
export function chosenCells(ctx: EffectContext): ZoneRef[] {
  return ctx.targets.flatMap((selection) =>
    selection.pick === "zone" ? [{ player: selection.player, row: selection.row, lane: selection.lane }] : [],
  );
}

/**
 * B5 E18: a completed quest's reward (Classic #90 In Too Deep): one of `rewards`, each an id the
 * answer carries back (`chosenOptions`) under its caption. Asked of the card's controller whoever's
 * turn it is — a non-active player's prompt on the opponent's turn (R79's prompt clock).
 */
export function chooseReward(args: {
  step: string;
  rewards: readonly { id: string; label: string }[];
  prompt?: string;
  data?: Record<string, unknown>;
}): Effect {
  return {
    kind: "chooseReward",
    apply(ctx): void {
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "reward",
        prompt: args.prompt ?? "Choose a reward",
        options: args.rewards.map((reward) => modeOption(reward.id, reward.label)),
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}

/** A pile a `pick` prompt draws its options from (B5 E18). */
export type PileSpec = { zone: "graveyard" | "exile" | "hand"; player?: PlayerSpec };

/** Which cards of those piles a `pick` prompt offers. */
export type PickFilter = {
  type?: CardType | CardType[];
  tags?: Tag[];
  costRange?: { min?: number; max?: number };
};

function pickable(state: GameState, card: CardInstance, filter: PickFilter): boolean {
  const types = filter.type === undefined ? undefined : Array.isArray(filter.type) ? filter.type : [filter.type];
  if (types !== undefined && !types.includes(cardTypeOf(state, card))) return false;
  if (filter.tags !== undefined && !filter.tags.every((tag) => defOf(state, card.defId).tags.includes(tag))) {
    return false;
  }
  const cost = effectiveCost(state, card);
  if (filter.costRange?.min !== undefined && cost < filter.costRange.min) return false;
  if (filter.costRange?.max !== undefined && cost > filter.costRange.max) return false;
  return true;
}

/**
 * B5 E18: a budgeted multi-pick from a pile, or several (Classic #34 Ancient Acquisition's "2 cards
 * from your graveyard", Radiant "4 from your graveyard or exile"; Classic #44 Back from the GY's
 * "Units with a total cost of (5) or less"). Every matching card of the piles is an option — no
 * Discover limit of three — carrying its cost as R65 reads it where it lies (`effectiveCost`: a
 * graveyard or exile card at its own cost), and with `budget` the picks may cost no more than it
 * together (`prompts.whyAnswerRefused`; `promptAnswers` lists only sets that fit). `min` and `max`
 * are picks, clamped to what the piles hold; `min` defaults to 0 ("up to"). The picks arrive in
 * `ctx.targets` in offered order. Piles are public here (graveyards, exile, the chooser's own hand),
 * so the prompt is the chooser's alone like any other (R81). No matching card asks nothing.
 */
export function choosePick(args: {
  step: string;
  from: readonly PileSpec[];
  filter?: PickFilter;
  min?: number;
  max: number;
  budget?: number;
  prompt?: string;
  data?: Record<string, unknown>;
}): Effect {
  return {
    kind: "choosePick",
    apply(ctx): void {
      const filter = args.filter ?? {};
      const seen = new Set<string>();
      const options: PromptOption[] = [];
      for (const pile of args.from) {
        const player = playerOf(ctx, pile.player ?? "self");
        for (const card of ctx.state.players[player][pile.zone]) {
          if (seen.has(card.id) || !pickable(ctx.state, card, filter)) continue;
          seen.add(card.id);
          options.push({
            key: `instance:${card.id}`,
            label: defOf(ctx.state, card.defId).name,
            selection: { pick: "instance", instanceId: card.id },
            cost: effectiveCost(ctx.state, card),
            ...(card.radiant ? { radiant: true as const } : {}),
          });
        }
      }
      if (options.length === 0) return;
      const max = Math.max(0, Math.min(Math.trunc(args.max), options.length));
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "pick",
        prompt: args.prompt ?? "Choose cards",
        options,
        min: Math.min(Math.max(0, Math.trunc(args.min ?? 0)), max),
        max,
        ...(args.budget === undefined ? {} : { budget: args.budget }),
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}

/**
 * What a Discover's options are (R247). `card`, the default, offers the definitions themselves: each
 * option is a catalog id, labelled with the card's name, and the view names the card behind it
 * (§10.8). `index` offers the definitions' §5 indices instead — #82 KY's Trial's "Discover among 3
 * distinct random numbers" — so each option is the number, labelled with it and naming no
 * definition, and the resume step turns the number it gets back into its card (`defByIndex`, with the set its
 * pool named, since a number is unique only within its set).
 */
export type DiscoverOffer = "card" | "index";

/**
 * §6.3 Discover: choose 1 of 3, drawn without replacement from the stated pool and shown only to
 * the chooser. The options are definitions, so the resume step decides what to do with the pick —
 * or, with `offer: "index"`, their numbers (R247).
 *
 * `query` may be a function of the context, read when the effect applies rather than when the hook
 * builds its list: a hook is rebuilt each time a paused list resumes (`prompts.runResume`), and a
 * pool that costs something to build — #97 Zephyrs' scorer plays every candidate (§10.7) — is then
 * built once, for the Discover that uses it, and not again for the effects after it.
 */
export function discoverFromCatalog(args: {
  step: string;
  query?: CatalogQueryArgs | ((ctx: EffectContext) => CatalogQueryArgs);
  count?: number;
  prompt?: string;
  data?: Record<string, unknown>;
  /** R247: what each option is, the card or its number. Default `card`. */
  offer?: DiscoverOffer;
}): Effect {
  return {
    kind: "discoverFromCatalog",
    apply(ctx): void {
      const self = ctx.self;
      const asked = typeof args.query === "function" ? args.query(ctx) : args.query;
      // §5.1: a random pool never offers the card that generated it.
      const pool = query(
        excludingDefId(asked ?? {}, self?.defId ?? ctx.defId),
      );
      if (pool.length === 0) return;

      const offered = ctx.rng.shuffle(pool).slice(0, args.count ?? 3);
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "discover",
        prompt: args.prompt ?? "Discover a card",
        options: offered.map((def) => {
          // R247: a number is offered as itself, so nothing in the option names the card it stands for.
          const option = args.offer === "index" ? def.index : def.id;
          return {
            key: `mode:${option}`,
            label: args.offer === "index" ? def.index : def.name,
            selection: { pick: "mode", option },
          };
        }),
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}

/**
 * Which library cards a `discoverFromLibrary` may reveal. It is not a `CatalogQueryArgs`: the pool
 * is a pile of instances rather than the catalog, so only the filters #51 KY's Private Tutor names
 * are here, and a card that needs tags or rarity out of a library should widen this rather than be
 * routed through `catalog.query`, which would offer cards the library does not hold.
 */
export type LibraryFilter = {
  type?: CardType | CardType[];
  costRange?: { min?: number; max?: number };
};

/**
 * #51's engine cell: "Field Trap counts as Trap". A `type` filter matches the field exactly, so
 * asking for "Trap" has to name both fields or #18 Bread and Butter and #71 Intern Stimmy silently
 * vanish from the pool. The reverse does not hold: asking for "Field Trap" means Field Traps only.
 */
const TRAP_TYPES: readonly CardType[] = ["Trap", "Field Trap"];

function filterTypes(filter: LibraryFilter): CardType[] | undefined {
  const asked =
    filter.type === undefined ? [] : Array.isArray(filter.type) ? filter.type : [filter.type];
  if (asked.length === 0) return undefined;
  return asked.flatMap((type) => (type === "Trap" ? [...TRAP_TYPES] : [type]));
}

/**
 * R65: a library card's cost is R65's one calculation for that instance (`effectiveCost`), which is
 * what #30 Archivist and #94 Genn's Greed read (R24, R66): a card never played has no X (so an X-cost
 * card reads 0) and no embiggen price (its base), and its `costMod` and `costOverride` travel with
 * it into every zone (R78), so #95's "every card in your library costs 2 less" moves its bracket.
 */
export function matchesLibraryFilter(state: GameState, card: CardInstance, filter: LibraryFilter): boolean {
  // R218: a unit-token card leaves a library only by being drawn or played (R11), so a reveal that
  // puts the pick in a hand passes over it, as a Recruit does.
  if (isUnitToken(state, card)) return false;
  const types = filterTypes(filter);
  if (types !== undefined && !types.includes(defOf(state, card.defId).type)) return false;

  const cost = effectiveCost(state, card);
  const range = filter.costRange;
  if (range?.min !== undefined && cost < range.min) return false;
  if (range?.max !== undefined && cost > range.max) return false;
  return true;
}

/**
 * §6.3's Discover row is explicit that this is the same primitive with the library as the pool:
 * "'Reveal N matching cards, then choose one' (KY's Private Tutor) is this same primitive with the
 * library as the pool: the revealed cards are that prompt's options, so only the chooser ever sees
 * them (§10.8)". So this is `discoverFromGraveyard` over a filtered library: `count` options drawn
 * without replacement with `ctx.rng.shuffle`, so the revealed cards are always different (R60).
 *
 * §10.8 is what makes revealing safe: "a card revealed out of a library is revealed only as an
 * option of the prompt that reveals it: the chooser sees it in full, the opponent sees only that a
 * prompt is open, and the rest of the library stays hidden from both." Nothing is copied out of
 * `state.pending.options`, so `viewFor` has one place to hide. The prompt therefore goes to
 * `ctx.controller` — the chooser — even when `player` names the other side's library as the pool.
 *
 * The options are real library INSTANCES, not definitions, which is the whole difference from
 * `discoverFromCatalog`: the resume step moves the pick with `addToHand({ instance: { of: "chosen" }
 * })` rather than creating a copy and leaving the revealed card in the library.
 *
 * No match at all opens no prompt: the effect fizzles and the card still resolves (§6.3), which is
 * the branch #51 answers with its Empty Notebook.
 */
export function discoverFromLibrary(args: {
  step: string;
  count?: number;
  player?: PlayerSpec;
  filter?: LibraryFilter;
  prompt?: string;
  data?: Record<string, unknown>;
}): Effect {
  return {
    kind: "discoverFromLibrary",
    apply(ctx): void {
      const player = playerOf(ctx, args.player ?? "self");
      const filter = args.filter ?? {};
      const pool = ctx.state.players[player].library.filter((card) => matchesLibraryFilter(ctx.state, card, filter));
      if (pool.length === 0) return;

      const offered = ctx.rng.shuffle(pool).slice(0, args.count ?? 3);
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "discover",
        prompt: args.prompt ?? "Choose one of the revealed cards",
        options: offered.map((card) => ({
          key: `instance:${card.id}`,
          label: defOf(ctx.state, card.defId).name,
          selection: { pick: "instance", instanceId: card.id },
        })),
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}

/** R50: Discover from the actual graveyard, so spell tokens there are eligible. */
export function discoverFromGraveyard(args: {
  step: string;
  count?: number;
  prompt?: string;
  data?: Record<string, unknown>;
}): Effect {
  return {
    kind: "discoverFromGraveyard",
    apply(ctx): void {
      const graveyard = ctx.state.players[ctx.controller].graveyard;
      if (graveyard.length === 0) return;

      const offered = ctx.rng.shuffle(graveyard).slice(0, args.count ?? 3);
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "discover",
        prompt: args.prompt ?? "Discover a card from your graveyard",
        options: offered.map((card) => ({
          key: `instance:${card.id}`,
          label: defOf(ctx.state, card.defId).name,
          selection: { pick: "instance", instanceId: card.id },
        })),
        resume: resumeSelf(ctx, args.step, args.data ?? {}),
      });
    },
  };
}
