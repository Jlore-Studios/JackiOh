// The creator's no-code builder (#920, SPEC §12.3, K3): a custom card as blocks a player picks from
// menus, and the card-kit Luau those blocks write. The builder knows only the kit's own words (#913,
// docs/custom-cards.md), so everything it writes is a script the kit runs; the code view shows and
// edits the same script.
//
// The model is plain data, saved beside the card (`CustomCard.builder`) and read back by the creator
// alone; the engine never reads it (R915006). The script it writes is a pure function of the model
// (`writeScript`), and the card's text of the model and the frame (`writeText`), so a card whose
// source still equals what its model writes is the builder's to show; one edited in code beyond
// that is code-only (`builderShows`), and says why.
//
// Numbers. Each amount a block names is one of the card's numbers (`{key}` in the text, `n.key` in
// the script), so Degrade and Upgrade move it and its Radiant face may print another value. The
// builder keeps their keys itself (`numbersOf`): a verb's own word, with a digit when two blocks
// share it (`dmg`, `dmg2`).
//
// Nothing here is a rule (CLAUDE.md rule 7): the frame and the script are judged by the engine's
// validator and the Luau module's check, and the builder only offers what they accept.

/** The kit's hooks a block list can hang on. */
export const TRIGGERS = ["cry", "death", "startOfTurn", "endOfTurn"] as const;
export type Trigger = (typeof TRIGGERS)[number];

/** Words that name one card or hero the player chooses (the kit's `ONE`). */
export const ONE_WORDS = [
  "an enemy unit",
  "an enemy",
  "a friendly unit",
  "a unit",
  "a unit or hero",
] as const;

/** Words that name a thing without a choice, a group, or a random one (the kit's other lists). */
export const OTHER_WORDS = [
  "your hero",
  "the enemy hero",
  "this unit",
  "each enemy unit",
  "each friendly unit",
  "each other friendly unit",
  "each unit",
  "all enemies",
  "everything",
  "a random enemy",
  "a random enemy unit",
  "a random friendly unit",
] as const;

/** What `c:count` counts. */
export const COUNT_WORDS = ["friendly units", "enemy units", "units", "cards in your hand", "cards in the enemy hand"] as const;

/** The tokens `c:summon` knows. */
export const TOKEN_WORDS = ["Rush Token", "Sheep Token", "Felinor Token"] as const;

/** Where an aura's changes go. */
export const AURA_WORDS = ["your units", "enemy units", "all units"] as const;

/** A block's target: words, the card's declared play-time target, or a target asked mid-script. */
export type BuilderTarget =
  | { kind: "declared" }
  | { kind: "ask"; words: (typeof ONE_WORDS)[number] }
  | { kind: "words"; words: (typeof OTHER_WORDS)[number] };

/** One block: a verb, what it aims at, its amounts. `amount`s are the base values of the card's numbers. */
export type BuilderStep =
  | { verb: "damage" | "heal"; target: BuilderTarget; amount: number }
  | { verb: "destroy" | "bounce"; target: BuilderTarget }
  | { verb: "buff"; target: BuilderTarget; attack: number; health: number }
  | { verb: "give"; target: BuilderTarget; keyword: string }
  | { verb: "draw" | "armor" | "mana"; amount: number }
  | { verb: "discard"; amount: number; who: "you" | "the enemy" }
  | { verb: "summon"; token: (typeof TOKEN_WORDS)[number]; amount: number };

export type BuilderVerb = BuilderStep["verb"];

/** A block list's optional condition: run the blocks only when a count compares so. */
export type BuilderCondition = { count: (typeof COUNT_WORDS)[number]; op: ">=" | "<="; value: number };

export type BuilderHook = { trigger: Trigger; condition?: BuilderCondition; steps: BuilderStep[] };

export type BuilderAura = {
  attack: number;
  health: number;
  give: string[];
  to: (typeof AURA_WORDS)[number];
};

export type BuilderActivate = { label: string; cost: number; steps: BuilderStep[] };

/** The builder's whole model of one card. */
export type BuilderModel = {
  version: 1;
  /** The Cry's target chosen as the card is played; null for none. */
  target: (typeof ONE_WORDS)[number] | null;
  hooks: BuilderHook[];
  aura: BuilderAura | null;
  activate: BuilderActivate | null;
  /** Each number's Radiant value by its key; a key not here is twice its base value. */
  radiant: Record<string, number>;
};

export function emptyModel(): BuilderModel {
  return { version: 1, target: null, hooks: [], aura: null, activate: null, radiant: {} };
}

/** A verb's word for its numbers. */
const NUMBER_WORD: Readonly<Record<BuilderVerb, string>> = {
  damage: "dmg",
  heal: "heal",
  destroy: "",
  bounce: "",
  buff: "buff",
  give: "",
  draw: "cards",
  armor: "armor",
  mana: "mana",
  discard: "discard",
  summon: "tokens",
};

/** One number a block names: its key, its base value, which block and field it is. */
export type BuilderNumber = { key: string; base: number; radiant: number };

/** Every block of the model, hook by hook, then the Activate's. */
function blocks(model: BuilderModel): BuilderStep[] {
  return [...model.hooks.flatMap((hook) => hook.steps), ...(model.activate?.steps ?? [])];
}

/** The fields of a block that are numbers, in order. */
function amountsOf(step: BuilderStep): number[] {
  switch (step.verb) {
    case "damage":
    case "heal":
    case "draw":
    case "armor":
    case "mana":
    case "discard":
    case "summon":
      return [step.amount];
    case "buff":
      return [step.attack, step.health];
    case "destroy":
    case "bounce":
    case "give":
      return [];
  }
}

/**
 * The card's numbers, in block order: each amount gets the verb's word, with a digit from the
 * second on (`dmg`, `dmg2`; a buff's attack and health are `buff` and `buff2`). The keys are the
 * builder's, so editing a block never renames another's.
 */
export function numbersOf(model: BuilderModel): BuilderNumber[] {
  const seen = new Map<string, number>();
  const numbers: BuilderNumber[] = [];
  for (const step of blocks(model)) {
    for (const base of amountsOf(step)) {
      const word = NUMBER_WORD[step.verb];
      const count = (seen.get(word) ?? 0) + 1;
      seen.set(word, count);
      const key = count === 1 ? word : `${word}${String(count)}`;
      numbers.push({ key, base, radiant: model.radiant[key] ?? base * 2 });
    }
  }
  return numbers;
}

/** The keys of one block's numbers, in the order `numbersOf` gave them. */
function keysByBlock(model: BuilderModel): string[][] {
  const keys = numbersOf(model).map((number) => number.key);
  let at = 0;
  return blocks(model).map((step) => {
    const mine = keys.slice(at, at + amountsOf(step).length);
    at += mine.length;
    return mine;
  });
}

function quoted(text: string): string {
  return JSON.stringify(text);
}

function targetLua(target: BuilderTarget): string {
  switch (target.kind) {
    case "declared":
      return "c:target()";
    case "ask":
    case "words":
      return quoted(target.words);
  }
}

function stepLua(step: BuilderStep, keys: readonly string[]): string {
  const n = (at: number): string => `n.${keys[at] ?? "missing"}`;
  switch (step.verb) {
    case "damage":
    case "heal":
      return `c:${step.verb}(${targetLua(step.target)}, ${n(0)})`;
    case "destroy":
    case "bounce":
      return `c:${step.verb}(${targetLua(step.target)})`;
    case "buff":
      return `c:buff(${targetLua(step.target)}, ${n(0)}, ${n(1)})`;
    case "give":
      return `c:give(${targetLua(step.target)}, ${quoted(step.keyword)})`;
    case "draw":
    case "armor":
    case "mana":
      return `c:${step.verb}(${n(0)})`;
    case "discard":
      return `c:discard(${n(0)}, ${quoted(step.who)})`;
    case "summon":
      return `c:summon(${quoted(step.token)}, ${n(0)})`;
  }
}

/** The script the model writes: `Card { … }` in the kit, a pure function of the model. */
export function writeScript(model: BuilderModel): string {
  const keys = keysByBlock(model);
  let at = 0;
  const body = (steps: readonly BuilderStep[], indent: string): string[] =>
    steps.map((step) => {
      const line = `${indent}${stepLua(step, keys[at] ?? [])}`;
      at += 1;
      return line;
    });
  const lines: string[] = ["-- Written by the card creator's builder.", "return Card {"];
  if (model.target !== null) lines.push(`  target = ${quoted(model.target)},`);
  for (const hook of model.hooks) {
    lines.push(`  ${hook.trigger} = function(c, n)`);
    if (hook.condition === undefined) {
      lines.push(...body(hook.steps, "    "));
    } else {
      const { count, op, value } = hook.condition;
      lines.push(`    if c:count(${quoted(count)}) ${op} ${String(value)} then`);
      lines.push(...body(hook.steps, "      "));
      lines.push("    end");
    }
    lines.push("  end,");
  }
  if (model.aura !== null) {
    const { attack, health, give, to } = model.aura;
    const fields = [
      ...(attack === 0 ? [] : [`attack = ${String(attack)}`]),
      ...(health === 0 ? [] : [`health = ${String(health)}`]),
      ...(give.length === 0 ? [] : [`give = { ${give.map(quoted).join(", ")} }`]),
      `to = ${quoted(to)}`,
    ];
    lines.push(`  aura = { ${fields.join(", ")} },`);
  }
  if (model.activate !== null) {
    lines.push("  activate = {");
    lines.push(`    label = ${quoted(model.activate.label)},`);
    lines.push(`    cost = ${String(model.activate.cost)},`);
    lines.push("    run = function(c, n)");
    lines.push(...body(model.activate.steps, "      "));
    lines.push("    end,");
    lines.push("  },");
  }
  lines.push("}");
  return `${lines.join("\n")}\n`;
}

const TRIGGER_WORD: Readonly<Record<Trigger, string>> = {
  cry: "Cry",
  death: "Death",
  startOfTurn: "Start of turn",
  endOfTurn: "End of turn",
};

function targetText(target: BuilderTarget, declared: string | null): string {
  if (target.kind === "declared") return declared ?? "a target";
  return target.words;
}

/** A number with its noun, singular or plural by its value (`{key|card|cards}`, as the catalog writes). */
function plural(key: string | undefined, one: string, many: string): string {
  return `{${key ?? "?"}|${one}|${many}}`;
}

function stepText(step: BuilderStep, keys: readonly string[], declared: string | null): string {
  const n = (at: number): string => `{${keys[at] ?? "?"}}`;
  switch (step.verb) {
    case "damage":
      return `deal ${n(0)} damage to ${targetText(step.target, declared)}`;
    case "heal":
      return `heal ${targetText(step.target, declared)} for ${n(0)}`;
    case "destroy":
      return `destroy ${targetText(step.target, declared)}`;
    case "bounce":
      return `return ${targetText(step.target, declared)} to its owner's hand`;
    case "buff":
      return `give ${targetText(step.target, declared)} +${n(0)}/+${n(1)}`;
    case "give":
      return `give ${targetText(step.target, declared)} ${step.keyword}`;
    case "draw":
      return `draw ${plural(keys[0], "card", "cards")}`;
    case "armor":
      return `gain ${n(0)} Armor`;
    case "mana":
      return `gain ${n(0)} mana`;
    case "discard":
      return step.who === "you"
        ? `discard ${plural(keys[0], "random card", "random cards")}`
        : `the enemy discards ${plural(keys[0], "random card", "random cards")}`;
    case "summon":
      return `summon ${plural(keys[0], step.token, `${step.token}s`)}`;
  }
}

function sentence(parts: readonly string[]): string {
  const joined = parts.join(", then ");
  return `${joined.charAt(0).toUpperCase()}${joined.slice(1)}.`;
}

/** The card's text the model writes, each `{key}` one of `numbersOf`'s numbers. */
export function writeText(model: BuilderModel): string {
  const keys = keysByBlock(model);
  let at = 0;
  const texts = (steps: readonly BuilderStep[]): string[] =>
    steps.map((step) => {
      const text = stepText(step, keys[at] ?? [], model.target);
      at += 1;
      return text;
    });
  const lines: string[] = [];
  for (const hook of model.hooks) {
    const said = texts(hook.steps);
    if (said.length === 0) continue;
    const condition =
      hook.condition === undefined
        ? ""
        : `If you have ${hook.condition.op === ">=" ? "at least" : "at most"} ${String(hook.condition.value)} ${hook.condition.count}: `;
    lines.push(`${TRIGGER_WORD[hook.trigger]}: ${condition}${sentence(said)}`);
  }
  if (model.aura !== null) {
    const { attack, health, give, to } = model.aura;
    const parts = [
      ...(attack === 0 && health === 0 ? [] : [`+${String(attack)}/+${String(health)}`]),
      ...give,
    ];
    if (parts.length > 0) lines.push(`Aura: ${to.charAt(0).toUpperCase()}${to.slice(1)} have ${parts.join(", ")}.`);
  }
  if (model.activate !== null) {
    const said = texts(model.activate.steps);
    const cost = model.activate.cost > 0 ? ` (${String(model.activate.cost)})` : "";
    if (said.length > 0) lines.push(`Activate${cost}: ${sentence(said)}`);
  }
  return lines.join("\n");
}

/**
 * K3: whether the builder can show the card: it has a model, and the source is exactly what that
 * model writes. A card edited in code beyond that is code-only.
 */
export function builderShows(model: BuilderModel | null, source: string): boolean {
  return model !== null && writeScript(model) === source;
}

/** Whether `value` is a builder model this build can read (a card saved by another build may not be). */
export function isBuilderModel(value: unknown): value is BuilderModel {
  if (typeof value !== "object" || value === null) return false;
  const model = value as Partial<BuilderModel>;
  return (
    model.version === 1 &&
    Array.isArray(model.hooks) &&
    (model.target === null || (typeof model.target === "string" && (ONE_WORDS as readonly string[]).includes(model.target))) &&
    typeof model.radiant === "object" &&
    model.radiant !== null
  );
}
