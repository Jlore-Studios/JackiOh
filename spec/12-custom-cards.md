## 12. Custom cards

A player may make cards of their own: a frame (name, type, cost, stats, keywords, text and its numbers) and a script in the card kit, a small readable layer of Luau over the engine's verbs. Custom cards are saved on the player's account, private to them, and played only in practice against the AI, in the player's own deck ([[R915000]]). Every official card is in Rust ([[§10.9]]); Luau runs only a custom card's script, in a second browser module that a normal game never loads ([[R915009]]).

### 12.1 What a custom card is

- **Its id** is `custom-` followed by 1 to 57 lowercase letters, digits and hyphens, `CUSTOM_ID_MAX_LENGTH` (64) characters in all; the server mints it when the card is first saved. No catalog id begins with `custom-`, so a custom card never shares an id with a catalog card or a token ([[R915000]]).
- **Its definition** is built from its frame: set `Custom`, which never ships ([[R1420]]), so no random pool, Discover or random deck ever finds a custom card; index its id; no tags; never a token. A game holds its custom cards' definitions as it holds a fused or crafted card's ([[R77]], [[R882]]), so the board, the inspect overlay and every reader see them as they see any card.
- **Its version** counts its saves, starting at 1. A game carries the version it was dealt with.

### 12.2 The frame

The frame is checked by the engine's validator, the same function the server, the practice worker and the engine's game setup call; a frame out of its limits is refused, never clamped ([[R915001]]):

- **Name**: 1 to `CUSTOM_NAME_MAX_LENGTH` (40) characters, not only spaces.
- **Type**: a Unit or a Spell. A Unit has an attack from 0 to `CUSTOM_ATTACK_MAX` (12) and a health from 1 to `CUSTOM_HEALTH_MAX` (12); a Spell has neither.
- **Cost**: 0 to `CUSTOM_COST_MAX` (10) mana, a plain number (no X, no embiggen).
- **Rarity**: one a card prints: Common, Rare, Epic, Legendary or Mythic.
- **Keywords** (a Unit only): at most `CUSTOM_KEYWORDS_MAX` (4), each once, from the game's own list as the kit offers it: Taunt, Rush, Charge, First Strike, Poisonous, Lifesteal, Reborn, Divine Shield, Trample, Windfury, Ranged and Flying.
- **Text**: 0 to `CUSTOM_TEXT_MAX_LENGTH` (240) characters. It may write a number as `{key}`, which the card shows as that number's value on the face it is on.
- **Numbers**: at most `CUSTOM_PARAMS_MAX` (6), each a key of 1 to 16 lowercase letters, digits and underscores starting with a letter, unique on the card, and its base and Radiant values, each 0 to `CUSTOM_PARAM_MAX` (20). Every `{key}` in the text names one of them. They are the card's params ([[§5.1]]), so Degrade, Upgrade and KY's Constant move them as they move any card's, better upward.
- **Radiant** ([[§5.2]]): a Radiant Unit's attack and health are the frame's Radiant stats, each from 0 (health 1) to `CUSTOM_RADIANT_STAT_MAX` (24), twice the base stats when the frame gives none; its numbers read their Radiant values. The keywords and text are the base face's.
- **Art** is procedural, drawn from the card's id and frame as an official card's is; no image is uploaded.
- **Script**: its source, 0 to `CUSTOM_SOURCE_MAX_BYTES` (8,192) bytes of card-kit Luau; an empty script is a vanilla card. The builder's own model of the card, which only the creator reads, is at most `CUSTOM_BUILDER_MAX_BYTES` (16,384) bytes of JSON.

### 12.3 The card kit

A custom card's script returns `Card { … }`, a table of the card's hooks, each a function of `c`, the card's view of the moment, and `n`, its numbers ([[R915002]]). The kit is the only Luau API a custom card sees: the engine's raw verbs (`J`, `require`) are not in reach of its script.

- **Hooks**: `cry` (a Unit's Cry, or what a Spell does when cast), `death`, `startOfTurn` and `endOfTurn` (its controller's), `aura` (stat and keyword changes while the card is on the field, as data) and `activate` (an Activate ability its controller may use once a turn). A card may also declare `target`, one card or hero chosen as it is played, for its `cry`: `c:target()` reads it, as an official card's declared target is chosen with the play ([[§10.5]]).
- **Statements**: a hook does things by calling `c`'s verbs, `c:damage(target, n)`, `c:heal`, `c:draw`, `c:discard`, `c:destroy`, `c:buff`, `c:give` (a keyword), `c:summon` (a token), `c:bounce`, `c:freeze` ([[R1174000]]), `c:armor`, `c:mana`, each recorded as an effect, in order; the hook's effects then resolve as an official card's do.
- **Targets in card words**: "an enemy unit", "a friendly unit", "a unit", "a unit or hero", "an enemy", "your hero", "the enemy hero", "each enemy unit", "each friendly unit", "each unit", "all enemies", "a random enemy unit", "this unit". A word naming one card the player chooses is asked for when the script asks (below); "a random …" is the match rng's pick when the effect resolves.
- **Numbers by name**: `n.dmg` is the card's number `dmg`, on the face it is on and after any tuning; a script never needs Radiant code when only its numbers differ. `c.radiant` says which face runs. A name the card's frame does not declare stops the hook with an error naming it.
- **Conditions and counting**: `c:count("friendly units")` and its kin, `c:hand_size()`, `c:health_of(…)`, `c:mana()`, read the board; `c:each(words, function(card) … end)` runs a statement for each card the words name.
- **Errors** read in plain words and name the line.

The authors' reference, `docs/custom-cards.md`, lists every word.

### 12.4 Choices mid-script

A hook may ask its controller to choose, `c:target(words)` for one card or hero and `c:choose({ … })` for one of several options, wherever the script calls it ([[R915003]]). The engine keeps a paused game as plain data, never a suspended script ([[§10.6]]), so the kit runs the hook again from the top when the answer comes:

- The first run records its effects up to the question, which resolve; then the question is asked as a prompt.
- On the answer the hook runs again with the answers so far: every board reading it made before the question answers as it did the first time, so it walks the same path, and the effects already resolved are not made again. The run goes on past the question with the answer, to the next question or the hook's end.
- A target word with nothing to choose answers nothing: the statements that use it do nothing, as an official card's target with no legal choice fizzles ([[§10.6]]).

### 12.5 When a script fails

A custom card's hook runs under the sandbox, the integer boundary and the memory cap every Luau script ran under in v0.4.0, and an instruction budget of `CUSTOM_HOOK_INTERRUPTS` interrupts ([[R915004]]). A hook that errors, runs out of its budget or memory, or returns what the kit refuses does nothing: none of its effects resolve, an aura gives nothing, and the game emits `scriptFailed` naming the card and the reason, which the board shows. The game goes on; the card is otherwise unchanged.

### 12.6 Where a custom card plays

- **Practice only** ([[R915005]]). A deck that holds a custom card is a practice deck: it plays against the AI in practice ([[§9.9]]) and nowhere else. It is judged by L2, L3 and L6 ([[§9.4]]) as any deck, a custom card counting as one card of it, and each custom card it names must be one of the game's custom cards.
- **Saving a deck** that names a custom card is allowed: decks are drafts ([[R250]]), so D3 also accepts a well-formed custom id, which need not name a card the player still has.
- **Online**: every queue, room, rematch and ranked path refuses a deck or trio that names a custom card, with its own reason, **L7**, "a deck with a custom card plays only in practice", before any other queue rule.
- **The AI's deck** never holds a custom card.
- **The sandbox** ([[R962000]]). A practice sandbox game ([[§9.9]]) whose setup carries custom cards may spawn each of them as it spawns a shipped card, and no custom card it was not given.

### 12.7 Replays

A game with custom cards carries them: `createGame` takes the game's custom cards, each with its frame, version and script source, and the state keeps them, so `hash_state` covers them and `fold(seed, decks, custom cards, log)` folds the same game anywhere the Luau module runs ([[R915006]]). A practice game kept to resume ([[R668]]) or to replay ([[R768]]) keeps its custom cards with it, so editing a card later never changes a game already played.

### 12.8 The AI against custom cards

The AI decides from what its seat may know ([[R185]]) and assumes nothing about cards it has never seen ([[R915007]]): a hidden card is resampled from the official cards, as before, and a custom card's definition is part of its view only once the card has been seen. It plays against a custom card through the engine like any card.

### 12.9 Saved on the account

An active account's custom cards are stored by the server in a table only their owner can read or write ([[R915008]]): at most `CUSTOM_CARDS_PER_ACCOUNT` (50), each saved whole by the routes under `/api/custom-cards` (list, get, create, update, delete). The server checks the frame with the engine's validator and the source's size, and never compiles or runs a script; the creator checks a script with the Luau module before it saves it. A deleted card leaves the decks that name it, which then cannot start a game until it is replaced.

### 12.10 Practice with custom cards

A practice game whose human deck holds a custom card runs in the Luau module, loaded only then ([[R915009]]). It records no card statistics ([[§9.11]]), no ladder progress and no tutorial progress. A normal game, practice or online, loads no Luau.

### 12.11 The creator

A signed-in player makes, edits and deletes their custom cards in the card creator, at `/creator` ([[R915010]]). It edits a card's frame in a form and its script one of two ways, which are two views of the same card:

- **The builder** writes the script from blocks picked from menus, all in the kit's words: a target chosen as the card is played, then triggers (a Cry, a Death, the start or end of the player's turn), each with effects and an optional condition, then for a Unit an aura and an Activate. Each amount a block names is one of the card's numbers, and the builder writes the card's text from the blocks until the player writes their own. The builder's model is saved beside the card, and only the creator reads it.
- **The code view** shows the same script in the kit, highlighted and editable, with each problem the Luau module's check finds shown on its line.

A card whose script is still the one its model writes opens in the builder. A card written in code, or whose script was changed in the code view, is code-only: the builder says why it cannot show it and offers to start the card over. The frame's issues come from the engine's validator as the player types, the script's from the Luau module, and the creator saves no card whose script has a problem; the server's own refusals are shown as it sends them. A card is saved to the account ([[§12.9]]), so it is the same on every device the player signs in on, and "Try it" starts a practice game against the AI with a random deck holding the card ([[§12.10]]).
