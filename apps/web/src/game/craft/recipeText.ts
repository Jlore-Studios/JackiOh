import type { CardType, CraftEffect, CraftHat, CraftHatKind, CraftKeyword, CraftRecipe, CraftVerb, Keyword } from "../../wire/index.ts";
import { CRAFT_HAT_PRICES, CRAFT_KEYWORD_PRICES, CRAFT_VERB_PRICES } from "../../wire/engineConfig.ts";

/** R1563010: Meditative+ #26 Scratch Pad's code, the text a player types for a recipe. It is a
 * spelling of the recipe and nothing more: `parseRecipeText` reads the text into the recipe the
 * answer carries and the engine judges, as it judges a recipe built of blocks (CLAUDE.md rule 7);
 * `recipeText` writes a recipe back out, so the empty canvas opens as code.
 *
 * ```text
 * card Unit "Pure Closure" {     // the type (Unit, Spell, FieldSpell, Trap) and the two name words
 *   stats(3, 4)                  // a Unit's attack and health
 *   keyword(Taunt)               // a keyword block; Armor takes its N: keyword(Armor, 2)
 *   echo(2)                      // a Spell's Echo
 *   cry {                        // a hat, by its block name, holding its effects
 *     damageTarget(3)            // an effect: its verb, then its N, keyword or card type
 *     grantKeyword(Taunt)
 *   }
 * }
 * ```
 *
 * `//` starts a comment; `;` and line breaks are optional; names are the blocks' own, a name of two
 * words written as one (`FieldSpell`, `DivineShield`, `FirstStrike`). The cost is
 * the prompt's, chosen before. A misspelt or misplaced word is a syntax error with its line, as a
 * compiler gives one; whether the card may be crafted is the engine's verdict alone. */

/** The text's word for each card type: the type's name, one word. */
const TYPE_WORDS: Readonly<Record<string, CardType>> = {
  Unit: "Unit",
  Spell: "Spell",
  FieldSpell: "Field Spell",
  Trap: "Trap",
};

const VERBS: ReadonlySet<string> = new Set(CRAFT_VERB_PRICES.map((row) => row.verb));
const HATS: ReadonlySet<string> = new Set(CRAFT_HAT_PRICES.map((row) => row.hat));

/** A keyword or a card type as one code word: its name with the spaces taken out. */
export function codeWord(name: string): string {
  return name.replaceAll(" ", "");
}

/** The text's word for each keyword. */
const KEYWORD_WORDS: ReadonlyMap<string, Keyword["kind"]> = new Map(
  CRAFT_KEYWORD_PRICES.map((row) => [codeWord(row.kind), row.kind]),
);

type Token = { kind: "word" | "number" | "string" | "punct"; text: string; line: number };

/** What a parse gives: the recipe, or every syntax error found with its line. */
export type ParsedRecipe = { ok: true; recipe: CraftRecipe } | { ok: false; errors: string[] };

class Syntax extends Error {}

function tokenize(text: string): Token[] {
  const tokens: Token[] = [];
  text.split("\n").forEach((raw, index) => {
    const line = index + 1;
    const code = raw.replace(/\/\/.*$/, "");
    const pattern = /\s*(?:([A-Za-z_][A-Za-z0-9_]*)|(-?\d+)|"([^"]*)"|([{}(),;])|(\S))/gy;
    let match: RegExpExecArray | null;
    while ((match = pattern.exec(code)) !== null) {
      const [, word, number, string, punct, stray] = match;
      if (word !== undefined) tokens.push({ kind: "word", text: word, line });
      else if (number !== undefined) tokens.push({ kind: "number", text: number, line });
      else if (string !== undefined) tokens.push({ kind: "string", text: string, line });
      else if (punct !== undefined) tokens.push({ kind: "punct", text: punct, line });
      else throw new Syntax(`line ${line}: unexpected "${stray ?? ""}"`);
    }
  });
  return tokens;
}

class Reader {
  private at = 0;
  constructor(private readonly tokens: Token[]) {}

  peek(): Token | undefined {
    return this.tokens[this.at];
  }

  line(): number {
    return this.peek()?.line ?? this.tokens[this.tokens.length - 1]?.line ?? 1;
  }

  next(what: string): Token {
    const token = this.tokens[this.at];
    if (token === undefined) throw new Syntax(`line ${this.line()}: expected ${what}, found the end`);
    this.at += 1;
    return token;
  }

  expect(punct: string): void {
    const token = this.next(`"${punct}"`);
    if (token.kind !== "punct" || token.text !== punct) {
      throw new Syntax(`line ${token.line}: expected "${punct}", found "${token.text}"`);
    }
  }

  is(punct: string): boolean {
    const token = this.peek();
    return token?.kind === "punct" && token.text === punct;
  }

  /** Skip the optional `;` between statements. */
  semicolons(): void {
    while (this.is(";")) this.at += 1;
  }

  done(): boolean {
    return this.at >= this.tokens.length;
  }

  /** `( arg, arg … )`: the arguments, each a number or a word. */
  args(): Token[] {
    this.expect("(");
    const out: Token[] = [];
    while (!this.is(")")) {
      const token = this.next("an argument or \")\"");
      if (token.kind !== "number" && token.kind !== "word") {
        throw new Syntax(`line ${token.line}: expected a number or a name, found "${token.text}"`);
      }
      out.push(token);
      if (this.is(",")) this.expect(",");
      else if (!this.is(")")) throw new Syntax(`line ${this.line()}: expected "," or ")"`);
    }
    this.expect(")");
    return out;
  }
}

function numberArg(token: Token | undefined, what: string, line: number): number {
  if (token === undefined || token.kind !== "number") throw new Syntax(`line ${token?.line ?? line}: ${what} takes a number`);
  return Number(token.text);
}

function keywordArg(token: Token | undefined, line: number): Keyword["kind"] {
  const kind = token?.kind === "word" ? KEYWORD_WORDS.get(token.text) : undefined;
  if (kind === undefined) throw new Syntax(`line ${token?.line ?? line}: no keyword is called "${token?.text ?? ""}"`);
  return kind;
}

function effectOf(verbToken: Token, args: Token[]): CraftEffect {
  const verb = verbToken.text as CraftVerb;
  const effect: CraftEffect = { verb };
  for (const arg of args) {
    if (arg.kind === "number") {
      if (effect.n !== undefined) throw new Syntax(`line ${arg.line}: ${verb} takes one number`);
      effect.n = Number(arg.text);
    } else if (verb === "grantKeyword") {
      if (effect.keyword !== undefined) throw new Syntax(`line ${arg.line}: ${verb} takes one keyword`);
      effect.keyword = keywordArg(arg, arg.line);
    } else {
      const type = TYPE_WORDS[arg.text];
      if (type === undefined) throw new Syntax(`line ${arg.line}: no card type is called "${arg.text}"`);
      if (effect.cardType !== undefined) throw new Syntax(`line ${arg.line}: ${verb} takes one card type`);
      effect.cardType = type;
    }
  }
  return effect;
}

/** Read Scratch Pad's code into a recipe at the prompt's `cost`, or give its syntax errors. */
export function parseRecipeText(text: string, cost: number): ParsedRecipe {
  try {
    const reader = new Reader(tokenize(text));
    const head = reader.next("\"card\"");
    if (head.kind !== "word" || head.text !== "card") throw new Syntax(`line ${head.line}: a card's code starts with "card"`);
    const typeToken = reader.next("a card type");
    const type = TYPE_WORDS[typeToken.text];
    if (type === undefined) throw new Syntax(`line ${typeToken.line}: no card type is called "${typeToken.text}"`);
    const nameToken = reader.next("the card's name");
    const words = nameToken.kind === "string" ? nameToken.text.trim().split(/\s+/) : [];
    if (words.length !== 2) throw new Syntax(`line ${nameToken.line}: the name is two words in quotes, as "Pure Closure"`);
    const recipe: CraftRecipe = {
      cost,
      type,
      adjective: words[0] ?? "",
      noun: words[1] ?? "",
      attack: 0,
      health: 1,
      keywords: [],
      echo: 0,
      hats: [],
    };
    reader.expect("{");
    reader.semicolons();
    while (!reader.is("}")) {
      const word = reader.next("a statement or \"}\"");
      if (word.kind !== "word") throw new Syntax(`line ${word.line}: unexpected "${word.text}"`);
      if (word.text === "stats") {
        const [attack, health, ...rest] = reader.args();
        if (rest.length > 0) throw new Syntax(`line ${word.line}: stats takes two numbers`);
        recipe.attack = numberArg(attack, "stats", word.line);
        recipe.health = numberArg(health, "stats", word.line);
      } else if (word.text === "keyword") {
        const [kind, n, ...rest] = reader.args();
        if (rest.length > 0) throw new Syntax(`line ${word.line}: keyword takes a keyword and at most its number`);
        const block: CraftKeyword = { kind: keywordArg(kind, word.line) };
        if (n !== undefined) block.n = numberArg(n, "keyword", word.line);
        recipe.keywords = [...recipe.keywords, block];
      } else if (word.text === "echo") {
        const [n, ...rest] = reader.args();
        if (rest.length > 0) throw new Syntax(`line ${word.line}: echo takes one number`);
        recipe.echo = numberArg(n, "echo", word.line);
      } else if (HATS.has(word.text)) {
        reader.expect("{");
        reader.semicolons();
        const effects: CraftEffect[] = [];
        while (!reader.is("}")) {
          const verb = reader.next("an effect or \"}\"");
          if (verb.kind !== "word" || !VERBS.has(verb.text)) {
            throw new Syntax(`line ${verb.line}: no effect is called "${verb.text}"`);
          }
          effects.push(effectOf(verb, reader.args()));
          reader.semicolons();
        }
        reader.expect("}");
        const hat: CraftHat = { hat: word.text as CraftHatKind, effects };
        recipe.hats = [...recipe.hats, hat];
      } else {
        throw new Syntax(`line ${word.line}: "${word.text}" is no statement (stats, keyword, echo or a hat)`);
      }
      reader.semicolons();
    }
    reader.expect("}");
    if (!reader.done()) throw new Syntax(`line ${reader.line()}: nothing may follow the card's closing "}"`);
    return { ok: true, recipe };
  } catch (error) {
    if (error instanceof Syntax) return { ok: false, errors: [error.message] };
    throw error;
  }
}

function argText(effect: CraftEffect): string {
  const args: string[] = [];
  if (effect.n !== undefined) args.push(String(effect.n));
  if (effect.keyword !== undefined) args.push(codeWord(effect.keyword));
  if (effect.cardType !== undefined) args.push(codeWord(effect.cardType));
  return args.join(", ");
}

/** A recipe as Scratch Pad's code: what `parseRecipeText` reads back into it. */
export function recipeText(recipe: CraftRecipe): string {
  const lines = [`card ${codeWord(recipe.type)} "${recipe.adjective} ${recipe.noun}" {`];
  if (recipe.type === "Unit") lines.push(`  stats(${recipe.attack}, ${recipe.health})`);
  for (const block of recipe.keywords) {
    const kind = codeWord(block.kind);
    lines.push(block.n === undefined ? `  keyword(${kind})` : `  keyword(${kind}, ${block.n})`);
  }
  if (recipe.echo > 0) lines.push(`  echo(${recipe.echo})`);
  for (const hat of recipe.hats) {
    lines.push(`  ${hat.hat} {`);
    for (const effect of hat.effects) lines.push(`    ${effect.verb}(${argText(effect)})`);
    lines.push("  }");
  }
  lines.push("}");
  return `${lines.join("\n")}\n`;
}
