# Audit: which Claude model plans, builds, reviews and advises, for medium and hard items

2026-11-09. Asked: is it worth switching the night bot to Sonnet 5.5 with an Opus advisor, staying on
raw Opus, using Opus over a Sonnet builder, or putting Haiku 5.5 under Sonnet and Opus, for medium and
hard items, with every call at `xhigh`?

**Short answer:** keep Opus for hard, and Sonnet to build medium. Three changes shipped with this note
(section 4): the own review's reviewer is Opus on every Claude lane, a hard fullsend part may be built on
a Claude account's Sonnet seat, and Sonnet's effort is split across the accounts as a test. The advisor,
a plan-check call, Haiku and a medium-plan test are not built, and section 5 says why. Every change of
model stays a new agent on the same lane: nothing here switches a model inside a session.

## Sources

- `bot-usage` at `ec379293`: `runs-2026-10.csv` and `runs-2026-11.csv` (4,134 runs, 10-06 to 11-09).
- `bot-state` (`state.json`, `plans/`) and `bot-journal` (1,005 issue files, 6,077 step rows), which
  give each step's model, minutes and cost, and the PR-to-issue map the CSV lacks (without it 722
  revise and review rows were filed under PR numbers and never joined to their issue).
- Anthropic: the advisor-strategy article and the advisor-tool docs, the multi-agent research post,
  Building Effective Agents, and the Claude Code docs for subagents, model configuration (effort),
  best practices, the advisor and pricing.
- **Not read:** the third-party posts and papers on planner/executor splits and reviewer bias (arXiv,
  Blackbox, Cursor, Qodo); this session's network could not reach them. Nothing below rests on them.

"Claude USD" is the Claude CLI's API-price figure, not a bill. Difficulty is known for 1,941 of the 4,134
runs (from the plans, the state and the journal's issue files); the rest are left out of the tables.
Items counted are the ones with an approved run, so a failed item drops out.

## 1. What each step costs

Medians from the journal's steps (n in brackets):

| Step | Medium, Sonnet | Medium, Opus | Hard, Opus |
|---|---|---|---|
| plan | $1.60 · 8 min (73) | $4.96 · 13 min (43) | $8.87 · 18 min (136) |
| build | $1.11 · 23 min (90) | $4.64 · 52 min (6) | $8.29 · 38 min (171) |
| revise | $0.65 · 13 min (52) | $1.57 · 20 min (17) | $2.54 · 25 min (172) |
| review | $0.41 · 3 min (121) | $0.97 · 3 min (138) | $1.34 · 4 min (383) |

Per token Opus 5.5 is only 2x Sonnet 5.5 ($4/$20 against $2/$10 a million); the 4x on a build is
mostly more tokens and minutes, from six Opus builds of unlike size. Do not read it as a price ratio.

Where the money goes (steps with a known difficulty and a cost): **medium** is plan 42%, build 24%,
review 24%, revise 9%, fix 2%; **hard** is build 40%, plan 26%, revise 16.5%, review 15%, fix 3%.

## 2. Whole items

| | Items | Runs | Revises | No rework | Median $ | Model min | To first approval |
|---|--:|--:|--:|--:|--:|--:|--:|
| Medium, Sonnet builds, Opus reviews | 255 | 2.7 | 0.24 | 82% | $5.0 | 40 | 1.8 h |
| Medium, Muse | 39 | 4.3 | 0.90 | 69% | flat plan | 163 | 3.5 h |
| Medium, Muse then Claude in turns | 95 | 6.4 | 2.05 | 15% | $7.4 | 169 | 6.8 h |
| Hard, Opus in every seat | 102 | 7.0 | 1.76 | 40% | $34 | 162 | 8.1 h |

- Of 356 Sonnet-built medium items, 8 (2%) later needed Opus to build or revise, 35 are unapproved so
  far (some in flight), and 16 had a blocked or failed run.
- Cap exposure: Opus-seat runs were cut off 26.6% of the time (24.5% of 1,268 model-hours), Sonnet
  with an Opus reviewer 17.8% (19.9% of 274 hours). 68% of hard items hit a cut-off.
- Review verdicts: Opus approved 76% of 143 medium reviews and 77% of 405 hard ones, Sonnet 74% of 127
  medium ones. Equal rates say nothing about equal catches.

## 3. Findings

1. **The after-merge spot-check works now.** All 13 checks recorded as `failed` fall between 11-03 and
   11-05 05:13, with the reason "was closed"; #3133 fixed the probe. Since 11-05 09:21 all 21 checks ran:
   16 clean and 5 with blocking findings (24%): 3 of 12 medium and 2 of 9 hard. Small numbers, but the only
   independent measure of self-approved merges, and Opus-on-Opus on hard did no better than medium.
2. **Medium's own review was Sonnet's since 11-03 (#2903).** Its approval shipped the item, and the
   spot-check above is the only Opus look at most of them. Opus reviews cost about $0.56 more each.
3. **#2903 changed plan, build and review together, so Sonnet planning cannot be judged alone.** Sonnet-
   planned medium items (51, all after 11-03) took 4.2 runs and 0.47 revises, against 2.3 and 0.15 for
   Opus-planned ones (179, all before it): 73% against 87% with no rework, $3.5 against $5.6. The own
   review is a run of its own, so it adds to the run count by itself, and conflict churn rose in the same
   week (the 11-05 audit). There is no overlap window to compare in.
4. **The usage CSV mislabels plan runs.** Its `models` column says `opus` for all 986 Claude plan runs;
   the journal's steps show 84 of them on Sonnet (a plan of #2925 cost $1.13 in 4 minutes). Not
   investigated; the per-step figures above use the journal.
5. **Outside evidence.** Anthropic's advisor article reports +2.7 points on SWE-bench Multilingual and
   -11.9% cost a task for Sonnet with an Opus advisor against Sonnet alone, on 4.6 models and arms that
   differed in thinking and system prompt, and publishes no Opus-alone arm. Its research post had Opus
   lead Sonnet workers (+90.2% on a research eval, about 15x a chat's tokens) and says most coding tasks
   "involve fewer truly parallelizable tasks". The best-practices page says a fresh reviewer helps
   because it has not seen the reasoning, "not a smarter answer". The docs make `medium` the default
   effort of Sonnet 5.5 and Opus 5.5 and say nothing for `xhigh` as a default. No source says Haiku
   cannot build; its only numbers are on browsing tasks.

## 4. What shipped

All in this branch; the bots cannot make these (`bot/` and `.harness/` are forbidden to them).

1. **Own review: Opus reviews, the builder's model fixes** (`plan.ownReviewer`, `RunReviewer`). Same
   subscription, same lane, same run; every call a fresh session, none resumed. It reverses #2903's
   "one model" for the reviewer only; the fixer is still the model that built. Muse, agy and Codex have
   one seat and are unchanged; a hard item was Opus already; Squishy runs no own review run.
2. **A hard fullsend part may be built on a Claude account's Sonnet seat** (`plan.BuilderSeat`,
   `plan/hold.go`), after Muse and the other medium seats, from an Opus plan; the reconcile (Opus) checks it,
   and a part it sends back goes to Opus on a Claude account (`plan.SentBackSeat`).
3. **Sonnet at `high` on claude-1, -5, -6, -8 and `xhigh` on claude-2, -3, -4, -7**
   (`.harness/providers.json`), balanced by Sonnet runs since 11-03 (about 103 and 114). See "The
   Sonnet effort test" in `bot/README.md` for what to compare and when to stop.
4. Tests: five updated to the new seats and efforts (own review approved by Opus, fixed on Sonnet; a hard
   part's seat on Claude; a build's effort read from its seat), two added (a Claude account lands a hard part on Sonnet; a sent-back hard part goes to Opus
   on Claude and to Muse on Muse).

## 5. Not built, and why

- **The advisor (`--advisor`).** It runs inside one session and sends the whole transcript to Opus at each
  call; the harness sets `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`, which may keep it off (untried);
  advice from Opus 5.5 comes back encrypted, so the journal could not hold it. Its best use, a look at the
  plan before the build commits, is a separate Opus call in the lane (a plan-check), which does not exist.
- **A medium-plan test (Opus, Sonnet, Sonnet and a plan-check).** The plan's model is tied to the build's
  by one lane, one model (#2903); testing it alone needs a per-account plan seat and the plan-check.
- **Haiku as builder.** No coding evidence, and on medium items build is 24% of the spend, so at most
  a fifth could be saved before any rework. The weak builder before #2129 committed conflict markers
  to the spec files again and again, and a weak seat takes no work since #2748.
- **A live orchestrator.** A subagent handed the same task "doesn't make the answer any better"; the plan,
  build and review phases already run as separate calls.

## What to do next

1. Let the effort test run to about 100 medium items a side, then set both sides to the winner.
2. Watch the spot-check's findings by who approved: Opus-approved medium items against the earlier Sonnet-
   approved ones (3 of 12 findings since 11-05).
3. Fix the CSV's plan `models` column (finding 4) so the next audit need not use the journal.
4. If Sonnet-built hard parts send back more than Muse's did, put the part floor back
   (`config.PartFloor` and `BuilderSeat`).
5. Decide on a plan-check call; it needs a prompt, a verdict shape and a step in `work`.

## Not measured

- Whether Sonnet can build a whole hard item: it never has, and the parts trial reaches only fullsend trees.
- Any effect of effort: no run varied it before this change.
- Reviewer catch rates by model: approval rates are equal, and nothing pairs two reviewers on one change.
- The cost of Fable as a reviewer, the only Claude above Opus.
