# Audit: Muse against the Claude accounts, and whether to buy a second Muse plan

2026-11-05. Asked: were the night bot's changes since the last audits efficient, what is still
outstanding, and is another Muse plan worth buying?

**Short answer:** Muse is the cheapest builder the bot has and its biggest single one: about a
third of the bot's merged pull requests, for $50 a month. It is now **bound by its plan, not its
lanes**: it spends its week by day 3 to 5 and is refused for the rest. That reverses the 10-27
audit's "leave Muse's plan as it is". A second plan would put the AWS boxes back to work on the
days they now stand dark. But the bot already merges about as fast as work is queued, so the gain
is mostly lower latency and spare Claude for hard items, not double the output. **Before buying,
fix the after-merge spot-check (finding 3).** It has failed all 12 times it ran, and it is the only
Opus check on Muse's self-approved merges.

## Sources

- `bot-usage` at `980cec46` (2026-11-05 03:56Z): `runs-2026-10.csv` and `runs-2026-11.csv` (3,361
  runs from 10-06), `readings-*`, `ticks-2026-11.csv` and `pools-2026-11.csv` (from 11-03 05:30Z),
  and `weeks.csv`.
- `bot-state` at `85c360fd` (`state.json`: `providers.muse`, `items`, `spot_checks`,
  `machine_memory`), plus 8 older `bot-state` commits for Muse's weekly meter from 10-27 to 11-02.
- `bot-journal` at `bf7d3efe` (887 files): each run's steps, what they were for, and conflicts.
- `main` since 2026-10-20 (squash subjects, to tell which pull requests merged), and closed pull
  requests from the API, to match each `bot/issue-<n>` branch to its pull request.
- #148 (status, drawn 11-05 04:10Z) and #2890 (usage, drawn 11-05 03:56Z).
- `bot/go/internal/cli/cli.go`, `plan/spotcheck.go`, `runner/runner.go`, `gh/methods.go`,
  `state/state.go`, `.harness/providers.json`, `bot/machine/README.md`, `training/README.md`.
- **Not read:** Meta's account page, EC2, or the Claude plans' prices. This session has no access to
  them.

A "merged item" below is an issue whose pull request number is in `state.json`'s `items` and whose
squash commit is on `main`. That covers 228 of the bot's 409 merges since 10-26: `items` drops an
item a while after it merges, so the rest cannot be matched. Revisions and reviews are filed under
the pull request's number and mapped back to the issue's. "Claude USD" is the Claude CLI's
API-price figure, not a bill.

## 1. Muse runs out of its plan by midweek

Muse's weekly meter, from `bot-state` (week of 10-26) and `readings-2026-11.csv` (week of 11-02):

| Week (Mon 00:00Z) | Readings | Reached 100% | Refused until | Days refused | Model hours |
|---|---|---|---|--:|--:|
| 10-26 | 6% Tue 21:46 · 14% Wed 05:02 · 30% Wed 17:41 · 51% Thu 05:46 · 67% Thu 17:54 · 88% Fri 05:56 · 99% Fri 15:20 | Fri 10-30 16:10 | 11-02 00:00 | 3.3 | 277.9 |
| 11-02 | 28% Mon 17:20 · 43% Tue 04:05 · 80% Tue 22:07 · 98% Wed 11:01 | Wed 11-04 14:01 | 11-09 00:00 | 4.4 | 143.1 |

- At full load Muse burns about **1.4 to 1.6 points an hour**, so the plan lasts about 62 to 70
  hours of flat-out running: 2.6 to 2.9 days of the week.
- The 10-27 audit read 6% after the week's first 46 hours and predicted 30 to 40% by the week's
  end. That prediction was wrong. Muse hit 100% three days later. The meter seems to under-read
  early in a week, as it did in the week of 10-12 (#682's open question). Either way, the rate has
  matched across two weeks running.
- Nothing outside the bot spends the plan now: `improve` and `unban` are stopped (#1677), and no
  box runs `gather` since 11-04 (`training/README.md`).
- Since 11-03 05:30Z, Muse spent 64% of its time `working`, 29% `capped` and 6% `idle`. Its 5-hour
  window never binds: 0 of 8 sessions reached it (#2890).

**While Muse is refused, the boxes go dark.** gpt and agy are capped as well (agy until 11-06,
gpt until 11-08), and Muse fills their lanes when it can, so nothing else runs on the boxes:

| | Muse working (11-03 05:30 to 11-04 14:01) | Muse refused (11-04 14:01 to 11-05 03:56) |
|---|--:|--:|
| AWS slots in use (of 6), time-weighted | 3.32 | **0.02** |
| GitHub slots in use (of 8) | 5.75 | 5.12 |
| Pool rows with GitHub's slots full | 3% | 27% |
| Queue length, mean | 0.8 | 2.0 |

**And merges drop.** Bot squash commits on `main` per window:

| Window | Muse | Bot merges a day |
|---|---|--:|
| Mon 10-26 06:00 to Fri 10-30 16:10 | working | 46 |
| Fri 10-30 16:10 to Mon 11-02 00:00 | refused (a weekend) | 29 |
| Mon 11-02 00:00 to Wed 11-04 14:01 | working | 47 |
| Tue 11-03 14:01 to Wed 11-04 04:00 | working (the same hours, a day earlier) | 51 |
| Wed 11-04 14:01 to Thu 11-05 04:00 | refused | 21 |

The Claude accounts do not take up the slack. While Muse was refused, the Claude accounts held
5.1 runs on average (4.7 before) and still sat `idle` 22% of the time (19% before). Two things
hold them back: GitHub's 8 slots, which they share, and the medium plans now waiting on "a medium or
strong planner" (#148's queue has 5).

## 2. What Muse delivers, against the Claude accounts

**Share.** Muse built 72 of the 228 matched merged items since 10-26 (32%), and 41 of 107 since
11-02 (38%). #2890 counts 83 approved items built by Muse in 14 days. That is the most of any
subscription (claude-3 is next, with 57).

**Each merged item, since 10-26, by who built it:**

| | Built by Claude (n=153) | Built by Muse (n=72) |
|---|--:|--:|
| Runs after the plan, mean | 2.76 | 4.17 |
| Revisions, mean (of them by Claude) | 0.82 (0.71) | 1.46 (0.64) |
| Claude model minutes, median / mean | 52 / 93 | **4 / 29** |
| Claude USD, median / mean | $4.97 / $10.47 | **$1.78 / $3.44** |
| Muse model minutes, median / mean | 0 / 11 | 155 / 214 |
| First build to merge, median | 1.5 h | 3.7 h |

**The same difficulty (medium):**

| Medium items, merged | n | Runs, mean | Claude min, median | Claude USD, median | Muse min, median |
|---|--:|--:|--:|--:|--:|
| Built by Claude (mostly Sonnet) | 47 | 1.9 | 25 | $1.64 | 0 |
| Built by Muse | 26 | 4.0 | 2 | $0.83 | 159 |

- **Muse is slow, but cheap in Claude.** A medium item takes Muse about six times the minutes, twice
  the runs and over twice the time to merge. It saves only about 25 Claude-minutes against Sonnet.
  Its real value is the **six AWS lanes**: only Muse, agy and gpt can run there, beside GitHub's 8.
- **Its time this week** (journal steps): build 58%, its own review 22%, merge conflicts 8%, sent
  back 6%, CI 4%. The week before: build 42%, review 21%, **conflicts 24%** (66 h).
- **Runs that ended without delivering** (since 10-26, by model time): Claude 23% (133 h of 588:
  5-hour cap 49 h, refusal 36 h, time budget 17 h, failed 11 h, blocked 11 h). Muse 15% (62 h of
  421: time budget 36 h, halt 11 h, blocked 10 h). A cut-off keeps its work, so not all of this is
  lost.

## 3. The after-merge spot-check has never run (bug)

`spot_checks` in `state.json` lists 12 checks, from 11-03 10:28Z to 11-05 02:02Z. **All 12 ended
`failed`**, each after 3 tries, with the reason `stopped: #<pr> was closed`. The journal shows each
try ending at its second step (`stop | #2909 was closed`, three runs on claude-8, #2888's file).
These are the 36 `stopped` Claude review runs since 10-26.

The cause: `MakeProbe` (`bot/go/internal/cli/cli.go:544`) stops any run whose thread is closed
("Deliver drops a closed thread's work anyway"). A spot-check's thread is the merged pull request
(`plan/spotcheck.go`: "a review run of a pull request that has merged"), and a merged pull request
is closed. So every spot-check stops before Opus reads anything.

Why it matters: 17 of the 41 Muse-built items merged since 11-02 had no Claude run at all. Muse
built them, approved them in its own review and shipped them. When #682 last measured it, Muse's
own review approved 37 of 40, while Opus blocked 15 of the 28 it saw. The spot-check is the only
check on that output, and it is not working. The wasted runs cost little, since each stops at
once. The real cost is the missing check.

**Fix (small):** in `MakeProbe`, skip the closed check when the run is a spot-check, or when the
pull request merged. Add a test with a merged pull request that carries a spot-check. Until then,
nothing the dashboard says about "spot-checked" merges is true.

## 4. Still outstanding from the 10-27 audit

| 10-27 item | Now |
|---|---|
| 3. Muse's refusal read as infra | **Fixed.** `runner/runner.go` matches `quota exhausted` and `rate_limit_error`. Both refusals since (10-30, 11-04) parked Muse until its reset (`refused_until`). |
| 5. A state file over 1 MiB wiped `bot-state` | **Fixed.** `gh/methods.go` reads the blob, and `state.go` raises `StateUnreadable`. `state.json` is 1,239,105 bytes now and intact. |
| 6. Runners killed mid-job by apt | **Fixed** (`b2cef2ca`). |
| 4. The boxes are 8 GB but run three jobs each | **Open.** `MemTotal` still reads 8,153,968,640 on every runner. #148: both boxes "short of memory" over 24 h, least available 1.8 GB, 11 of 14 `b2` runs stalled 5 s or more (longest 166 s), all 14 swapped. It needs AWS's Paid plan (`bot/machine/README.md`). |
| Rec. 4: leave Muse's plan as it is | **Outdated** (finding 1). |
| Rec. 5: compare Meta's account page with `/usage` | **Open.** The early-week under-read (finding 1) makes it worth doing once. |

**Also open:**

- **A new conflict hotspot.** #2150 (10-28) ended the `spec/11-rulings.md` and `spec/INDEX.md`
  conflicts (370 conflicting-file counts in the journal before it, none after). The golden-file conflicts ended after 10-31. Since
  11-02 there have been 80 conflict merges, mostly in the effects registry:
  `apps/web/src/routes/dev/fx.test.tsx` (17), `fx/motifs/gallery.ts` (16), `fx/cards/bespoke.ts`
  (15), `apps/web/README.md` (11) and `fx/constants*.ts` (19). Every card-animation item appends to
  the same lists. It is the same shape as the rulings file, and the same fix would apply: one file
  per entry, folded or globbed.
- **The Claude side's own tuning** (#2890 section 15). Raise `start_headroom.five_hour` from 5% to
  10% on seven accounts: on each, 14 to 27 of the runs that started inside it were cut off within
  10 minutes. claude-3 has no cap, so it runs into Anthropic's 100% instead: 32 refusal cut-offs since
  10-29. A cap near 95% would turn those into clean stops.

## 5. Buying a second Muse plan

**What it would buy.** On this rate, a second $50 plan is another 2.6 to 2.9 days of full Muse
running a week. It would run on the six box lanes that now sit dark for 3.3 to 4.4 days (finding
1). While Muse works the bot merges 46 to 51 a day; while it is refused, 21 to 29. The upper
bound is therefore about 20 more merges a day on the days the second plan covers.

**What limits that.** The bot is close to matched with its supply. Over the last 7 days it answered
301 requests (`Queued …`) and merged 295 pull requests (#148), and the queue held 1 to 2 items on
average. A second plan pays off fully only if the queue stays fed. Otherwise its value is that
medium work stops waiting midweek, and Claude's 5-hour sessions go to hard items. Seven of the
eight accounts reached their cap in 56 to 91% of their sessions (#2890).

**What it takes.** A second Meta account or a larger tier. Check that Meta's terms allow a second
account. A larger tier, if Meta sells one, needs no change to the harness. A second account fits
the box design, which is one unix user per subscription (`bot/machine/README.md`):

1. Make an `agent-muse-2` user on each box and run `sudo -iu agent-muse-2 muse login`.
2. Register its runners with `register-runners.sh`.
3. Add a `muse-2` provider to `.harness/providers.json`, with its own `runs_on`, `lanes` and
   `fill_in`, and a place in `priority`.

`machine_parallel` (6) still holds the boxes' jobs together, so the two plans would mostly take
turns, not double up. A person must make this change, since the bots never edit `.harness/` or
`bot/`.

**The free alternative is pacing.** At about 0.45 points an hour per running job, one lane lasts
the week with about 25% to spare, and two lanes cap around day 4 to 5. #2890 already suggests
`lanes` 4 → 1. Pacing gets the same Muse work spread over the week, so the boxes are never fully
dark, but it adds no work. Pacing a second plan too would keep about 2.5 lanes busy all week.

**Verdict.** For $50 a month, yes, once the spot-check works and if Meta allows a second account
or offers a larger tier. On this data it is the cheapest capacity the bot can add: the alternatives
are more GitHub runner slots, which the org's 20 jobs cap, or a Claude account. If you do not buy
it, at least pace Muse (lanes 4 → 2 or 1), so the medium lanes do not go dark from Wednesday
afternoon to Sunday night.

## What to do, in order

1. **Fix the spot-check probe** (finding 3). It is a small change in `bot/go/internal/cli/cli.go`
   with a test, and needs a person, since the bots never edit `bot/`.
2. **Decide on pacing:** set Muse's `lanes` to 2, or to 1 if no second plan is coming.
3. **Buy the second plan, or a larger tier,** if Meta's terms allow it. Then set it up as section 5
   says.
4. **Resize the boxes** (AWS's Paid plan) or drop to two jobs a box. It matters more with a second
   plan, which makes the box lanes the limit again.
5. **Break up the effects registry's shared lists** (section 4) the way #2150 broke up the
   rulings.
6. **Once,** check Meta's account page against `/usage`.

## Not measured

- Meta's figures for the plan, or whether it sells a tier above $50.
- What the Claude plans cost, so cost per merge across subscriptions is given in Claude minutes
  and API-price USD, not dollars billed.
- The quality of Muse's self-approved merges after they land. That is what the spot-check would
  measure (finding 3).
- 181 of the 409 bot merges since 10-26, whose items have left `state.json`. The shares in
  section 2 come from the 228 that matched.
- Whether the items differ in size between builders beyond the difficulty label. Many items carry
  none, so the like-for-like table covers only those rated medium.
