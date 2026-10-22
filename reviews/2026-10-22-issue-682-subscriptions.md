# Audit: the subscriptions again, a week on (#682)

2026-10-22, for #682. Was Muse's $50 plan worth it? Does Codex earn its place? What does Devin
still do before 2026-11-02? The audit then asks two wider questions: is this the best way to use
Muse, and how can the AI keep getting better for little money?

## Sources and window

- **The week:** Muse's first full week on the $50 plan, 2026-10-12 00:00 to 10-19 00:00 UTC. That is
  its meter's own week. On 10-11 it was still capped by the $15 plan until midnight, so #682's
  10-11 to 10-18 runs one idle day earlier.
- **The days after:** 10-19 00:00 to 10-22 18:40 UTC, reported apart. On 10-19 Muse got a second
  lane (#1277), and the training lanes moved onto its plan (#1276). What happened next decides
  the recommendation.
- **`bot-journal`** at `b88a0682`: 1,524 run sections, 2026-10-06 to 10-22. They give every step's
  model, minutes and verdict.
- **`bot-state`:** all 15,477 commits since 10-08, for every Muse, Codex, agy and Claude usage
  reading and every dated model-minute and token record. The live file holds only the last 200.
- **Pull requests:** each run's last commit was matched to a pull request head (`refs/pull/*/head`),
  and that pull request to its squash merge on `main` for the lines changed. 253 of the 428
  squash merges since 10-10 matched. The rest are people's, Squishy's or `patches ship`.
- **The status issue (#148)** as of 2026-10-22 18:35 UTC. #190 was folded into it by #754.
- **The training lanes' session comments** on #901 (unban) and #597 and #1373 (improve), and
  `training/history/`.
- **Claude cost** is the API-equivalent cost the Claude CLI reports per call (`tokens_spent`), not
  what the subscriptions cost. It was matched to runs by account and time; 89% of the week's
  $2,582 matched. Muse and Devin report no tokens.

The scripts are not committed. Every figure can be counted again from the two branches.

## The answer

| Subscription | Recommendation |
|---|---|
| **Muse ($50)** | **Keep the $50 plan, for the night bot alone.** On two lanes the night bot uses about 70% of it, about twice what the $15 plan held. Take the training lanes off it now. Since 10-19 they have spent about 64% of its week and promoted nothing. They also left the night bot without Muse from 10-22 11:15 until 10-26 00:00 UTC, which is 3.5 days of 7. |
| **Codex** | **Keep it only if it is on its cheapest plan; otherwise cancel it.** Its quality is good: 3 of 3 merged, 2 of 2 first reviews passed. But on GPT-5.6 Sol its week lasts about 4 to 5 model hours, about 3 merged pull requests. Never put it back on `easy_first`: in the week it built nothing, because there was nothing easy to build. |
| **Devin (to 11-02)** | **Let it end.** It built one item for the night bot in the week. Nothing needs to replace it, because the easy rule leaves about one easy item a week. Until 11-02 it is the better home for any LLM training session, since its plan is already paid. |

**Were the Muse changes worth it, against more traditional work?**

- **The $50 plan:** yes. Muse never stood idle in its first week, against 2.5 days of 7 on the
  $15 plan.
- **The second lane:** yes. Muse merged 46 pull requests in the 3.5 days it ran, at better
  quality.
- **Moving the training lanes onto Muse:** no. They took about 117 Muse hours and produced no
  promotion. They also cost the night bot 3.5 days of its biggest builder: at the two-lane rate of
  about 13 merged pull requests a day, that is about 45 pull requests.
- **The traditional option did better in both senses.** The one fair gain in AI strength since
  10-09 came from the `learn` lane: gen 13 on 10-21, a script with no model in the loop, run on the
  owner's idle PC. Plain building of issues is what Muse is good at.

## 1. Muse

### The week (10-12 to 10-19, one lane)

| | Muse | Sonnet, the same week |
|---|---|---|
| Runs | 144: 56 plans, 58 builds, 29 revisions, 1 review | |
| How they ended | 132 delivered (53 planned, 64 approved, 12 fullsend parts, 2 built, 1 reviewed); 11 blocked; 1 could not run | |
| Model hours | 75.7 | |
| Pull requests it built that merged | 34, +15,780 / −2,287 | 57, +25,544 / −3,874 |
| Median build step | 36 min | 18.5 min |
| Opus's first review approved it | **13 of 28 (46%)**; 11 of 23 medium | **53 of 56 (95%)**, all medium |
| Blocking findings when Opus asked for changes | 1 in 11 of its 15 blocks (others 2, 3, 5, 8) | 1 to 3 |
| Revisions per merged pull request | 0.91: 0.26 merge conflict, 0.38 review, 0.21 CI | 0.35: 0.32 merge conflict, 0.02 CI |
| Its own in-run review approved | 37 of 40 | (Opus reviews Sonnet in the run) |
| Claude spent per merged pull request | **$9.63**, 62 min (Opus reviews and Opus revisions) | **$6.20**, 51 min (plan, build and review) |

- **Usage.** Its 5-hour reading peaked at 5% and its week at 9%. It was never refused and never
  paused on a limit, so it stood idle for usage on 0 days of 7. On the $15 plan it was 2.5.
- **The meter read oddly.** The weekly figure fell to 0% twice mid-week (10-12 19:14 and 10-15
  19:04), while it still showed a reset on 10-19. It also under-read against the week after
  (below).
- **Its quality held, and is far better than before #700.**
  - #593 counted 1.2 CI failures and 2.5 revisions per Muse pull request. This week: 0.21
    revisions to fix CI, and 0.91 revisions in all.
  - Opus still sends back half of it. On the same difficulty, Sonnet passes Opus's first review
    twice as often.
  - Muse's own review remains close to a stamp: it approved 37 of 40, and Opus then blocked 15 of
    the 28 it saw.
- **In this week Muse saved no Claude.** A medium pull request Muse built took more Claude, in
  cost and minutes, than one Sonnet built. Most of that went on Opus reviewing it and then fixing
  what it blocked, since a rejection's revision runs at the reviewer's tier.

### The days after (10-19 00:00 to 10-22 18:40, two lanes)

| | Muse | Sonnet |
|---|---|---|
| Runs, model hours | 100, 65.8 h | |
| Merged | 46, +23,194 / −1,366 | 44, +23,487 / −5,107 |
| Opus's first review approved it | 24 of 34 (71%); 22 of 30 medium | 42 of 44 (95%) |
| Revisions per merged pull request | 0.54 (CI 0.02) | 0.45 (almost all merge conflicts) |
| Claude per merged pull request | **$5.10** | **$8.58** |

Muse got better, and now saves Claude. The status issue's last 7 days (10-15 to 10-22) agree: 69
merged pull requests built by Muse, more than any other subscription (claude-3 has 41).

### The cap, and why

- **The week filled in under three days.** The weekly meter climbed from 1% at 10-19 17:45 UTC to 95% at
  10-22 11:15. A build or a plan needs 5 points of headroom (`start_headroom`), so the night bot has
  started no Muse run since 11:15. It will start none until the reset at 10-26 00:00 UTC.
- **The cause is the training lanes.** Since #1276 the `improve` and `unban` lanes run Muse on the
  training box, on the same plan and outside the harness. None of their time is in the bot's
  records.
- **How much they used.** Their session comments add up to 56.8 hours on improve (#1373) and 59.9
  on unban (#901), against the night bot's 65.8. About 183 Muse hours filled the week, and the
  lanes were 64% of them.
- **They are now failing in a loop.** From 16:35 to 18:37 UTC today, 110 sessions (55 a lane)
  ended after 0 minutes, about one a minute across the two lanes. Each one posted a comment on #901
  or #1373.
  `training/loop.sh` has no backoff after a failed session.
- **How big the $50 plan is.** On these readings, about 190 Muse hours fill a week: about 2.7 times
  the $15 plan's 70 (#593). The first week's 9% for 76 hours does not fit that figure, which
  predicts about 40%. Either the meter under-read that week, or Meta changed it on about 10-19. The
  repository cannot tell which. A person can check Meta's account page against the panel.
- **What the night bot needs.** On two lanes it ran at about 131 Muse hours a week: about 70% of
  the $50 plan, and nearly twice the $15 plan. A cheaper plan would cap the two lanes by mid-week.
  The $50 plan is the right size for the night bot, but not for the night bot plus two
  round-the-clock training sessions.

## 2. Codex (ChatGPT)

- **Weak, with `easy_first` (10-11 to 10-20).**
  - It was refused until 10-16 23:52 UTC by the weekly cap its 10-10 run had reached.
  - From 10-17 to 10-20 it made seven short calls, 44 model minutes in all. They have no journal
    sections; the journals leave out suggestion surveys. They still took its weekly meter from 6%
    to 30%.
  - **It built nothing.** The week rated about 147 items, and one of them was easy: #773, which
    Devin built. `easy_first` had nothing to take, so the experiment measured nothing. A weak,
    easy-only seat is starved by design under the easy rule.
- **Medium, on GPT-5.6 Sol (since 10-20).**
  - It made 10 runs: 5 plans, 4 builds and 1 revision.
  - 3 pull requests merged (+787 / −39). Opus approved both it reviewed on the first try, and
    none needed a revision.
  - Then it reached 96% of its week after about 4.6 model hours, and it is refused until 10-27
    18:45 UTC.
  - In the last 7 days its calls sent 113 million input tokens, with no cached tokens reported
    (#148). The Claude accounts' calls are almost all cache reads. If caching can be turned on or fixed for the
    Codex CLI, it would stretch this allowance more than anything else would.
- **Decision.** Its quality is good, but it adds about 3 pull requests a week. Keep it only if
  that is worth the plan's price, which the repository does not record. Leave it off `easy_first`,
  and spend no more harness work on it.

## 3. Devin, until 11-02

- **In the night bot:** one build in the week, #773 (easy), merged at +80 / −10. Codex built
  nothing beside it.
- **In the training lanes:** it ran at least 236 hours in the week (improve 150, unban at least
  87). Generations 10 to 12 (10-13 and 10-14) came out of that. They were measured before #995 fixed
  the unban gate's deal, which gave the candidate's seat shaped decks and the parent's unshaped
  ones. Replayed with both seats dealt alike, generation 12's 76/100 became 48/100.
- **Can Codex carry the easy items once Devin goes?** There is almost nothing to carry: about one
  easy item a week, and Codex is medium now. Easy items will go to the medium seats, Muse first.

## 4. Is this the best way to run Muse and the night bot?

The changes, most value first:

1. **Take the training lanes off Muse's plan today.**
   - A person stops `jackioh-train@improve` and `jackioh-train@unban` on the training box
     (`sudo systemctl stop …`). Or points `loop.sh` back at Devin until 11-02.
   - That gives the night bot back about two thirds of Muse's week, from the reset on 10-26.
   - Give `loop.sh` a backoff, so that a refused session does not post a comment a minute.
2. **Keep Muse on two lanes and the $50 plan.** A third lane does not fit the 8 GB night box: two
   builds at once already swap (`reviews/2026-10-19-issue-383-muse.md`, finding 1). It would fit on
   GitHub's runners, which that audit recommends trying for a week. That move also runs clippy and
   the tests on every Muse build, which the box skips.
3. **Stop paying a model for mechanical merge conflicts.**
   - In the week, 68 revisions ran only because `main` had moved: 30 model hours, about what agy
     spent in the whole week.
   - 42 of them (18.3 hours) conflicted only in three files: `spec/INDEX.md`, which is generated;
     `spec/11-rulings.md`, whose paragraphs are appended at the end; and
     `crates/engine/tests/golden/games.jsonl`, which is append-only. In the 3.8 days after, it was
     45 of 77.
   - The fix: have the harness's merge step regenerate `INDEX.md` (`cargo jackioh spec index`) and
     take both sides of the two append-only files. Then run the checks, and hand the item to a
     model only if they fail.
   - `bot/` is a forbidden path for both bots, so a person or an interactive session builds it.
4. **Measure the tier order instead of assuming it.**
   - Muse comes before Sonnet for medium items. In the first week that cost Claude more, not less;
     in the days after, it saved Claude.
   - Two numbers would settle it each week: Claude cost per merged pull request by builder, and
     the first strong review's approval rate by builder and difficulty.
   - The journals already record each run's model, cost and verdicts, so the status issue can
     count both. The only thing nothing records is the training box's Muse hours: the meter counts
     them and the bot does not.
5. **Expect a Claude-only bot whenever the plans line up.** Right now every non-Claude seat is
   capped at once: Muse at 95% until 10-26, agy at 100% until 10-23 22:00 UTC, and Codex refused
   until 10-27. claude-2 is at its 90% cap and claude-5 at its 60%. Until agy comes back on Friday
   evening, the bot runs on claude-1, 3, 4, 6 and 7 alone.

## 5. Making the AI better, for little money

### What the lanes did

| Lane | Who ran it | Model hours since 10-09 | Fair promotions |
|---|---|---|---|
| `improve` | Devin, then Muse from 10-19 | about 271 (#597 about 214, #1373 about 57) | **none**; `training/history/improve.jsonl` is empty |
| `unban` | Devin, then Muse from 10-19 | about 156 on #901, plus earlier drafts | gens 1 to 12 passed under the unequal deal (#976, #995). None since. |
| `learn` | a script, `learn.yml`, on the owner's PC; no LLM | none (about 40 PC hours a run) | **gen 13, 10-21**: 220 of 400, exactly the bar (`training/history/learn.jsonl`) |
| `gather` | a script, one core | none | gates nothing |

About 427 LLM hours went into `improve` and `unban`, and the only fair gain came from the script.

The sessions' own notes explain why. Their candidates' dev screens sit within a few points of the
parent, centred near half a point, and they have already tried and dropped:
- knob and weight sweeps;
- a linear evaluator;
- ISMCTS, twice;
- a second opponent reply.

Two things work against the unban lane in particular:
- **The gate fights the goal.** An unbanned card enters the candidate's decks only. The session
  notes measure games holding a released card at about 5 points worse, so the lane must win 60 of
  100 while carrying that handicap.
- **It re-gates until a seed passes.** The one pass after #995 (10-17, session 16) followed real
  gates of 46 and 54 on the same family. The next session called that 60/100 "just the roll", and
  its own six tickets scored 44 to 59. The pass was then set back when `main` moved.

### A realistic setup

1. **Make `learn` the engine.**
   - It is the only lane that has gained, and it costs only the PC's idle hours.
   - Run it twice a week rather than weekly.
   - Give it more self-play data: its fit report's learning curve asks for more than the
     100,000-game export.
   - Wire in the policy prior (`training/ml/policy_fit.py`, `retrain_policy.sh`), which is built
     but switched off, as a second candidate.
2. **Gate with a sequential test, one per family of changes.**
   - Replace the fixed bars (220 of 400 for improve and learn, 60 of 100 unpaired for unban) with an
     SPRT on the paired hold-out deals: edge 0 against +0.06 (about +21 Elo), at a 5% chance of
     either error.
   - That averages about 1,500 unpaired games, fewer when paired, and stops early on a clear loser.
     It reuses `arena --paired` and the lane seeds.
   - Log every gate a lane runs. A family gated again after a small change continues its one test
     rather than starting a fresh chance.
   - Today's 220 of 400, played once, passes a true +21 Elo candidate only about 15% of the time
     (paired). Gated again and again on one family, as the unban lane does, a fixed bar also passes
     equal candidates by chance.
3. **Treat the shadow ban as performance work, not a training lane.**
   - All 17 bans are Meditative cards, banned for `timeout`: a decision over 2 s
     (`AI_SWEEP.decision_ms`) at the browser's budget, in a game the card was forced into
     (`crates/ai/src/shadow_ban.rs`).
   - Open one night-bot issue per card, or per cluster that times out the same way. It is done when
     `cargo jackioh sweep <id>` is clean at both tiers, and a non-inferiority check shows the AI
     with the card is no worse than about −20 Elo.
   - That work goes through the bot's review and costs a build, not an always-on session.
4. **Sweep each set when it ships.** Today a sweep runs only when someone opens an issue for it
   (#732, #1186). A step in `patches-ship.yml`, or in the daily `super.yml`, could sweep the new
   ids and open the `sweep --write` pull request.
5. **Track strength across generations.**
   - Play gens 8, 12 and 13 against `main` with `arena --paired` on the local runner, daily or
     weekly.
   - Fold the results into the Bradley-Terry fit `bot-calibrate` already has. A regression then
     shows up as a number, not a hunch.
6. **Use LLM sessions only as bounded issues.** If a person has a concrete idea for the AI (a
   search change, a misplayed card), it goes through the night bot as an issue with a measured
   acceptance. It does not go into a five-hour loop that runs around the clock.
7. **Retire the training box** once the LLM lanes stop. Only `gather` would be left on it, which
   one core of the PC can run, so that saves about $78 a month (`bot/machine/README.md`).

## Not measured

- **What each plan costs.** The Claude, Codex and Devin prices are not in the repository, so cost
  per pull request is given in Claude's API-equivalent dollars and in model hours.
- **Muse's meter.** The repository cannot say which of its two weeks read true. A person comparing
  Meta's account page with the `/usage` panel would settle it.
- **Difficulty is recorded on pull requests, not issues.** The by-difficulty rows count merged
  pull requests only.
- **Early training drafts.** The unban lane's sessions before #901 (10-14) were not counted, so
  its hours are a floor.

## What came of it (2026-10-22)

- **#1674 (merged):** Muse's and Squishy's weeks have no start headroom, and Squishy's weekly cap is 100%. agy, Codex and the Claude accounts keep the 95% start threshold.
- **#1675:** the 17 shadow bans are swept again under generation 13, and what is left is sorted by cause. One issue per cause follows from it.
- **#1676:** the development card statistics come from the learn lane's exports, in place of `gather`.
- **#1677:** the `improve` and `unban` lanes stop, and the training box becomes a second night box with one Muse build a box. The alternative is Muse on GitHub's runners and the box stopped. Section 5's last step said to retire the box; the owner chose to keep it for the night bot.
- **#1678:** the shadow ban is swept again after each learn promotion, and when a set ships.
- **#1679:** the training gates become a sequential test.
- **#1680:** the ladder bots' seeds are recalibrated for generation 13.
- **#697** holds everything about the local runner. **#1336** is closed: the learned evaluation shipped.
