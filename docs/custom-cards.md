# Custom cards: the card kit

A custom card is a card you make (SPEC §12): a frame (name, type, cost, stats, keywords, text and its
numbers) and a script in the **card kit**, a small set of words over the engine. This page is the kit's
reference. The creator's builder writes kit code for you; the code view shows and edits the same card.

Custom cards are saved on your account, private to you, and play only in practice against the AI, in your
own deck (§12.6). The AI never holds one. Official cards are in Rust; Luau runs only custom cards, in a
browser module that loads for a game with a custom card and no other.

## A card

```lua
-- Book of Heal, as a custom card. Spell. Text: "Heal a unit or hero for {heal}."
return Card {
  target = "a unit or hero",
  cry = function(c, n)
    c:heal(c:target(), n.heal)
  end,
}
```

A script returns `Card { … }`, a table of the card's hooks. Each hook is a function of `c`, the card's
view of the moment, and `n`, the card's numbers. A hook does things by calling `c`'s verbs; each call is recorded, in order, and the
effects resolve as an official card's do once the hook returns.

`Card` knows these keys, and refuses any other:

| Key | What it is |
| --- | --- |
| `cry` | A Unit's Cry, run as it is played; a Spell's effect, run as it is cast. |
| `death` | A Unit's Death, run as it dies. |
| `startOfTurn` | Run at the start of its controller's turn while it is on the field. |
| `endOfTurn` | Run at the end of its controller's turn while it is on the field. |
| `aura` | Stat and keyword changes while it is on the field (below). |
| `activate` | An Activate ability its controller may use once a turn (below). |
| `target` | One card or hero the player chooses as the card is played, for its `cry` (below). |

## Numbers

Each number on the card has a name, written `{name}` in its text: `n.name` reads it, on the face the card
is on (base or Radiant) and after any Degrade, Upgrade or KY's Constant. A card whose Radiant face only
has bigger numbers needs no Radiant code. `c.radiant` is `true` on the Radiant face. Numbers are `n`'s and
verbs are `c`'s, so a number may share a verb's name: `c:heal("your hero", n.heal)`. A name the card does
not declare stops the hook with an error naming it.

```lua
-- "Deal {dmg} damage to an enemy unit."
cry = function(c, n)
  c:damage("an enemy unit", n.dmg)
end
```

Every number is a whole number. A verb given anything else stops the hook with an error naming the line.

## Targets

A verb's target is words, or what `c:target` answered.

**One card or hero the player chooses** (asked when the script reaches it, below):
`"an enemy unit"`, `"an enemy"`, `"an enemy unit or hero"`, `"a friendly unit"`, `"another friendly unit"`,
`"a friendly unit or hero"`, `"a unit"`, `"another unit"`, `"a unit or hero"`, `"a character"`. A choice
never includes the card itself.

**One thing, no choice:** `"your hero"`, `"the enemy hero"`, `"this unit"` (the card itself; also
`"this card"`).

**Every unit of a side:** `"each enemy unit"`, `"each friendly unit"`, `"each other friendly unit"`,
`"each unit"`, `"each other unit"`; with the heroes too: `"all enemies"` (also `"each enemy"`) and
`"everything"`. A group is read off the board as the hook runs.

**Random:** `"a random enemy"` and `"a random enemy unit"` (for `c:damage`), `"a random friendly unit"`
(for `c:buff`). The match picks when the effect resolves.

## Verbs

| Verb | Does |
| --- | --- |
| `c:damage(target, n)` | Deals n damage. |
| `c:heal(target, n)` | Heals n: a unit up to its maximum health, a hero with no cap. |
| `c:destroy(target)` | Destroys the units it names. |
| `c:bounce(target)` | Returns the units it names to their owners' hands. |
| `c:freeze(target)` | Freezes the units it names: each loses its next attack or position switch, and thaws at the end of the turn it could have made one. A hero is never Frozen. |
| `c:buff(target, attack, health)` | Gives the units it names +attack/+health (health defaults to attack). |
| `c:give(target, keyword)` | Gives the units it names a keyword: Taunt, Rush, Charge, First Strike, Poisonous, Lifesteal, Reborn, Divine Shield, Trample, Windfury, Ranged or Flying. |
| `c:draw(n)` | You draw n cards (1 by default). |
| `c:discard(n, who)` | Discards n random cards: yours, or `"the enemy"`'s. |
| `c:summon(token, n)` | Summons n tokens (1 by default) for you: `"Rush Token"`, `"Sheep Token"` or `"Felinor Token"`. |
| `c:armor(n)` | Your hero gains n Armor. |
| `c:mana(n)` | You gain n mana this turn. |

A verb aimed at a choice that had nothing to choose does nothing.

## Reading the board

| Reader | Answers |
| --- | --- |
| `c:count(what)` | How many `"friendly units"`, `"enemy units"`, `"units"`, `"cards in your hand"` or `"cards in the enemy hand"` there are. |
| `c:health(target)` | A hero's or a unit's health (one card or hero; 0 for nothing). |
| `c:mana_left()` | Your unspent mana. |
| `c.your_turn` | Whether it is your turn. |
| `c:each(group, function(card) … end)` | Runs the function for each unit of a group, then each hero it includes. |

A reading sees the board as the hook began: verbs are recorded, not done, until the hook returns.

```lua
-- "Give each friendly unit +1/+1. If you have 3 or more, draw a card."
cry = function(c)
  c:buff("each friendly unit", 1, 1)
  if c:count("friendly units") >= 3 then
    c:draw(1)
  end
end
```

## Choices

`c:target(words)` asks the player for one card or hero, and `c:choose({ … })` for one of two or more
options (it answers the option's text), wherever the script reaches them:

```lua
-- "Choose one: deal {dmg} damage to an enemy unit, or draw {cards} cards."
cry = function(c, n)
  if c:choose({ "Deal damage", "Draw cards" }) == "Deal damage" then
    c:damage(c:target("an enemy unit"), n.dmg)
  else
    c:draw(n.cards)
  end
end
```

The engine keeps a paused game as plain data, so a question cannot pause the script itself. The effects
before the question resolve, the question is asked, and the answer runs the hook again from the top:
every reading it made before the question answers as it did, so the script walks the same path, and the
effects that already resolved are not made again (§12.4). A script never notices, but this is why a
hook must do everything through `c`: a value it kept anywhere else would not survive the question.

`c:target(words)` with nothing to choose answers `nil`, without asking.

**A target chosen as the card is played.** A card with `target = "…"` asks for that target as it is
played, as an official card does; `c:target()` with no words is that choice. Use it for a Spell or a Cry
whose target is the point of the card: the play is refused or allowed by the same rules as an official
card's.

## Auras

```lua
-- "Your units have +1 Attack and Taunt."
aura = { attack = 1, give = "Taunt", to = "your units" },
```

`attack` and `health` are whole numbers or the name of one of the card's numbers (`attack = "bonus"`);
`give` is a keyword `c:give` takes, or a list of them; `to` is `"your units"` (the default), `"enemy units"` or
`"all units"`. Your units include the card itself.

## Activate

```lua
activate = {
  label = "Draw a card",
  cost = 2,
  run = function(c, n)
    c:draw(1)
  end,
},
```

Once a turn its controller may pay `cost` mana (0 by default) and run `run`, which may ask questions and do
anything a hook does.

## When a script fails

A hook runs under the sandbox, the integer boundary, a memory cap and an instruction budget of
`CUSTOM_HOOK_INTERRUPTS`. A hook that errors, loops past its budget or runs out of memory does nothing:
none of its effects resolve, and the board says which card failed and why. The game goes on (§12.5).

The creator checks a script before it saves it: Luau's compiler, the lint (no `pairs`, no `/`, no
fractions, nothing the sandbox removed) and `Card { … }`'s own checks, each shown on its line.

## What a script cannot do

- `require` anything: the kit is all a card sees; the engine's raw verbs stay underneath.
- Iterate a table in hash order (`pairs`, `next`), sort, or make a fraction (`/`, `^`): a game must fold
  the same everywhere (L6, L7). Use `for _, x in ipairs(list) do`, `//` for whole division and
  `math.floor`.
- Keep anything between runs: a hook's locals start fresh on every run.

## The creator

`/creator` (from the landing page's "Make cards" and the deck builder) lists your custom cards and
edits one at a time (§12.11, R915010). The form holds the frame; the card's effects are written in one
of two views of the same card:

- **Builder**: blocks picked from menus, every word one of the kit's. Pick the target the card asks
  for as it is played, add triggers (Cry, Death, start or end of your turn), give each its effects and
  an optional condition ("only if you have at least 3 friendly units"), and for a Unit an aura or an
  Activate. Each amount is one of the card's numbers (`dmg`, `dmg2`, `cards`, …), whose Radiant value
  the numbers table sets, and the builder writes the card's text from the blocks until you write your
  own. `apps/web/src/creator/builder.ts` is the whole vocabulary and the Luau it writes.
- **Code**: the script itself, highlighted, with each problem the Luau module's check finds marked on
  its line. Anything in this guide can be written here.

A card whose script is still exactly what its blocks write opens in the builder; one written or
changed in code stays code-only, and the builder says so (undo the change to go back, or start over).
The frame's issues are the engine validator's, shown under their fields as you type; the script's come
from the Luau module, which the creator loads the first time it checks one. A card with a script
problem is not saved. Saving puts it on your account, so every device you sign in on has it; a save
from a tab that is behind another save is refused and offers the newer one. "Try it against the AI"
saves the card and starts a practice game with a random deck holding it.

## Practice

A custom card goes in a deck like any other card (the deck builder's "Custom" filter), and the deck is
then marked practice only: the queue, rooms, rematches and ranked play refuse it (L7, R915005), and the
server refuses it too. Started from practice, a game whose deck holds custom cards runs in the Luau
module (R915009); the AI's deck stays a normal deck. The game records no card statistics, no ladder
and no tutorial progress, and it is kept to resume and to replay with the cards as they were when it
was dealt (R915006), so editing a card later never changes a game already played. A deck naming a card
you deleted cannot start until the card is replaced.

## Limits

| | |
| --- | --- |
| Name | 1 to `CUSTOM_NAME_MAX_LENGTH` (40) characters |
| Type | Unit or Spell |
| Cost | 0 to `CUSTOM_COST_MAX` (10) |
| Attack / health | 0 to `CUSTOM_ATTACK_MAX` (12) / 1 to `CUSTOM_HEALTH_MAX` (12); Radiant up to `CUSTOM_RADIANT_STAT_MAX` (24), twice the base by default |
| Keywords | at most `CUSTOM_KEYWORDS_MAX` (4), from the list above |
| Text | at most `CUSTOM_TEXT_MAX_LENGTH` (240) characters |
| Numbers | at most `CUSTOM_PARAMS_MAX` (6), each 0 to `CUSTOM_PARAM_MAX` (20) on each face |
| Script | at most `CUSTOM_SOURCE_MAX_BYTES` (8,192) bytes |
| Cards per account | `CUSTOM_CARDS_PER_ACCOUNT` (50) |
