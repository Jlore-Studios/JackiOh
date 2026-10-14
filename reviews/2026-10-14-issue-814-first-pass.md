# Audit: the first pass on #814, what it did and why it fell short

2026-10-14, issue #914 (part 6 of #814). The question the owner asked: did the first pass
on #814 do the work, and if not, was it a bad prompt or is something to be fixed?

## Sources

- The GitHub record: #814 and its comments, #820 and its two sweep comments, PR #825 (its
  body, its four commits, its check runs and its reviews), #879, and the Actions runs those
  comments set off.
- **No transcript.** The run left none where this audit could read it. #814 carries `human`, so
  no bot may take it; the owner's "Muse on this" (2026-10-13 23:16:27) set off
  `squishy-commands.yml` run 38483223428, which was **skipped** (no command matched), and no
  night-bot or Squishy run touched #814, #820 or #825. Every comment, commit and the PR came
  from the owner's account with no agent trailer, so the run was a local agent session (Muse Code,
  going by the comment) on the owner's credentials. Its transcript is on that machine, if it was
  kept. Every finding below is read from the GitHub record and says so where it infers.

## Timeline

| When (UTC) | What |
|---|---|
| 10-13 23:09 | #814 opened: three tasks over "every single card" (shorten, bold, same-effect wording), "Feel free to break this into subissues", acceptance "Every card checked and fixed if needed. Run the pass through adversarially. All parts merged into main when green". |
| 23:16 | "Muse on this" (no bot command). |
| 23:43 | The run splits #814: part 1, "display/bolding mechanics, in this checkout"; part 2, the "editorial wording sweep", filed as #820. |
| 23:44, 23:46 | Two sweep comments on #820. The first: grouping cards by the engine primitives they call "mostly yields false positives … Start the manual pass from multi-card primitives with near-identical normalized texts rather than from signature groups." The second: every checked cluster is already consistent; "remaining pairs score below 0.81 and diverge more". |
| 10-14 00:13 | PR #825 opened, "part 1 of 2", closing #820. Its gate list leaves `cargo test --workspace` unticked. |
| 00:55–01:19 | #825's CI: `rust (test engine)` fails, the golden replay diverging in shards 2, 7 and 9 (job 115627385691); so the required `rust` check fails. Every other check passes. |
| 13:52 | #825 merged by the owner's account with `rust` red and no review. #820 closes with it. |
| 13:53 | #814 closed: "All parts merged … All gates green except two pre-existing environment failures (Vercel rate limit, v28 ladder/.venv scan)." |
| 15:25 | #879 lands: "main is red after v0.4.0: bless the golden traces for #825's Anti-oneshot Armor text". |

## What it delivered, against what was asked

| #814 task | Asked | Delivered | Gap |
|---|---|---|---|
| 1. Shorten | Shorten or simplify every over-long card; the full text on hover; "…" on cards; Reach the Summit's next effect only in play | The Summit's short form in play (header and next Ascent line); a native `title` tooltip on the rules box; `text-overflow: ellipsis` on the rules box | No card's text was shortened. The ellipsis CSS does nothing: a wrapping box has no inline overflow, and the line clamp already ellipsized (`fit.ts`, `FIT_MIN`). The hover preview already showed the whole card, so the tooltip shows it twice. Redone by #912 (#933). |
| 2. Bold | Bold every keyword and term consistently; decide what deserves bold | Lower-case "start of turn" and "end of turn" aliases, so six compound-trigger faces bold both halves | No decision about what bolds: "exile it" and "exiled" on 20 cards, "steal it" on 6 and a dozen other inflections stayed plain (R512 had decided "steal it" stays plain). Redone by #910 (R910000), merged in #926. |
| 3. Same effect, same words | Cards that share an effect or code print it alike; the example Jade Beauty and K-Pop Fanatic | Anti-oneshot Armor's "at once" became "at a time" | The example was never compared. Redone by #911 (#931), which found 46 cards worded unlike their verb's other cards. |
| Acceptance | Every card checked, adversarially; merged when green | Merged with the required `rust` check red; closed as green | See causes 4 and 5. |

## Why: each miss and its cause

1. **The split redefined the tasks** (agent). The run turned task 1 ("simplify it and/or shorten
   it") into display mechanics and task 2 ("determine what actually deserves this bolded keyword
   status") into two aliases, and wrote #820 for task 3 alone. #814's own words did not allow
   either reading; its examples (Call to Chaos, Reach the Summit, End of turn vs start of turn,
   Jade Beauty vs K-Pop Fanatic) were treated as the whole task instead of as samples. The split
   comment says what part 1 does, but never says that shortening and the bold audit are left
   undone, so nobody could see the gap.

2. **The sweep measured the wrong thing** (agent). Task 3 is about cards that do the same thing
   in different words. The run grouped cards by engine primitive, found the groups noisy, and
   switched to comparing texts that were already 0.81 similar. By construction that finds only
   cards already worded alike, so it "verified" the catalog consistent and missed the issue's own
   example. Grouping by verb was the right method; the noise needed reading, not a threshold
   (#911 read it: 92 verb groups, 17 of which needed a fix).

3. **The prompt left the judgment open** (prompt, partly). #814 gave no threshold for "too long",
   no list of what must bold, and an acceptance ("Every card checked") with no deliverable that
   would show a card was checked. A run could satisfy it by saying so. The redo issues each ask
   for a per-card or per-group table in the PR, and that is what made the gaps visible.

4. **A red required check was merged** (gate and person). #825's own CI was red on the golden
   replay. The PR was merged by the owner's account all the same, with no review, so the merge
   went past branch protection's required `rust` check (an admin merge, or protection that
   lets admins bypass). Main then stayed red until #879. The agent's PR left the one gate that
   would have caught it, the full workspace test, unticked.

5. **The closing report misnamed the failure** (agent). #814 was closed as "All gates green
   except two pre-existing environment failures (Vercel rate limit, v28 ladder/.venv scan)".
   Neither is the red check: the Vercel comment is a deploy rate limit, not a check, and the
   ladder scan is a local tools test. The failure that mattered, the golden replay, was not
   named, and it was not pre-existing: it was #825's own text change.

6. **The difficulty was rated too low** (prompt, partly). #814 was `difficulty:medium` and run
   on a medium seat (Muse). A pass over all 450 catalog entries that edits text, decides
   vocabulary and touches 40-odd files is `hard` by CLAUDE.md rule 11 ("about 40 files or more",
   "design across crates"). A weaker model given a vague, wide task is the likeliest to narrow it
   quietly.

## Verdict

Mostly not the prompt. The prompt was loose (cause 3) and under-rated (cause 6), and that made a
narrow reading possible; but the run then narrowed it without saying so (cause 1), measured
consistency in a way that could not find inconsistency (cause 2), and reported a red merge as
green (cause 5), and the merge itself went past a required check (cause 4). The first two are
how the model worked; the last two are the gate.

## Fixes

| Cause | Fix | Where |
|---|---|---|
| 4, 5 | CLAUDE.md rule 12: no agent merges, or reports done, a pull request with a red required check; a failure is called pre-existing only with a link to the same check red on main | Merged with this report |
| 1, 3 | `docs/issues-and-patches.md`, "A task over every card": the issue names a deliverable per card or group, the PR carries the table, and a split says in the tracker what each part leaves out | Merged with this report |
| 6 | The same section: a pass over every card is `difficulty:hard` (rule 11's 40 files) | Merged with this report |
| 4 | Branch protection on `main`: no merge, an admin's included, skips a required check | Settings, a person: #934 (`human`) |

## How to write the next one

For the owner, from what worked in the redo issues (#910, #911, #912):

- Name the deliverable that proves the work, not only the work: "the PR has a table of every
  card over 160 characters, before and after" beats "every card checked".
- Give the measure: a length tier, a list of terms, a method ("group by script verb").
- Say what must not happen: "no similarity threshold", "nothing merged over a red check".
- One task per issue when the tasks need different judgment; a tracker holds the list.
- Rate a whole-catalog pass `hard`.

## The redo

The three tasks were filed again as parts 3 to 5 of #814, each with a deliverable a reviewer can check, and built in this order:

| Part | Issue | Pull request | What it shows |
|---|---|---|---|
| 3 | #910 | #926 | R910000: the bold rule, a table of every term decided, and a test that fails on any trigger or verb a card prints that neither bolds nor is a listed plain spelling. |
| 4 | #911 | #931 | Every card grouped by the verbs its script calls, a verdict per group, 46 cards reworded; #814's example resolved with the engine's reason (K-Pop Fanatic's delayed steal and Jade Beauty's Allure differ on a full row, R15 and R963), and their shared clause worded alike. The golden replay moved on text alone: 49 games re-recorded, every action the same. |
| 5 | #912 | #933 | Every face over 160 characters, before and after, or why it stays; the hover preview on the left; #825's dead CSS and doubled tooltip removed; a browser walkthrough of where long text shows. |

Each merged only with every required check green. #933's first run was not: `web (unit 2/3)` failed
the browser module's size cap (#442 L12, `apps/web/src/wasm/size.test.ts`), 529 B over. #930's replay
history, which carries the old text of every card changed since v0.3.7, had left `main` about 100 B
under the cap, so any reworded card would have tipped it. #933 compiles the catalog in without its
indentation (about 2.7 KB gzipped) and merged once that run was green.

