# Audit: is Muse working, at 6% of its week?

2026-10-27, asked because Muse's weekly meter read 6% on the status issue. The question: is Muse
idle, or does Meta's $50 plan simply hold far more than the night bot spends?

**Short answer:** Muse is working, harder than any other subscription. The 6% is real for the
night bot's own use: Muse is held back by its lanes and boxes, not by its plan. The audit found
four problems along the way. Two are bugs in the harness and two are gaps between the config and
the machines.

## Sources

- `bot-state` at `428ffcba` (2026-10-27 22:18Z): `providers.muse` (usage and the last 49 calls),
  `machine_memory` (the last 60 machine runs), and Muse's usage reading in every `bot-state` commit
  since 10-13 (21,892 commits, sampled every three hours, plus every change around 10-22 and 10-25).
- `bot-journal` at `373b3d7e`: 2,125 run sections, 301 of them Muse's since 10-12.
- The status issue (#148) as it was at 2026-10-27 22:25Z.
- Actions: run 40180350668 (job 121129888791, its log), 40548624605 and 40593640511.
- The tree at `5e130b1` (`main`): `bot/harness/runner.py`, `state.py`, `gh.py`, `memory.py`,
  `.harness/providers.json`, `bot/machine/README.md`, and the two earlier audits
  (`2026-10-19-issue-383-muse.md`, `2026-10-22-issue-682-subscriptions.md`).
- **Not read:** EC2 or Meta's account page. This session has no AWS or Meta access.

Muse's week runs from Monday 00:00 UTC. "This week" is 10-26 00:00Z to 10-27 22:15Z, about 46 hours.

## 1. Muse is working, and works more than anyone

| This week | Runs | Model hours | Outcomes |
|---|---|---|---|
| `muse` | 50 | **58.7** | 43 approved, 5 planned, 1 built, 1 infra |
| `claude-3` | 61 | 32.6 | |
| `claude-8` | 42 | 21.7 | |
| `claude-4` | 26 | 20.2 | |
| `claude-7` | 37 | 18.2 | |
| `claude-6` | 27 | 17.8 | |

- The status issue's last 7 days give Muse 48 of 48 outcomes delivered, 23 pull requests opened,
  15 merged, 0 failed and 0 that could not run. Its only infra run this week is #1867 on 10-26.
- At 22:25Z it held all six of its lanes: four of its own, plus gpt's and agy's, which it fills
  while those two sit capped (gpt at 97% of its week, agy at 99%).
- Its runs hold a lane longer than a Claude account's. A Muse build or revision this week took a
  median 54 model minutes and a mean of 80. The Claude accounts took a median of 33 to 43. The items
  differ, but finding 4 below slows Muse's runs too.
- Ten medium builds were queued at 22:25Z, each waiting on "a medium or strong model", with Muse
  listed as busy. Muse is short of lanes, not of work.

How busy its lanes were (journal intervals, which leave out each job's setup):

| Period | Muse's lanes then | Share of minutes with 0 / 1 / 2 / 3 / 4+ Muse runs | Mean |
|---|---|---|---|
| 10-20 00:00 to 10-22 20:48 | 2 | 39 / 25 / 36 / 0 / 0 % | 0.97 |
| 10-26 05:00 to 10-27 14:30 | 2, plus fill-in from 10-26 22:56 | 16 / 34 / 47 / 2 / 1 % | 1.37 |
| 10-27 14:30 to 22:15 (#2069) | 4, plus fill-in | 0 / 12 / 16 / 42 / 30 % | 2.9 |

## 2. Why 6%: the plan is big, and last week's 100% came from the training lanes

Muse's weekly meter, compared with the model hours the night bot recorded:

| Week | Night bot's Muse hours | Meter | Percent per model hour |
|---|---|---|---|
| 10-12 to 10-18 | 75.1 | 9% by 10-15, then a mid-week drop to 0 and 4% by 10-18 | about 0.17 |
| 10-19 to 10-22 20:48 | about 60 | **1% to 100%** | about 1.6 |
| 10-26 to 10-27 22:15 | 58.7 | 6% | about 0.10 |

The 10-19 week is the outlier, and #682's audit already found the cause. On 10-19 #1276 moved the
training box's `improve` and `unban` lanes onto Muse, on the same Meta plan and outside the harness.
They spent about 117 Muse hours between 10-19 and 10-22, which the bot never recorded. The meter
started climbing about an hour a percent from 10-19 22:00Z and hit 100% at 10-22 20:48Z. The night
bot then stood without Muse until the reset at 10-26 00:00Z, 3.1 days of 7. #1677 stopped both
lanes on 10-25, and this week's rate matches the first week's again.

**What that means.** On this week's rate, Muse running all six lanes flat out until 11-02 would end
the week somewhere around 30 to 40%. The plan has room for far more than the two boxes can run.

**The caveat is unchanged from #682's audit.** Even with the training lanes counted, the 10-19 week
burned the meter about three times faster per hour than this week does. Either the training
sessions cost far more per hour than night-bot runs, or the meter was off in one of those weeks. Only
Meta's account page, checked against the `/usage` panel, can settle that. If the meter is wrong,
Muse could cap without warning, and finding 3 means the bot would not recognise the cap when it
happened.

## 3. The bot does not recognise Muse's refusal (bug)

On 10-25, while Muse was really capped, the bot sent it six runs (#1817 twice, #1831 twice, #1826,
#1908). Each ended at once and was filed as **infra**. The job log (121129888791) gives the real
message, which the journal cut short:

```
agent loop failed: model failed: API error 429 [request_id=…]: Subscription quota exhausted.
Your usage window resets at 2026-10-26T00:00:00Z. (rate_limit_error)
```

Checked against `bot/harness/runner.py` on `main`:

- `RATE_LIMIT_WORDS` does not match it. It looks for "rate limit" with a space, "quota exceeded",
  "exhausted your quota" and "quota reached". It does not look for `rate_limit_error`, "quota
  exhausted" or `429`.
- `RESETS_IN` reads "resets in 3h", not "resets at <ISO time>", so `park_for` would fall back to an
  hour anyway.
- `MuseCli.run` sets `infra_hint` when `muse` exits 2. `RunResult.infra` is then true, and
  `_with_usage` returns early on an infra result. So the "a failed call whose reading has a window
  full is that window's refusal" fallback never ran.

In a Python shell on this tree, that message gives `RATE_LIMIT_WORDS.search(...)` = no match,
`park_for(...)` = `+PT60M`, and `RunResult(False, "", 2, error=msg, infra_hint=True)` has
`infra=True, rate_limited=False`. The 10-19 audit predicted this ("Muse's words for a refusal are
not known… the first sign will be a failed call the bot does not recognise as a refusal"). The
words are known now. #1907 ("`muse` could not work 3 times in a row") opened on 10-25 and closed
itself on 10-26 when Muse worked again. Nothing changed in the code.

**Cost:** six claims, six lanes taken for a few minutes each, and hand-offs. #1817 went to gpt,
which spent 11.7 minutes and changed nothing. The bigger risk is later: a cap the meter misses
reads as a broken machine, not as a wait until the reset.

**Fix (small, `bot/harness/runner.py`):** add `rate_limit_error|quota exhausted` to
`RATE_LIMIT_WORDS`, and let `_resets_in_minutes` or `park_for` also read "resets at <ISO time>".
Make sure a rate-limit match wins over `infra_hint`, either in `MuseCli.run` or in
`RunResult.infra`. Add one test with the message above.

## 4. The boxes are still 8 GB, but run three jobs each (config gap)

`.harness/providers.json` (Muse's note), `bot/machine/README.md` and #2069 (merged 10-27 14:28Z)
all say both boxes became `m7i-flex.xlarge` (16 GB) on 10-26, and #2069 runs three jobs a box. Every
memory reading the boxes have taken since 10-25 11:45Z, the latest at 22:04Z today, reports
`MemTotal` of 8,153,964,544 to 8,153,976,832 bytes. That is 7.6 GiB, an `m7i-flex.large`. The night
box and the `b2` box both read the same.

The status issue's memory lines agree. Both boxes are "short of memory" over the last 24 hours:
least available 2.2 GB and 2.5 GB of 7.6 GB, 16 and 12 runs that swapped, and stalls of 5 s or
more in 4 and 5 runs. The night box's longest stall was **766 s**. Single readings this afternoon
show 18 million pages swapped on the night box (20:10Z to 20:22Z) and 5.1 million on `b2` (17:52Z).
The 10-19 audit found that two building jobs already overfill 7.6 GB. #2069 now allows three a box.

The likely cause is the account's plan. The project is on AWS's Free plan (`bot/machine/README.md`,
the credits watch), which "refused instance types outside the Free Tier" when the training box was
built. Its largest m7i type is `m7i-flex.large`, and `m7i-flex.xlarge` is not on the list
(AWS's EC2 Free Tier page). So the type change ("stop each, change its type, start it again",
`bot/machine/README.md`, Moving the boxes to three jobs each) most likely never took. A person with
AWS access should check both instances' type. Until they really are `xlarge`, three jobs a box makes
Muse's runs slower than they need to be. Upgrading to the Paid plan lifts the limit, and AWS's
Free Tier FAQ says the credits left keep applying to the bills until they expire.

## 5. A state write wiped `bot-state` on 10-25 (bug, every subscription)

`bot-state` commit `3de4fa42` ("delivering run 40177864479", 10-25 09:12Z) cut `state.json` from
1,051,203 bytes to 362. That emptied `items`, `handled`, `providers` (every subscription's usage
readings and spent calls), `machine_memory`, `statistics` and the rest. That is why the bot forgot
Muse was capped and sent it the six runs in finding 3. The status issue's statistics also count
only Muse's "last 49 runs" because of it.

The cause: `state.json` had just grown past 1 MiB. For a file over 1 MB, GitHub's contents API
returns the metadata with an empty `content`. `GitHub.get_file` decodes that to `""`, and
`State.update` runs `normalise(_loads(""))`. `_loads` turns the `ValueError` into `{}`, and
`update` writes the near-empty state back. `State.load` takes the same path. On every other day
since 10-20 the file sat between 640 and 850 KB. It is 564 KB tonight and growing, so the next spike
past 1 MiB will wipe it again.

**Fix (`bot/harness/gh.py`, `state.py`):** when the contents API returns `encoding: "none"` or an
empty `content` with a non-zero `size`, read the blob (`git/blobs/{sha}`, which serves up to 100 MB)
or the raw media type instead. Separately, `update` should refuse to write when the file existed
but could not be parsed, not start again from `default_state()`. A test with a 1 MB file the fake
returns without content would hold it.

## 6. Runs "died" when a box's runner was stopped mid-job (added 2026-10-28)

"The run working on this died … the model job left no result" (#2155) came 10 times from
2026-10-12 to 10-28 (`died #…` commits on `bot-state`), out of about 1,800 claims. Three were
claude-7 on GitHub's runners, whose `Install the claude CLI` step failed. The other seven were on
the boxes, and every one of those ended the same way: the work step was **cancelled** partway
through, its log saying "The runner has received a shutdown signal", with Muse, cargo and rustc
still running. Six were Muse's, one agy's:

| When (UTC) | Runner | What stopped it |
|---|---|---|
| 10-14 13:45 | `night-vm-muse-2` | not found |
| 10-22 07:00, twice in one second | `night-vm-muse-3`, `-4` | Ubuntu's unattended upgrades (06:00 to 07:00) |
| 10-24 06:20 | `night-vm-agy` | the same window |
| 10-26 23:00 | `night-vm-muse` | runners re-registered four minutes after #2028 merged |
| 10-28 03:11, 03:13 | `night-vm-muse-fill`, `night-vm-muse-3-b2` | `setup.sh` installing zram (finding 4), one per box |

On Ubuntu 24.04, needrestart restarts every service whose libraries an apt run replaced, at the end
of the run, whether interactive or not. The two on 10-28 came from this audit's own zram change:
its apt install ran needrestart, with its output sent to `/dev/null`. The 06:00 to 07:00 deaths fit
the daily unattended upgrade. The box's own logs (`/var/log/unattended-upgrades/`, the runners'
journal) were not read here and would confirm it. Each death throws away the run's work: #2155
lost two hours of Muse, #2115 an hour and twenty minutes.

**Fix (on this branch):** `setup.sh` writes `/etc/needrestart/conf.d/night-vm.conf` before its
first apt run. It sets `override_rc` to 0 for `actions.runner.*` and `jackioh-train@*`, so no apt
run restarts a runner or a training lane; they take new libraries at their next restart.
Re-registering runners while one holds a job still kills it, which `bot/machine/README.md` now says.

## What to do

1. **Fix finding 5 first.** It will happen again, to every subscription, and it caused the wasted
   runs in finding 3. Done on this branch: `get_file` reads a file over 1 MB from its blob, and
   `StateStore` raises `StateUnreadable` before it reads or writes over a state that is not JSON.
2. **Fix finding 3,** so a real cap parks Muse until its reset instead of reading as infra. Done on
   this branch: `RATE_LIMIT_WORDS` knows Muse's words, `park_for` reads a reset given as a time, and
   a refusal is never infra.
3. **Check the instance types (finding 4).** If they are 8 GB, either resize them as the README
   says, or set the box runners back to two jobs each until someone does.
4. **Leave Muse's plan as it is.** On this week's rate it cannot reach its cap from the boxes
   alone. More Muse needs more lanes: real 16 GB boxes, or Muse on GitHub's runners from a
   `MUSE_AUTH` secret (the 10-19 audit's recommendation, never tried). GitHub's runners were full at
   22:25Z (20 of 20, 46 jobs queued), so that is not free either.
5. **Once,** compare Meta's account page with the `/usage` panel, to settle which week's meter rate
   is true (#682's open question).

## Not measured

- Meta's own figures for the plan. The `/usage` panel is the only reading.
- The EC2 instance types, read directly. Finding 4 comes from the boxes' own `/proc/meminfo`.
- Whether this week's mix of items is lighter than earlier weeks'. That could also explain a lower
  meter rate per hour.
