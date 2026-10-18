# A learned evaluation for the AI (#466): options and a plan

Issue #466 proposes a small neural network, trained in Python on self-play and run in Rust inside
`crates/ai`, to replace or blend with the hand-written evaluation (`crates/ai/src/evaluate.rs`, SPEC
§9.9 "Evaluation"). This note is its planning pass of 2026-10-15. It sets out what changed since
the issue was written, whether the network can beat today's AI, and each design decision with its
options and a recommendation. Then it lists the seven parts, each a sub-issue of #466, and the
calls that are a person's.

Nothing here is built yet. Each part is a sub-issue of #466, #1006 to #1012 (The parts), and
takes the rulings it needs from its own issue's block: #1006 owns R1006000 to R1006999
(`docs/issues-and-patches.md`, Ruling numbers).

## Where things stand

- **The blockers are closed.** #306 closed on 2026-10-08. #443 merged as #501: `analysis/` is a uv
  project on Python 3.13 with every dependency pinned in `uv.lock`, the conventions part 4
  follows.
- **The bars moved.** The issue quotes #306's 85 of 100 against the parent. Since #850 (R850000,
  R850001) the improve lane promotes on **220 of 400 paired games** on 200 hold-out deals, and
  screens on a fixed dev set (`arena --set improve-dev`). No lane plays random since generation 9
  (R732001). An AI that truly wins 55% passes about half the time, and one at 58% nine times in ten.
  A learned evaluation needs a real edge of about 5 points, not the 35 the old bar asked for.
- **The improve lane has never promoted.** `training/history/improve.jsonl` is empty after 63
  Devin sessions (#597). Every promotion up to generation 12 came from the unban lane; generation 9
  was #732's one-off measurement.
- **What the lane has ruled out** (`training/improve.md`, #850, #597's session notes):
  - weight and knob sweeps of the eval and the search (s35, s41);
  - a **linear** evaluator fitted to 19,918 self-play positions (s15). It matched the hand eval
    (AUC 0.749) and added nothing;
  - rollout votes, a second-ply vote and more determinizations (s14);
  - ISMCTS: "measured-failed at any budget the 2s browser limit allows" (s53, s55);
  - three hand-written terms for what a stat line cannot see. These were the engine yield (s60,
    +0.020), turn modifiers (s61, +0.017 on 120 games) and recurring hooks × durability (s63a,
    0.000 on 400 paired games, interval −0.080 to +0.080). The session closed the family: "the
    baseline eval's stat/cost proxies already track script power closely enough".
- **The lane's notes name this issue as the next step.** Session 63: "the fitted nonlinear evaluator
  is blocked on the positions-recording pipeline (tools, outside the lane)". That pipeline is
  part 1 (#1006).
- **The training box is full.** It is one `m7i-flex.large` (2 vCPUs) running three lanes. On
  2026-10-15 a 400-game screen took about 91 minutes at load 6 (s63a). Another (s61) projected about
  5 hours at load 9 to 10 and was killed. The disk filled at 30 GB of `target/`.
- **Free Devin ends 2026-10-16** (#383): after that Devin's CLI draws on the plan's allowance at 6× to
  12× credits.
- **The repository is public.** GitHub's standard hosted runners are free for it.

## Will it beat the current AI?

The current AI is a 600-node beam search (`AI_BUDGET`: 3 determinizations, beam width 4, a lethal
solver, the opponent's reply simulated once). Its leaves are scored by `AI_EVAL`'s 37 terms and 21
keyword values: concave hero health, stats and keywords through §10.4's layers, hand, library,
threat and pressure, and a turn-cap ramp. The weights were tuned by hand and then by the lane.

The hand evaluation sees a card through its stats, its keywords and about six special cases
(Brittle, plague and grade counters, Animated, cost changes, lapsing credit). What it cannot see is
what the card's **text** does over the next turns. The lane's loss autopsies keep finding exactly
that: "midgame snowballs: an engine, a unit that grows or makes value every turn, priced at its
body today and left unanswered" (s36, s44). There are also cards with a measured leak: med-019 is
played-and-won 9.9% of the time over 111 games, med-003 39.8%.

**What a network can add that the lane could not:**

1. **A worth per card, learned from games.** A feature per card id and zone (on the board, in the own
   hand, in the own readable backrow) lets the network learn what each of the 370 cards and 80
   tokens is worth beyond its stats, in context, from outcomes. The lane has tried writing such
   terms by hand three times, and each read zero. A learned per-card worth needs nobody to write the
   right formula for each card. It also retrains when a patch changes a card.
2. **Interactions.** A Taunt's worth depends on the enemy's attackers, a hand's on the mana to play it,
   an engine's on how long it survives. A linear model cannot price a product of terms, which is why
   s15's refit matched the hand eval and nothing more.
3. **Game phase.** The weights the hand eval uses are the same on turn 3 and turn 25 except for the
   `closing` ramp. A network reads the turn as an input.

**What it cannot fix:** the search's horizon (one opponent reply), hidden information (it reads only
`redact`, R185), and the browser's time limit. It is a better leaf, not a deeper search.

**Why it may still fail.** s63a's reading is a real possibility: if stats and cost already price
most of a card's text, a learned per-card worth adds little, as the hand-written ones did. A value
fitted to outcomes is also noisy: one label per game, shared by every position of that game. And a
better AUC need not mean better play. What matters is ranking the beam's sibling lines at one
decision, not telling won games from lost ones (s15 shows that gap).

**So the plan measures playing strength early and cheaply.** #466's first order put the first
game against the parent after quantisation and WASM parity. The parts below play a native screen
on the dev set right after the first fit (part 5, #1010). It stops there if
`edgeLow ≤ 0` on 800 paired games.

**It pays off even if the network does not ship.** The position data ranks where the hand eval is
wrong: positions it scores as won that the seat loses, grouped by card and by turn. That turns the
improve lane's guesses ("a sweep or trace on 087/097/034 would name their mechanism", s61) into a
measured list.

**How it ranks against the other routes to a stronger AI:**

| Route | Status | Note |
|---|---|---|
| Hand-tuned weights and terms | mined out (s35, s41, s51, s60, s63a) | the lane's default activity for 63 sessions |
| Deeper search (ISMCTS) | failed at the browser's budget (s53, s55) | |
| A learned leaf evaluation | this note | needs the data pipeline first |
| A learned move ordering (a policy prior over the root's candidates) | not tried | a follow-up once positions exist: the same data, labelled with the action the search chose |
| A bigger `AI_BUDGET` | a person's call; no lane may raise it | duel `msP95` reads about 690 ms on the loaded box (s63a) against `AI_GATE.max_decision_ms` 1500, but generation 9's bans are all `timeout` |

## How much data and compute

Is 100,000 games enough for a game this complex? Not to learn the game from nothing, and the plan
does not ask that of it.

- **What is learned.** The network does not learn to play. The search, the lethal solver and the
  determinization stay as they are. The network starts as the hand eval (D4's skip connection) and
  learns only a correction to it. That is a far smaller job than an AI from scratch.
- **Two anchors.** AlphaZero learned chess from nothing over 44 million self-play games, played on
  5,000 TPUs: out of reach, and not this job. TD-Gammon reached expert backgammon, a game of chance
  like this one, with a network of about 200 inputs and 40 to 160 hidden units. It trained on
  300,000 to 1.5 million self-play games on early-1990s hardware. That is the scale this plan sits
  at.
- **What counts is games, not positions.** A game's 70 or so positions share one outcome, so
  100,000 games are about 100,000 independent labels. With them:
  - a card is in about 10,000 decks and on the board in a few thousand games;
  - that is enough to fit one worth per card and zone (a win rate over 3,000 games has a standard
    error of about 1 point);
  - it is not enough for a 200,000-weight network, nor for interactions between particular pairs of
    cards.

  So the first network is small: about 50,000 weights, a first layer 16 wide over D5's inputs. It
  grows only as the data does.
- **Measure, do not guess.** Part 4 (#1009) fits on 10,000, 30,000 and 100,000 games and plots the
  held-out loss against the data. If the curve is flat by 100,000, data is not the limit: more
  compute will not help, and the features or the model must change. If it is still falling, export
  more before the screen.
- **Where the compute goes.** Almost all of it goes into playing the games, which is the engine on
  a CPU. Fitting a network this size takes minutes, so a GPU would not speed up the bottleneck.

| Games | vCPU-hours (measured: 9.6 vCPU-seconds a game, wall time) | 20 hosted runners | AWS on demand (about $0.05 a vCPU-hour) |
|---|---|---|---|
| 100,000 | about 270 | under 4 h, one dispatch | about $13 |
| 1 million (TD-Gammon's scale) | about 2,700 | about a day and a half: six 6-hour dispatches | about $135; spot is less |
| 10 million | about 27,000 | about two weeks without a break, past any fair use of free CI | about $1,350; spot is less |

Up to a million games is free on hosted runners or cheap on AWS. Ten million is where a person
would have to pay for compute, and the learning curve says whether it is worth that.

**Better labels before more games.** Offline TD(λ) runs on the logs already stored (D4's D), so
it needs no new games and lowers the labels' noise. Its targets blend the outcome with the network's
own prediction a few positions later. A bigger search as the teacher (D4's C) helps little in this
game. The beam's horizon is the seat's turn and the opponent's reply whatever the budget, so a
6,000-node search sees an engine's later turns no better than a 600-node one.

## The decisions

Each decision lists its options and a recommendation. The ones a person must make are gathered at
the end.

### D1. Where self-play runs

The export is the expensive part. Measured for this note (2026-10-15, `arena --a self --b self
--games 40` on 4 cores): 96 s of wall time and 327 CPU-seconds, so about **8 CPU-seconds a game**
and about 1,500 games an hour on a 4-vCPU machine. A game ran 16 turns on average, with about 36
decisions per seat, so about 70 decision positions a game. Fitting a small network takes minutes on
any CPU.

A useful first dataset is about **100,000 games**. Each card then appears in about 10,000 of them
(40 of 370 cards dealt a game), enough for D5's per-card worth.

| Option | Games an hour | 100,000 games take | Cost | Fits because | Against |
|---|---|---|---|---|---|
| **A. The training box as it is** | under 450 (at most 1 of its 2 cores is spare) | over 9 days | none | already set up | full: it would slow both lanes' screens further (s61, s63a) |
| **B. A bigger AWS box**, or one spot box per export | about 3,000 on 8 vCPUs | about 33 h | about $0.40/h for an `m7i.2xlarge` on demand (about $13 for the export); spot is less | the training lanes live there; a bigger box also speeds their screens (#383) | AWS's Free plan refuses anything above 2 vCPUs, so it needs the paid plan; about $192 of credits as of 10-06; a person's setup |
| **C. GitHub-hosted runners**, a `workflow_dispatch` matrix like `super.yml`'s fuzz shards | about 30,000 on 20 jobs × 4 vCPUs | under 4 h | free for a public repository | the export is embarrassingly parallel and reproducible from seeds; artifacts carry the files; nothing to administer; `ci-duration.yml` does not watch it | a `.github/` change, a person's; one export is bounded by artifact size and retention (90 days at most); GitHub's terms limit Actions to work on the project itself, which training the project's own shipped AI falls under, by our reading |
| **D. A person's own machine**, by hand or as a self-hosted runner (#1008) | about 6,000 on 16 cores | about 17 h | none | fastest to start; as a runner (label `jackioh-selfplay-local`) C's workflow dispatches to it, so its runs are logged and repeatable | its own setup and disk (`target/` grows past 30 GB); safe on this public repository only while "Require approval for all external contributors" stays on, as for the night box (`bot/machine/README.md`) |
| **E. #383's k3s training pool** | on demand | — | about $170 a month for the whole plan | the long-term home if training grows | 10 to 14 days of a person's build work, not started |

**Recommendation: C and D together, through one workflow (#1007), and keep the training box for
the lanes.** A dispatch picks GitHub's runners or the local one (#1008). One dispatched run of
under four hours gives what the box's spare core would take one to three weeks to play, at no cost. Logs are the stored form (D3), so a run's artifact holds a few hundred
MB compressed, not tens of GB. B is the right second step if the lanes later retrain the network on
every patch. Upgrading the box would also cure the lanes' starved screens, which is worth it
whatever #466 decides.

### D2. Whether an LLM supervises it

Today each lane is one Devin session after another on a standing prompt. The lane is good at
diagnosis (its notes found the extra-turn horizon hole and the Expedition leak), but it has not
turned 63 sessions into a promotion. An LLM in the retraining loop is also the step most exposed to
the "fishing" that `improve.md` forbids: retrain, re-gate, keep the lucky read.

| Option | What runs | Against |
|---|---|---|
| **A. No LLM in the loop.** A script: export, fit, write the weights, screen on the dev set against a control, gate only when `edgeLow > 0` | one `training/ml/retrain.sh`, by a person or a scheduled job | someone must think of better features or models; the script cannot |
| **B. The improve lane runs it.** `improve.md` learns the recipe; Devin chooses features and sizes | Devin on the box | Devin's cost after 10-16; the box's CPU; a lane may change `crates/ai/**` only, so it can retrain but never change the exporter or the trainer |
| **C. Both.** The script is the only way a network is made; the lane, or a person's session, proposes changes to features or model shape as screened stack commits, each run through the same script | A for every retrain, B or a person for design | two places to read for what happened |

**Recommendation: A until a network has passed its screens (parts 4 to 6), then C (#1012).** The pipeline is mechanical. A run is
reproducible from its seeds, and the gate's seeds name the committed tree, so a scripted retrain
cannot fish as long as it gates only after a passing dev screen. An LLM earns its cost on *what to
try next*. Put `features.rs` in `crates/ai` and make the exporter and trainer read the feature
length from the data. Then the lane can add a feature, bump `FEATURES_VERSION`, re-featurize from
the stored logs and retrain, all inside the paths it may change.

### D3. What the export stores: features or game logs

The issue plans an exporter that writes features at play time. Every arena game already holds its
whole log (`ArenaOutcome.log`), and a log replays exactly from its seed (`replay.rs`; the catalog
it was played on, R767000). That gives a second option:

| Option | Self-play reruns when the features change | Size | Note |
|---|---|---|---|
| **A. Features at play time** (the issue) | yes, every `FEATURES_VERSION` | about 1 KB per position | simplest reader |
| **B. Logs at play time, features by a replay pass** (`cargo jackioh positions <logs> --out`) | no: re-featurizing is a fold, milliseconds a game | a few KB per game | the arena, the gate and the gather lane can all keep their logs too, so every game anyone plays becomes data |

**Recommendation: B.** Feature design is where the iteration goes (D2), and B makes each try cost
seconds instead of a new export. The replay pass computes `features(redact(state, seat))` at each
decision point of each seat. One `features` function serves the pass and the inference (the
issue's rule), and the R185 test applies to it unchanged.

B's one catch: an engine change that alters how a game plays out stops its older logs replaying
the same way. Each log carries its final hash, so the pass checks the hash and drops a game that
no longer reaches it. A big engine change means a new export, as A would need anyway.

### D4. What the network learns to predict

| Option | Label | For | Against |
|---|---|---|---|
| **A. The game's outcome** (the issue) | 1, ½ or 0 for the seat | free with every game | noisy; one label shared by about 16 turns' positions |
| **B. A, plus the hand eval as an input and a skip connection** | as A; the output is `a·hand + net(x)` | the network starts as the hand eval and learns only what it misses; worst case is no change | none worth the name |
| **C. A deeper search's score** (distillation) | what a 10× budget search scores the position | a label per position, less noisy than outcomes | each label costs a big search; the beam's horizon is the same whatever its budget (How much data and compute), so it teaches little about an engine's later turns |
| **D. Bootstrapped (offline TD(λ))** | outcome blended with the network's own prediction a few positions later | less variance than A; runs on the stored logs with no new games | a few fit-and-relabel passes instead of one fit |

**Recommendation: B first. D is the follow-up if the learning curve says the labels are the limit.**

### D5. The network's shape

The search calls the evaluation about once per node, a few hundred to a thousand times a decision,
so inference must cost microseconds.

| Option | Inputs | Cost per call | Learns per-card worth |
|---|---|---|---|
| **A. A dense MLP over aggregates** (the issue's list: health, armor, unit stats, hand and library sizes, mana, the hand eval's terms) | about 100 to 200 numbers | about 15k multiply-adds | no: a card is its stats |
| **B. A sparse accumulator, chess's NNUE shape:** A's dense inputs plus one binary feature per (card id, zone, side) | about 3,000 inputs, 40 to 80 active; a first layer 16 wide (about 50,000 weights) at 100,000 games, wider only as the data grows | the active columns summed (adds only), then a small dense head: a few µs | yes |
| **C. A set network:** a small MLP per unit (its card's embedding, its stats, its keywords), pooled per side, then a head | per unit | about 10× B's | yes, and per-unit interactions |

**Recommendation: B.** It is the smallest shape that can learn what the hand eval cannot see. C is
the step up if B's per-card worth is too coarse. A card id the weights do not know (a new card, or
a custom card, #915) gets no sparse feature and is valued by its dense stats alone, so a new card
never breaks the AI.

### D6. Where it plugs into the search

| Option | Calls replaced | Risk |
|---|---|---|
| **A. Everywhere `evaluate(.., &AI_EVAL)` is called** (`search.rs`, `reply.rs`, `simulate.rs`) | all | the beam prunes on the new scale, and auto-answers change too |
| **B. Only where a line is finally scored:** `reply_score` and `static_score` for the finalists | a fraction | the beam's shape stays the hand eval's; the network picks among its lines |

**Recommendation: B first, then A as its own screened change.** Either way the output is mapped
onto the hand eval's points (a logit times a fitted scale), because `unspent_mana`, the lethal
constants and the ladder bots' `temperature` (R1452) are all in evaluation points. A promotion is
followed by the bots' re-calibration anyway (`docs/bot-calibration.md`).

**Measured (2026-10-18): B takes half the correction.** With the 36,000-game network, the whole
correction changed the chosen action at about one search decision in four (561 of 2,400 sampled with
`think`), yet won only +0.010 edge over 1,600 paired games. Fitted to outcomes, the network tells
positions a game apart better than lines a move apart. Half of it won +0.064 [+0.024, +0.105] on the
same deals, so a finalist takes `LEARNED_CORRECTION_SHARE` of the correction (R1012001).

### D7. Arithmetic, determinism and size

The issue decides on integer inference (i8 weights, i32 sums, ReLU, shifts) so that native and WASM
agree bit for bit. Integers are safe and keep the weights small. But an `f64` network with ReLU
only, its loops written in one order, is deterministic by the same argument the hand eval already
rests on. IEEE 754 makes `+`, `×` and comparisons exact on every target, and Rust never
reassociates or fuses them. ReLU is written as a comparison rather than `f64::max`, so a signed
zero cannot differ.

| Option | Extra work | Weights in the WASM bundle (B's shape: about 50k weights at first, 200k at most) |
|---|---|---|
| **A. Integers** (the issue) | quantisation, a Python integer reference, per-layer shifts, a loss check (part 6) | 50 to 200 KB as i8 |
| **B. `f64` with ReLU** | none | 0.4 to 1.6 MB raw, more as JSON text |
| **C. B to screen natively, A to ship** | the quantisation only once a screen has shown a gain | as A |

**Recommendation: C.** The first screen runs native against native, where parity is not in
question, so it needs no quantisation. Quantise only a network worth shipping. Integers then keep
the browser's download small, and part 6's parity fixture (#1011) proves agreement either way. The
weights go in as text (`include_str!`, rule 4). i8 stored as base64 in a JSON field keeps that
small.

### D8. Where the training code lives, and in what language

| Option | For | Against |
|---|---|---|
| **A. Python and PyTorch (CPU), `training/ml/`**, its own uv project on `analysis/`'s conventions | fast to experiment; the issue's choice; #443's conventions exist | a second language in the loop |
| **B. The same, inside `analysis/`** | one Python project | CPU torch is hundreds of MB, and `super.yml`'s notebooks job would install it daily for nothing |
| **C. Rust in `crates/tools`**, a hand-written MLP with SGD | one language; no Python on the runners | slow to experiment with; every model change is Rust |

**Recommendation: A**, with a `pyproject.toml` that takes CPU-only torch from PyTorch's index.

### D9. Which games use it

- **1v1 games, every tier:** yes. The tiers change resources, never the evaluation (R180).
- **Duos:** keep the hand eval until the network is trained on Duos duels; `effective_life` and the
  pool are inputs it has never seen (R964000).
- **The tutorial:** it plays the AI every tier plays (R290), and its lesson tests have vetoed real
  gains before (#850, finding 5). The default is to ship to it too and let the R293 tests decide. If
  they fail, the fallback, the tutorial keeping the hand eval, changes R290 and is a person's call.

### D10. Patches and versions

The weights record `FEATURES_VERSION` and the catalog version they were trained on. A
`FEATURES_VERSION` mismatch is refused at build time (a test). A catalog mismatch is allowed:
the cards' dense stats are read live, and only their learned extra worth is stale. The gather lane
already plays every patch, so retraining per patch is cheap (D1, D3).

## The parts

Issue #466 tracks them, one sub-issue each, in build order. Strength is measured (parts 4 and 5)
before any shipping work (part 6):

| Part | Issue | What | Who | Blocked by |
|---|---|---|---|---|
| 1 | #1006 | Features and the log export: `crates/ai/src/features.rs` with `FEATURES_VERSION`, `cargo jackioh selfplay` (logs, on seeds that are neither the gate's nor the dev set's, R850001), `cargo jackioh positions` (logs to features, each game's hash checked) | decide later; hard | nothing |
| 2 | #1007 | A dispatched export workflow: a sharded `selfplay` matrix on GitHub's runners or the local runner (D1) | people (`.github/`) | #1006 |
| 3 | #1008 | A local self-hosted runner for the exports, label `jackioh-selfplay-local` | people (a machine) | nothing; its first run needs #1006 and #1007 |
| 4 | #1009 | The first 100,000 games and an offline fit in `training/ml/`: the calibrated hand eval, a linear fit and D5's network; the learning curve; the hand eval's most misjudged cards. **Go/no-go 1** | decide later; hard | #1006 |
| 5 | #1010 | `learned.rs` in `f64` behind `AI_EVAL_KIND` (D6's B, 1v1 only) and an 800-game native screen on `improve-dev`. **Go/no-go 2**: `edgeLow > 0` | decide later; hard | #1009 |
| 6 | #1011 | Integer inference (D7), the parity fixture natively and in the WASM build, the perf gate and `msP95` under 2,000 | decide later; hard | #1010 |
| 7 | #1012 | `training/ml/retrain.sh` with no LLM inside it (D2), `improve.md`, the gate's dry run, and the promotion: `AI gen N (improve): …` | decide later; hard | #1011 |

## Rough cost

| Item | Compute | Money |
|---|---|---|
| Part 4's first export, 100,000 games (about 7 million decision positions) | under 4 h of 20 hosted runners, or about 17 h of a 16-core local runner | none |
| Scaling to a million games, if the learning curve asks for it | about a day and a half of hosted runners | none on hosted runners; about $135 on AWS |
| A fit | minutes of CPU | none |
| Part 5's screen | 800 games: about 32 minutes on a 4-vCPU runner (measured above), hours on the loaded box | none on hosted runners |
| A bigger training box (optional; helps the lanes whatever happens here) | `m7i.xlarge` or `m7i.2xlarge` | about $150 or $290 a month on demand (#383), paid plan needed |
| Devin, if the lane supervises retraining after 10-16 | per session | Devin's plan, 6× to 12× credits |

## A person's calls

1. **Compute (D1).** The export workflow (#1007, `.github/`) and the local runner (#1008, a
   machine) are people's. A bigger AWS box means the paid plan.
2. **Supervision (D2).** Whether the improve lane takes retraining over in #1012, given Devin's
   cost after 2026-10-16.
3. **The tutorial (D9).** Only if the lesson tests reject the network: whether the tutorial may keep
   the hand eval, which changes R290.
4. **The bars.** #466 leaves them to a person. Since #850 they are 220 of 400 paired games, about a
   55% true win rate, low enough that a learned evaluation's first real gain can pass.
5. **Who builds the code parts** (#1006, #1009 to #1012): label one `bot:build` for the night bot
   (`difficulty:hard`, so Opus only), or run it in a session a person watches. None is queued.
