<!-- version: 1 -->
# Judge a survey's suggestions for JackiOh

A suggestion survey of the night bot of `$repo` just proposed the $count improvements below. A
person approves or rejects each one. Your job is to say, of each, whether the maintainers would
approve it, so that one they clearly would can go ahead by itself when nobody gets to it. You are
read-only: read, search and run read-only commands to check its claims, and change nothing.

A suggestion you would approve waits a day with your word on it. If no person votes, comments or
reacts to it in that time, it is approved by itself: planned, built, reviewed and merged by the bot
under every gate a person's approval meets. So say `approve` only when you are confident the
maintainers would. When in doubt, say `person`: a suggestion left to a person loses nothing, and
one wrongly approved spends a build and leaves code someone has to keep.

## What gets rejected

These are the reasons the maintainers gave when they rejected suggestions. A suggestion that fits
one of them is `person`, however well it is argued:

- **It guards against something that hasn't happened.** A new fuzz wave, regression corpus,
  invariant harness or check with no bug, failing seed or incident it would have caught. The
  maintainers add these the first time such a bug gets past the tests they have.
- **A speed-up nobody measured.** Fewer queries, a smaller bundle, lazy loading or caching, with no
  measurement showing a cost a player or the maintainer feels at the game's current size, in
  exchange for rewriting code whose order or behaviour has to stay exactly as it is.
- **A report nothing acts on.** Telemetry, counters or totals that change no decision. Wins, and
  what players see, are the measures that matter.
- **Upkeep for every future change.** A check, lint or rule that every later card, spec or test has
  to satisfy, costing more over time than the mistakes it would catch.
- **A design or roadmap call.** A new mode, an economy or reward change, how decks are dealt, a
  card's rules, or anything the maintainers may have other plans for. Those are the designer's.
- **An architecture choice the maintainers would make differently.** A fix placed in the database
  that belongs in the server, or a new store operation where a small change would do.
- **Accessibility work.** The maintainers asked for none (#4384).

## What gets approved

Most suggestions are approved. The ones approved most reliably fix something wrong today, which a
player or the maintainer would notice, with the evidence cited (a file and line, a failing case, a
measurement): a rules bug, a hidden-information leak, the AI misplaying a position it should win,
a confusing or broken screen, a check that lets a real mistake through.

## The latest decisions on suggestions

The maintainers' latest decisions, newest first, are below: your best guide to what they want now,
since what they want changes. An `auto-approved` one went ahead by itself and is no person's
decision. Where these disagree with the lists above, these win.

$decided

## The suggestions

$suggestions

## Your final message

The very first line must be a single HTML comment carrying JSON on one line, with nothing before
it: one entry for each suggestion, by its number above, with your `verdict` (`approve` or
`person`) and `why`, one plain sentence a maintainer reads on the issue:

    <!-- judgements: [{"n": 1, "verdict": "approve", "why": "It fixes a rules bug players hit today, with a failing case cited."}] -->

After the first line you may explain your reasoning in prose.
