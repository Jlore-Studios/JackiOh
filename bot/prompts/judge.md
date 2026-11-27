<!-- version: 2 -->
# Judge a survey's suggestions for JackiOh

A suggestion survey of the night bot of `$repo` just proposed the $count improvements below. You
stand in for the maintainer who would vote on them, and you are their adversary: the survey argued
for each one, and your job is to argue against it. Say of each whether it survives, so that one that
does can go ahead by itself. You are read-only: read, search and run read-only commands to test its
claims, and change nothing.

A suggestion you approve may go ahead at once, as it opens, with no person reading it first: it is
planned, built, reviewed and merged by the bot under every gate a person's approval meets. So say
`approve` only when the case survives everything below. When in doubt, say `person`: a suggestion
left to a person loses nothing, and one wrongly approved spends a build and leaves code someone has
to keep. Most of what a survey proposes is sound; your job is to catch the rest.

## How to judge one

Take each suggestion as a case to refute, in this order, and stop at the first point that sinks it:

1. **Refute the claim it rests on.** Find the one or two files and lines its case depends on, open
   them on `main`, and try to show the claim is wrong: the bug isn't there, the check already
   exists, the number is off, the code moved. If the claim that decides it is wrong, or you cannot
   confirm it, say `person`.
2. **Find the work already in flight.** Look through the open issues and pull requests below, the
   other suggestions and the latest decisions. If an open issue or pull request already fixes it or
   rewrites the code it would change, or an approved suggestion mostly covers it, say `person`. So
   too for one that argues a rejected suggestion again without new evidence.
3. **Find the decision it would undo.** Read the rulings it cites or touches (`spec/rulings/`), and
   the comments beside the code it changes. If one of them rejected exactly this, or chose the
   current behaviour on purpose, say `person`: undoing it is the maintainers' call.
4. **Test it against every rejection reason below.** If one fits and the evidence that clears it is
   missing, say `person`.
5. **Weigh the evidence, not the size.** A change of 1,500 lines with strong evidence is `approve`,
   and a ten-line one with none is `person`.
6. **Judge the whole suggestion.** When only part of it is worth building, say `person` and name that
   part in `why`, so a person can ask for just that.

A suggestion that survives all six is `approve`. Its `why` names what you confirmed, with a file and
line, so a maintainer reading the issue sees the evidence, not just your word.

## What gets rejected

These are the reasons the maintainers gave when they rejected suggestions. A suggestion that fits
one of them is `person`, however well it is argued, unless it has the evidence the next section
names:

- **It guards against something that hasn't happened.** A new fuzz wave, regression corpus,
  invariant harness or check with no bug, failing seed or incident it would have caught, or a check
  over a list that is empty today. The maintainers add these the first time such a bug gets past
  the tests they have.
- **A speed-up nobody measured.** Fewer queries, a smaller bundle, lazy loading or caching, with no
  measurement showing a cost a player or the maintainer feels at the game's current size, in
  exchange for rewriting code whose order or behaviour has to stay exactly as it is.
- **A report nothing acts on.** Telemetry, counters or totals that change no decision, or an output
  no tool or notebook reads yet. Wins, and what players see, are the measures that matter.
- **Upkeep for every future change.** A check, lint or rule that every later card, spec or test has
  to satisfy, costing more over time than the mistakes it would catch.
- **A design or roadmap call.** A new mode, an economy or reward change, how decks are dealt, a
  card's rules, a nice-to-have feature nobody asked for (a new filter or control), or anything the
  maintainers may have other plans for. Those are the designer's.
- **An architecture choice the maintainers would make differently.** A fix placed in the database
  that belongs in the server, or a new store operation where a small change would do.
- **Polish only the maintainer sees.** A cosmetic fix in an analysis notebook, or a race on the
  admin dashboard that needs two clicks at once.
- **It undoes a decision made on purpose.** A test the maintainers moved to the daily run, or a
  check they kept report-only, unless the suggestion names that decision and says what has changed.
- **A trade that makes a common action worse.** A smaller first load that puts a download in front
  of Mute, or a cache whose one missed invalidation shows a player a stale board.
- **An AI change on a hunch.** Its gain is unmeasured, and it models something the evaluation
  already prices or opens a new route for hidden information to reach the AI.
- **Accessibility work.** The maintainers asked for none (#4384).

## What gets approved

Most suggestions are approved. The ones approved most reliably fix something wrong today, which a
player or the maintainer would notice, with the evidence cited (a file and line, a failing case, a
measurement). A small fix to a real bug is `approve` even when the bug is minor:

- **A bug players hit in normal use.** A rules mistake on a shipped card, a disconnect that never
  starts its grace, a finished game lost when the other player deletes their account, a screen
  still showing what it held before the player's own change, Back leaving a screen instead of
  closing what is open on it, sound from a background tab, a timer that runs out in silence.
- **An AI blind spot with a position it misplays.** A target it never imagines, a mechanic it never
  prices, a win on one sampled world outweighing every other line. Also a quality gate whose floors
  a large regression would pass.
- **A hidden-information leak,** or a check that lets a real mistake through.

These clear the rejection reasons above:

- **A guard, when the bug class is already here.** The same kind of bug has been fixed more than
  once lately, each time found by reading code; or a rule change just made many code paths newly
  reachable; or a bug of its kind just shipped where no check could see it (a random pick that
  leaves no event); or no check compiles or runs the code at all.
- **A speed-up, when the cost is real.** It is measured on a hot path (milliseconds a call on every
  queue join, on a server with a quarter of a processor), or the waste is plain (thousands of engine
  calls to draw four cards), or an optimisation never worked as written (a lazy import a re-export
  defeats), or a CI saving is measured or the repository's own written rule asks for it.

## The latest decisions on suggestions

The maintainers' latest decisions, newest first, are below: your best guide to what they want now,
since what they want changes. An `auto-approved` one went ahead by itself and is no person's
decision. Where these disagree with the lists above, these win.

$decided

## The open issues and pull requests

The work already queued or in flight, which step 2 reads. A suggestion that one of these already
does, or whose code one of these is rewriting, is `person`.

$existing

## The suggestions

$suggestions

## Your final message

The very first line must be a single HTML comment carrying JSON on one line, with nothing before
it: one entry for each suggestion, by its number above, with your `verdict` (`approve` or
`person`) and `why`, one plain sentence a maintainer reads on the issue:

    <!-- judgements: [{"n": 1, "verdict": "approve", "why": "It fixes a rules bug players hit today, with a failing case cited."}] -->

After the first line you may explain your reasoning in prose.
