# Writing for players

The client's voice guide. Every word the client shows a player follows it (buttons, headings,
labels, tooltips, accessible names, notices, errors, the coach, the log), in one voice: a friendly
host at the table. `src/writing.test.ts` checks what a machine can.

It covers the client's own words. The cards' text, the engine's and the server's sentences (shown
as they come, only translated), the legal pages (`src/routes/privacy.tsx`, `terms.tsx`,
`copyright.tsx`) and a crash's technical detail under a plain heading (kept for a bug report) are
not the client's to reword.

## Warm

- Talk to the player: "you" and "your". JackiOh is "we": "We couldn't send an email just now."
- Never blame. Say what happened, then what to do: "That code didn't work. Check the newest email
  we sent, or ask for a new code."
- An exclamation mark only for a cheer: "Quest complete!"

## Plain

- The words a player uses at the table. A word that needs a developer or a statistician to explain
  it gives way to what it means. No rule letters (Rule L4), sections (§2.1) or ruling numbers.

| Not this | This |
|---|---|
| standard error | the margin: about how far luck alone can move a rate |
| sample, sample size, artifact | games, "too few games" |
| Δ, delta | vs ("Played vs held") |
| Elo | rating |
| drill-down | details |
| SFX | sound effects |
| seed, replay, "the server's" | what it means: "we couldn't check how it was played" |
| Failed to load … | … didn't load |

## Short

- One idea to a sentence. A button is a verb and what it acts on: "Copy code".
- Say a thing once: "Your decks: 1 of 3 have won a game", not "1 of 3 decks have won".

## Consistent

- Sentence case: "Find a match", "Patch notes". Capitals only on the first word, on names (JackiOh,
  Jlore, a card, a set, a mode, a screen such as the Almanac) and on the game's terms as the cards
  print them (Unit, Spell, Trap, Tribute, Radiant). `src/wording.test.ts` holds the game's words.
- A full stop ends a sentence; a button, a heading or a label has none.
- The straight apostrophe: can't, you're, as the cards, the engine and the server print it.
- Curly double quotes around a name or a phrase the player typed or will see: Delete “Aggro”?
- One character for the ellipsis, for something under way: "Loading your decks…".
- — for an aside, – for a range or a score (A–Z, You 1 – 1 Opponent), · between parts of a dense line.
- When something fails, say what didn't happen and what to do: "The card statistics didn't load."
- American spelling: color, behavior, center, favorite, canceled, labeled.

## Translations

Every sentence has an entry in `src/i18n/zh-CN.json`. Reword the English and you rename its key,
reword the Chinese as plainly, and drop the old key.
