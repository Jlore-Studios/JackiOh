# `jackioh-ai`

The computer opponent of practice games (spec §9.9). It plays one seat of an ordinary engine game:
it reads a state, returns one `ActionBody`, and the caller reduces it. It also plays the AI team of a
Duos game, both seats, on the server (§13.7, [Duos](#duos-137) below). The design is
`docs/polish/3-ai.md`; this file is the contract the WASM module, the tools and the training lanes
rely on.

## Purity

Pure and seeded like the engine (CLAUDE.md rule 4): its `Cargo.toml` depends on the engine, `serde`,
`serde_json` and `indexmap` only, and its `clippy.toml` is the engine's (no clock, no I/O, no
environment, no threads, no `HashMap`, no `RefCell`). Every random draw comes from an `Rng` the
caller passes in. The clock arrives only as a callback (`AiOptions::should_stop`, `SweepOptions`'s
`now`); a decision the sweep's clock found slow goes out the same way, to `SweepOptions`'s `retime`
(R977000), which `cargo jackioh sweep` answers against the perf gate's yardstick. A node counter keeps its tallies in `Cell`, which the lint allows.
The Death, the turn-hook and the activation prices of a decision (R2424000, R2477000, R2564000) travel
with it as one value, `evaluate::Prices`, passed by reference from `decide` through the search to the
scorers that read them; a call outside a decision passes `Prices::NONE` (plain `evaluate`, the probes, the
baselines, the tools) and prices nothing. The crate holds no thread-local.

`src/` never depends on `jackioh-cards`. Callers call `jackioh_cards::register_all()` first: the WASM
module, the `jackioh` tools, and this crate's tests (dev-dependency).

Every AI number is a named constant (CLAUDE.md rule 9): `AI_BUDGET`, `AI_GATE_BUDGET`, `AI_SEARCH`,
`AI_DETERMINIZE`, `AI_MULLIGAN`, `AI_MULLIGAN_CURVE`, `AI_DEATH_PROBE_NODES`, `AI_TURN_HOOK_PROBE_NODES`
and `AI_ACTIVATION_PROBE_NODES` in `src/config.rs`, and
the deck, gate, sweep and development-run numbers in their modules (`AI_DECK` in `deck.rs`, `AI_GATE` in `gate.rs`,
`AI_SWEEP` in `sweep.rs`, `AI_DEV_RUN` in `dev_run.rs`), with the Duos gate's `DUOS_GATE_GAMES_SMOKE`,
`DUOS_GATE_WIN_FLOOR` and `DUOS_GATE_GREEDY_WIN_FLOOR` in `src/config.rs`. The tiers are the engine's (`AI_DIFFICULTY`, `AI_TUTORIAL`
and Duos's `duos_ai_team` in `crates/engine/src/config.rs`).

## `decide` reads only `redact`

```rust
pub fn decide(state: &GameState, seat: PlayerId, options: &mut AiOptions) -> Option<Decision>;
pub fn decide_styled(state: &GameState, seat: PlayerId, options: &mut AiOptions, style: &AiStyle) -> Option<Decision>;
pub struct AiStyle { pub temperature: f64, pub missed_lethal_chance: f64 }
pub struct AiOptions<'a> { pub rng: Rng, pub budget: SearchBudget, pub should_stop: Option<&'a dyn Fn() -> bool> }
```

`decide` is the one entry point. It reads the true `GameState` only through `ai_to_act` and `redact`
(R185): `redact` blanks what the seat may not know (the opponent's hand (its Resources among it,
R2250000) and library, the backrow cards it cannot read, an enemy face-down card its own prompt offers among
them (its option reads the view's `Face-down card`, R177), its own library's order and the cards in
it that came from the opponent, the seed, the event history, every card's hidden Nerfs, its own
hand's too (R1845000), what the opponent bought at the Marketplace (R1853004), and the opponent's Created cards' generators (R1956001)), and strips from the events its queues keep what `view_for` strips of a
hidden card. Every simulation runs on a `determinize`d copy whose hidden cards are resampled from the
non-token cards of every set the opponent has not shown (R185, R380), a hidden backrow card from the
traps alone, minus any whose live aura would change a shown stat (R602), and of the shown cost while
an unseen trap of that cost is left (R351, R762). So two states that differ only in hidden cards give the same decision under
the same rng. The caller reads `options.rng.cursor()` back afterwards.

The AI never concedes and never offers a draw, and it declines every draw offer at once (R188). It
answers its mulligan as soon as the mulligans open (R265, R266): `mulligan_keep_curved` sends back
every card costing more than `AI_MULLIGAN.keep_max_cost` except the one proven four-drop the shaped
keep holds, and keeps the cheap ones to `AI_MULLIGAN_CURVE` (#993): at most two one-drops, and only
the cheapest card when none is an early play, counted in the seat's crystals (its mana bonus and,
going second, The Coin). Nothing here reads a difficulty, and a handicap only as the seat's mana (the
mulligan's bonus and cap): the tiers change the AI seat's resources only (R180), and the tutorial's
opponent is the same AI on `AI_TUTORIAL` with the lesson's fixed deck (R290, R291).

A question of a sequence (Meditative+ #20 KY's Competition, R1184000) is answered at once, before
any search, with the option its redacted statement works out to (`ky_competition::worked_answer`),
never Stop: the redacted copy judges no answer right (R465), so a search would rank the answers alike
and spend a decision's budget on a guess up to thirty times a turn, and worked out each answer costs
one redaction (R1184002).

`decide` runs a bounded lethal solver (`lethal.rs`), then a turn-level beam search, scoring the best
lines one turn deeper after the opponent's reply (`reply.rs`: the cards it watched arrive, the Activates
it can see on the field, then its attacks, a kill priced at what it removes (a unit with Reborn unspent
less the body it comes back as, #1796, and less what its Death leaves behind, R2424000; plus a Trample
attacker's excess on the hero and a Lifesteal attacker's heal, #1820), and once none is worth making the switches to Defense Position its own
evaluation prefers) on every determinization, and plays the first action of the best mean. A line
leaving a countdown waiting (an effect "in N turns" with its public badge, R1181000) is replied to again
on a copy whose countdowns come due at the seat's next turn start, and the scores are blended at
`AI_REPLY.countdown_share` per turn the farthest still waits past its owner's next turn start (#1995);
with none waiting, nothing changes. A Dormant card (R1839000) acts in no line and threatens no face until
it wakes, but the evaluation counts it as the unit it wakes as, `slumber_turn_share` of it per turn left
on its countdown, or `slumber_waiting_share` with none (R2380000). A Unit standing in the backrow
(R1081000) counts once, as the Unit it is, never again as a backrow card (`standing_unit_priced_once`,
R2490000; off in `GREEDY_EVAL`). A unit's Death counts by what it
leaves behind (R2424000): at the root `death_prices` prices, within `AI_DEATH_PROBE_NODES`, each live
Death on the field and in the seat's hand (spawned first, R923001) as its controller's evaluation after
the sandbox's destroy less after its remove (R923003); `unit_worth` adds `AI_EVAL.death` of it,
`kill_worth` takes all of it off a kill. A Starship Piece is priced the same way, by the copy its death
fuses into its owner's Starship, its probe after every Death's, so pieces never take a Death's nodes
(R2566000). A card's start-of-turn and end-of-turn hooks count
by one firing (R2477000, R2565000): at the root `turn_hook_prices` fires, within `AI_TURN_HOOK_PROBE_NODES`, each
such hook of the units on either field, of the backrow cards the seat can read on either side (a face-down
enemy card is never probed) and of the seat's hand's Field Spells and Units (set or placed first,
R923001) through the engine's trigger queue, and prices it as its controller's evaluation after less
before, each crystal gained at `next_turn_mana`; `material` adds `AI_EVAL.turn_hook` of it for a readable
backrow card, and `unit_worth` for a unit, so an enemy engine is worth more to answer and an own one more to keep.
A unit's Activate counts by one use (R2564000), and so does a face-up backrow card's, a Field Spell's
(R2603000): at the root `activation_prices` ends the seat's turn and then the enemy's, within
`AI_ACTIVATION_PROBE_NODES`; at the start of each player's next turn it tries that player's units' uses,
then its face-up backrow cards' (`activate_sources`), as the reply does (`visible_activation`), and
prices the best as the controller's evaluation after less before, each crystal at `next_turn_mana`;
`unit_worth` adds `AI_EVAL.activation` of it for both sides' units, and `material` for each readable
backrow card, inside its Brittle share.
It re-plans after every action, so a
lethal the solver proves is played trimmed to the actions it needs
(`trim_lethal`: its shortest suffix that still wins on every determinization, then less each other action
it still wins without, R1965000; none, when the budget or the clock stops the trim before it has that
suffix). A free play (a (0) Spell back from the graveyard under In Too Deep's reward L, R454, or The Coin)
could otherwise open the same lethal at every decision. Each line tries its moves in move order
(`candidates.rs`): attacks on the enemy hero, kills the attacker survives, each attacker's best kill (its
target, or with Cleave a neighbour, #1821) at an even trade or better by the opponent model's own read
(`reply.rs`'s `trade_value`, #1779), the plays round-robin, answers, the Trades
of Tradable hand cards (R1838003: on a determinized copy, so the card one would draw never steers it), the
buys and sales at the Marketplace (R1853004), the other attacks, position switches and End Turn; the root expands its first `root_branching` and each deeper step
its first `branching`, End Turn always among them. Inside one card's plays, each way of being played (its
X, Embiggen, Equip, face-down set, Tribute set) gets its first variant before any way gets a second lane or
target, the dearest way first, then the cheapest, then the rest dearest first, Tribute sets cheapest units
first (#1778). Its root tries a play in every distinct placement (the zones beside its lane and
across it), each in its lowest lane, or its midlane lane when it has one, which a Cry may read (R2203001);
deeper levels, the lethal solver and the reply try the first `AI_SEARCH.zone_variants` (#1635). The beam
drops an open line whose position an earlier one reached (two orders of the same moves into the same lanes
are one position). A line that differs from an earlier one only in where its units stand is a lane variant
(#1797): the evaluation reads no lane, so it takes no place in the beam, but when the beam keeps the earlier
line, the first `AI_SEARCH.zone_variants` - 1 of them end their turn where they stand after it, below that
line on a tie. So the placements the beam tries reach the reply, which prices a Cleave into a unit's
neighbours; a lane the reply scores the same as a better one of that play is not a finalist of its own. The
lethal solver's visited set still reads a position without its units' lanes. Position
switches are the last tier of the move order, so a level's `branching` cut drops them from a busy board;
the first `AI_SEARCH.position_switches` stand-downs (Attack to Defense of a unit an enemy unit that could attack it next turn would kill and survive, `kills_and_survives`; R2021000) it cuts are
kept (#1759), so a unit that cannot win its next fight can stand down however many earlier moves the board offers. An action the
true state refuses (a cost no determinization could see) is never submitted: `submitted` sends the
first action of the next finalist the true state accepts, best mean first, with the search's reason, and
only when none is accepted falls back (`DecisionReason::Fallback`) to `replacements_for`'s endTurn, answer
or other legal action (#1575).

`decide_styled` is `decide` under an `AiStyle` (R1452), the server's ladder bots' strength knob beside
their budget: a `missed_lethal_chance` to pass over the lethal line the solver found (the search then
keeps off every line that wins this turn, `lines_missing_lethal`, or it would find the same lethal),
and a softmax
`temperature`, in evaluation points, over the finalists' means (`pick_finalist`) instead of always the
best. Each takes one draw from `options.rng` when it is above zero and none at zero, so `decide`,
which is `decide_styled` at `AI_STYLE_EXACT`, draws only for its determinizations, as it always has.

## The lethal hint (R2102000, R2291000)

```rust
pub fn lethal_hint(state: &GameState, seat: PlayerId, should_stop: Option<&dyn Fn() -> bool>) -> bool;
pub const LETHAL_HINT_SEED: &str;                  // "ai:lethal-hint"
pub const LETHAL_HINT_DETERMINIZATIONS: usize;     // AI_BUDGET.determinizations
pub const LETHAL_HINT_NODES: usize;                // AI_BUDGET.lethal_nodes
```

Whether `seat` has a sure win this turn by what it can see, for the lethal hint of every game the
practice worker plays against the AI (§9.9): the practice worker asks it for the human's seat. Like `decide` it reads the true state only
through `redact`, so two states that differ only in what the seat cannot see give one answer. It
fills the redaction in as `LETHAL_HINT_DETERMINIZATIONS` worlds drawn from `LETHAL_HINT_SEED`'s
stream, a stream of its own (never the match's or a caller's), and answers whether `find_lethal`
proves a line that wins on every one of them within `LETHAL_HINT_NODES` nodes. It is false outside
`seat`'s own main phase with no prompt open, once the game is over, when the search panics, and
when `should_stop` (the browser's clock, as the AI's) stops the search before it proves a line.
`decide` never calls it, so it changes no decision and no game.

## The unstoppable lethal (R2660000)

```rust
pub fn unstoppable_lethal(state: &GameState, seat: PlayerId, should_stop: Option<&dyn Fn() -> bool>) -> bool;
pub const UNSTOPPABLE_LETHAL_SEED: &str;                // "ai:unstoppable-lethal"
pub const UNSTOPPABLE_LETHAL_DETERMINIZATIONS: usize;   // AI_BUDGET.determinizations
pub const UNSTOPPABLE_LETHAL_NODES: usize;              // 2 × AI_BUDGET.nodes, the whole check's
pub const UNSTOPPABLE_LETHAL_REPLY_NODES: usize;        // AI_BUDGET.lethal_nodes, one proof of the opponent's lethal
```

Whether `seat`, at its own turn, faces a lethal showing on the opponent's board that nothing it does
this turn stops. Like `lethal_hint` it reads the true state only through `redact`, and it counts only
what shows: the opponent's hand and face-down cards are taken out, so no unseen trap stops the seat's
answer and no unseen card makes the lethal. It fills the rest in as
`UNSTOPPABLE_LETHAL_DETERMINIZATIONS` worlds from `UNSTOPPABLE_LETHAL_SEED`'s stream and, on each,
plays every line of the seat's turn through the engine (every legal action but R84's and the
mulligan, "end turn" first, a position once), and for each line that hands the turn over asks
`find_lethal` for the opponent's lethal on the board alone, the card it drew dropped, then plays that
line again. It holds only when every line on every world ends lost. Every doubt answers false: a cut
walk (`UNSTOPPABLE_LETHAL_NODES`, `should_stop`), a failed step, a step that rolls the game's random
numbers or moves a card of the seat's library (one outcome of a roll, one guessed order, could hide
the escape), an opponent's lethal that rolls, moves a card of its own library or opens a prompt for
the seat, a panic, and every moment but the seat's own main phase with no prompt open. A turn that ends with the
Marketplace open ends without it, so the opponent's turn start rolls no offers (R1853000), and a lethal
that needs the opponent's turn to end is not counted: both err only towards playing on. `decide` never
calls it; the ladder bots ask it before conceding.

## Budgets

A decision's cost is counted in nodes: one node is one `reduce` call the AI makes, the reply's
included, or one turn hook its root fires (R2477000, R2565000). A step whose `reduce` does far more work than most is charged extra nodes up front
(`surcharge_nodes`: a play that ranks every shipped card, an attack into an armed My Pawn, a fused
card holding either, and a play the other side's Pareto Optimality judges, per card held; #1240),
and once a step has run, for each such card it cast without playing it, from a graveyard or at random
(`announced_surcharge_nodes`, read off its `cardAnnounced` events; #1347).
A prompt answered ahead inside a simulation (`best_auto_answer`: the other seat's in a step, the
reply's, and the beam's look past the seat's own; #891, #1112) tries at most
`AI_SEARCH.max_auto_answer_options` answers, one node each: all of them, in `action_key` order, when
there are no more, and otherwise that many drawn from the whole set on a stream of the world's seed
and the prompt's id (`AI_SEARCH.auto_answer_seed`; #1382), so neither a late place in the list nor
its order keeps an answer from being tried.
Every simulated step goes through `reduce_unpiloted`, which leaves a turn the autopilot owes
(Cognitive Offloading, R1183002) owed instead of playing it by §10.7's scorer inside the step: a line
that hands a turn over is scored where that turn begins (#1337).
`AI_BUDGET` is what the browser plays with; `AI_GATE_BUDGET`, the gates' and the sweep's, is the
same budget, so both measure the AI that ships. The same state, seed and budget always give the
same `Decision`. The browser's wall-clock cap arrives as `should_stop` (the WASM binding's deadline),
which ends the search early with the best line found.

## Decks and the shadow ban (R184, R186, R390, R1370, R1390, R817000, R976000)

`build_ai_deck(rng, size, &AiDeckOptions)` deals `size` distinct non-token ids from every set by
weighted sampling without replacement (`AI_DECK`), leaving out `SHADOW_BAN_IDS` unless `banned` says
otherwise. `lean_set` (R1370) names a set at least `AI_DECK.lean_min_share` of the deck comes from, a
hard floor kept together with the units' and the theme's; absent, the draw is exactly the one it always
was. `build_ai_deck_traced` also counts the picks that floor forced, the measurement `lean_boost` was
chosen by. Every deck holds at most `AI_DECK.max_quickdraw` (2) Quickdraw cards, an `include`d one
counted and always dealt, and Quickdraw is never a theme (R817000). `gated_sets` (R1390) names the sets the draw never deals from: its default is the soft gate
`AI_DECK_GATED_SETS` in `src/config.rs`, so every AI deck holds none of them whatever ships or a
thread previews, and a lean on a gated set leans on the newest ungated set that ships (`ungated_lean`).
A player's random deck passes `Some(vec![])` beside `banned: Some(vec![])`; an `include` still forces a
gated card in by name. Emptying the list lifts the gate and deals every deck as before it, which issue
#732 did once it had measured the AI on the Meditative set, the list's one set until then. It held the
Stinkyscape set from its release (issue #1884) until issue #1885 measured the AI on it: gating a set that
ships before the AI is trained on it is listing it there. `shape`
(R976000) multiplies each card's weight by `shaped_weight`, the unban lane's dealt-quality table in
`src/shadow_ban.rs`; it is off unless a caller sets it, whatever `banned` holds, and no caller does, so
practice, the quality gates and both seats of an arena or a promotion deal unshaped. The mulligan still
reads `shaped_weight` (`mulligan_keep_shaped`).
`src/shadow_ban.rs` holds `SHADOW_BAN` (each card with the sweep flags that banned it) and
`SHADOW_WATCH` (cards found at risk and cleared). Both are decided by the sweep, never by hand, and
never copied in by hand either: `cargo jackioh sweep --write <report>…` rewrites the file's header block
and both tables from the reports of record (`training/history/sweep-<date>.md`, the output of `--report`,
oldest first), and `cargo jackioh sweep --check` and a `jackioh-tools` test fail on any difference.
A card takes the row of the latest report that lists it, in either table (R1186000), and none when the
latest report to sweep it cleared it (R1675000); `SHADOW_BAN` keeps the Meditative, Meditative+, Stinkyscape and Training Grounds rows,
less `SHADOW_UNBANNED`, the unban lane's removals (`--write` takes none out but those and the cleared).
The ban governs AI deck
building and nothing else: a banned card stays legal for every player. Since #732 the ban holds
Meditative cards only, at most half of the set (R732000), and since #1186 Meditative+ cards too, each
set held to its own half (R1186000), and since #1885 Stinkyscape cards, at most
`STINKYSCAPE_BAN_CEILING` (35) of them (R1885000), and since #2203 Training Grounds cards, at most half
of them (R2203000): no sweep, lane or person adds a card of an
earlier set, and the unban lane (`training/unban.md`) only removes entries.

**When it is swept** (R1678000): every push to `main` that moves `generation.json`'s `generation`
(every promotion writes it) or adds a set to `SHIPPED_SETS` starts a sweep of record on GitHub's
runners, `.github/workflows/sweep.yml`. `cargo jackioh sweep --due <base>` names the cards: the bans and
the watched cards of the sets the ban may hold when the AI changed, every non-token card of a set that
ships now and did not. Its report goes under `training/history/`, `--write` reads it last, and a
changed table arrives as a `Patch: the shadow ban swept again after …` pull request with auto-merge
on, `generation.json`'s `shadowBan` and the practice-claim fixture with it. So the sweep after a
promotion may ban a card of those four sets again when the new AI is slow with it, within each
set's ceiling; a record `--write` refuses opens an issue for a person instead.

`FIXTURE_BANNED_IDS` (issue #1113) is the frozen fixture pool: a copy of the table as it stood
when the seeded fixtures were dealt from it. It is never updated with the table; a fixture that
needs a new deal changes its seed instead. R290's tier games, the `cargo test` gate smokes behind
`fixture_game_config`/`run_fixture_gate_games`, and the named practice seeds (R265's mulligan
seeds, B34's walk, R677's glitch-swap) and spec 13's URL games deal the AI from it. `gate --full`,
promotion, sweep and live practice keep the live table.

## Matches and the quality gates

`play_match(&MatchConfig, &mut MatchHooks) -> MatchRecord` plays one whole game between two
controllers (the AI, `greedy_action` or `random_action`, §10.7's random policy) and returns its log,
hash and bookkeeping; `play_ai_turn` plays one AI turn out. The gates (`run_gate_games`, `AI_GATE`) are
three matchups whose subject alternates seats, every game folded back to its hash. A run of n games
needs `gate_needed(matchup, n)` wins (the rule spec §9.9 proposes); only wins count.

| Matchup | Subject | Opponent | Full run | Smoke run |
| --- | --- | --- | --- | --- |
| `ai-vs-random` | the AI on Easy | random policy on Easy | 91 of 100 | 17 of 20 |
| `ai-vs-greedy` | the AI on Easy | greedy baseline on Easy | 28 of 50 | 10 of 20 |
| `hard-vs-easy` | the AI on Hard | the AI on Easy | 40 of 50 | 16 of 20 |

The perf gate holds each decision of the first ai-vs-greedy game, and two hand-built wide boards,
within the node budget and under `AI_GATE.max_decision_ms`, and the two pinned positions
(`tests/fixtures/pinned-improve-dev-*.json`, #1240) under `AI_GATE.pinned_decision_ms`. Tuning
changes the weights in `src/config.rs`, never the floors, the seed counts or the brief's rates. The
gates see only a broken AI; whether a change made it stronger is the training lanes' question
(`training/README.md`, `cargo jackioh promote`).

Every gate game is clean (B31): nothing refused or thrown, no fallback, a result, a log that folds to
its hash, and no panic caught in an AI's search (`SearchStats.sim_panics`, summed in `MatchRecord`,
#1429). In the browser such a panic is a WebAssembly trap that loses the whole decision. A failure
names the seed, the seat and the first message (`Decision::first_sim_panic`, native only).

A seat the AI plays has a skill (R747000): `SeatController::Ai { budget, skill }`, the chance in
`[0, 1]` that each decision follows the AI, the rest being the skill miss (`skill::skill_miss_action`,
R2214000): a softmax at `AI_SKILL_MISS_TEMPERATURE` over the greedy baseline's one-ply scores of the
true state's candidates, read on one determinization of what the seat sees. `None` is `AI_SKILL_MAX`:
full skill, which takes no draw, so a seat without one plays as it did before the knob; `AI_SKILL_MIN`
is the miss at every decision, with no search. In
between, `skill::plays_best(skill, rng)` rolls the seat's own seeded stream once per decision
(`skill::clamp_skill` pins the range, a NaN reading as full), so a seeded run reproduces. The knob
knows nothing of cards or sets: it keeps meaning the same when the AI improves or a set ships. The
arena (`crates/tools/README.md`) and the server's ladder-bot seat (`crates/server/README.md`) roll it
the same way, and the skill-to-Elo curve is measured with the arena.

`MatchConfig.setup` (`MatchSetup`: `dealt`, `lastBoards`, `glitchBoards`, written beside the config's
other keys) carries the setup inputs `create_game` takes beyond the decks and handicaps (R433, R417,
R564, R678), and `MatchConfig::fold_args` folds a record with all of them (#1697). Unset, a game is
created, played and folded exactly as before. No gate sets it; `SweepOptions.setup` opts a sweep in,
the same inputs in seat order in every game.

## Development runs for the card statistics (R378)

`dev_game_config` and `dev_game_record` deal and play game n of a development run as All Random
deals a live game (R258), both seats AI on the spec's resources at `AI_BUDGET`, filed as a game
record of source `dev` under the patch the run tests, id `dev:<patch>:<seed>` (`dev_record_id`).
`cargo jackioh stats --out <file>` writes a run; `jackioh-server stats-import <file>` loads it.
`forced_dev_game_config`, which `DevRunOptions.force` selects, deals the same game with one card in one
seat's deck, p1's in odd games and p2's in even ones (R1284000). Both decks are dealt with the card
banned, and the holder's card at the slot `${seed}:force` picks is replaced by it.
`DevRunOptions.setup` creates every game of a run with the same setup inputs (`dev_run_config`), so a
record stays a pure function of its number.

## The learned evaluation's features (#466, #1006)

`src/features.rs` turns a position into the learned evaluation's inputs (`docs/learned-eval.md` D3,
D5, D10). One function serves the replay pass that makes training rows (`cargo jackioh positions`)
and, once a network ships, its inference in the search, so the two never read a position apart.

```rust
pub const FEATURES_VERSION: u32;
pub fn features_of(state: &GameState, seat: PlayerId) -> Features;   // features(&redact(state, seat), seat)
pub fn features(view: &GameState, seat: PlayerId) -> Features;       // view already redacted
pub fn dense_names() -> Vec<String>;                                 // the dense layout, in order
pub struct Features { pub dense: Vec<i32>, pub sparse: Vec<(String, i32)> }
```

- It reads only `redact` (R185, R1006000): two states that differ only in what the seat cannot see
  give the same features. The projections that read card text (`unit_view`, `hero_armor_of`,
  `face_threat`) run on the view less the backrow cards the seat cannot read, which are counted and
  never read.
- **dense**: integers, named by `dense_names`: the turn, who acts and whether the seat is p1; per
  side, the seat's own first, its life (`effective_life`) and armor, mana, crystals and debt, its
  hand, library, graveyard and backrow (and how much of the backrow the seat can read), its units in
  aggregate (count, positions, attack, health, armor, damaged, Radiant), one count per keyword kind
  (bar Tradable, a hand keyword newer than the layout, so the fitted weights still read it, R1838000),
  the face damage its board threatens and its Jade and extra turns; then the seat's hand by cost,
  and how many of its cards are Radiant (R1012004).
- **sparse**: `(key, count)`, sorted, the key `<zone>:<side>:<card id>` for each card the seat can
  name: `board` (both sides), `hand` (its own) and `backrow` (its own, and the enemy's face-up ones),
  and `hand_radiant` (its own) beside `hand` for a Radiant card in its hand (R1012004).
  Ids stay strings, so a patch or a new card never moves an index; an id a network's weights do not
  know is no input at all.
- `FEATURES_VERSION` names the layout; any change bumps it, and a network's weights record the
  version they were fitted on.

## The learned evaluation (#466, #1010)

`src/learned.rs` is a small network over `features` that corrects the hand eval where the search
finally scores a line (`docs/learned-eval.md` D4, D6, D7, D9):

```rust
pub enum EvalKind { Hand, Learned }
pub const AI_EVAL_KIND: EvalKind;                 // config.rs: Hand, unless a promotion turns it on
pub const LEARNED: Network<'static>;              // compiled in from src/learned/weights.json by build.rs
pub fn finalist_score(state: &GameState, seat: PlayerId, next: NextSwing, prices: &Prices) -> f64;
```

- With `Hand` nothing changes. With `Learned`, `reply_score` scores a finalist after the opponent's
  reply with the network's logit mapped back onto the hand eval's points, `points = hand + (b +
  net(x))·U/a` (R1010000, R1010001), taken at `LEARNED_CORRECTION_SHARE`: `hand + share·(points −
  hand)` (R1012001). The beam's pruning keeps the hand eval. 1v1 only: a Duos duel keeps the hand
  eval (R1010002).
- `src/learned/weights.json` is what `training/ml/jackioh_ml/export.py` writes from a fit: the feature
  version and dense names it was fitted on (a test fails on a mismatch), the standardizer, the sparse
  keys it has rows for (an unknown key is no input), the layers and the skip connection. `build.rs`
  turns it into constant slices: data at compile time, no static at run time (rule 4). The checked-in
  file may be the null network, whose correction is zero everywhere.
- Inference is `f64`, one fixed order, clipped ReLUs as comparisons, no libm, for a float network
  (`"format": 1`, `Learned::Float`), which part 5's native screen measures. The network that ships is
  an integer one (`"format": 2`, `Learned::Int`, `src/learned/int.rs`, #1011): `training/ml`'s
  `quantize.py` folds the standardizer into an i16 first layer over the raw features, i8 head layers,
  i64 sums, floor shifts and clips, and one IEEE division at the end, so the native gates and the
  browser's WASM build give the same answer bit for bit (R1011000; `tests/fixtures/learned-parity.json`,
  held natively and through the WASM module's `learned_parity`).

## The policy prior (#1171)

`src/policy.rs` is a table of weights, fitted by `training/ml`'s `policy_fit.py` on the moves the AI
played, that re-sorts the moves the beam expands before it takes its prefix (`docs/learned-eval.md`,
learned move ordering):

```rust
pub enum MoveOrder { Heuristic, Learned }
pub const AI_MOVE_ORDER: MoveOrder;               // config.rs: Heuristic, unless a promotion turns it on
pub const POLICY_PRIOR: PolicyPrior<'static>;     // compiled in from src/policy/weights.json by build.rs
pub fn policy_keys(view: &GameState, seat: PlayerId, action: &ActionBody, rank: Option<usize>) -> Vec<String>;
pub fn policy_label(state: &GameState, seat: PlayerId, played: &ActionBody) -> PolicyLabel;
pub fn prior_order(state: &GameState, seat: PlayerId, moves: Vec<ActionBody>, prior: &PolicyPrior<'_>) -> Vec<ActionBody>;
pub fn beam_search_with(det, seat, counter, budget, order: MoveOrder, prior: &PolicyPrior<'_>, prices: &Prices) -> Vec<Line>;
```

- With `Heuristic` nothing changes. With `Learned`, `search::expansion` sorts each line's candidates
  that are not End Turn by the prior, best first and ties in their own order, then takes the first
  `root_branching` or `branching` and End Turn, as before (R1171001). It never adds, drops or changes
  a move and spends no node. A table of no keys, the checked-in null one, leaves every order as it is,
  and so does a Duos duel.
- A move is a handful of text keys read from `redact` alone (R185, R1171000): its kind, its rank in the
  heuristic order (capped at `POLICY_RANK_CAP`), the card it plays or acts with (named by R1006000's
  rule, else `hidden`), where it lands and the class of what it aims at. No key holds an instance, hero
  or prompt id. Its score is the sum of its keys' weights; a key the table lacks adds nothing.
- `policy_label` is what `cargo jackioh positions --policy` writes on each decision row: the keys of
  every candidate the search's root lists on decide's probe world and which one was played.
  `src/policy/weights.json` records the `POLICY_KEYS_VERSION` and catalog it was fitted on (a test fails
  on another version); `build.rs` turns it into constant slices, data at compile time (rule 4).

## Duos (§13.7)

`src/duos.rs` plays a seat of a Duos game (`jackioh_engine::duos`): one AI plays both seats of team B,
and any human seat the server hands it after a disconnect (§13.8).

```rust
pub fn duos_decide(state: &DuosState, seat: DuosSeat, options: &mut AiOptions) -> Option<DuosActionBody>;
pub fn duos_decision(state: &DuosState, seat: DuosSeat, options: &mut AiOptions) -> Option<DuosDecision>;   // { body, fallback, sim_panics, first_sim_panic }
pub fn duos_passes(state: &DuosState, seat: DuosSeat) -> Vec<String>;
pub fn duos_gate(games: usize, seed: &str, opponent: DuosOpponent) -> DuosGateReport;   // { opponent, games, wins, draws, rejected, fallbacks, sim_panics }
pub fn duos_greedy_action(state: &DuosState, seat: DuosSeat, rng: &mut Rng) -> Option<DuosActionBody>;
```

- `duos_decide` is `decide` on the seat's own duel: its mulligan and its prompts as a 1v1 seat answers
  them, and its own turn by the same search. When `decide` would end the turn, it first sends the
  passes `duos_passes` names, one action at a time, then the `endTurn`. `None` when the seat owes
  nothing. The same state and options give the same action.
- `duos_decision` is `duos_decide` with where its action came from: `fallback` is true when the
  wrapper's own fallback picked it (`decide` answered nothing the seat may send) or `decide` itself
  fell back (`DecisionReason::Fallback`); `sim_panics` and `first_sim_panic` are the panics `decide`'s
  search caught and the first one's message (#1429).
- `duos_passes` names the hand cards to pass now, best first: a card the seat cannot play this turn
  and its teammate can (a round seat that has not ended, can afford it and has a place for it: an
  open zone of its row or, failing that, any play of it the engine lists, onto a Stack, into the
  backrow with Flying, onto an Equip host or a stack base, or into a zone its Tribute frees), when
  the seat can pay the pass; with `pass_cost` 0 also a card the teammate can afford next turn and the
  seat cannot; never more than the passes left or the teammate's free hand slots. Immediate passes
  rank by the teammate's best bounded follow-up (R1127000; at most `DUOS_PASS_MAX_FOLLOWUPS` plays
  on a fixed-seed probe), ties by cost then hand order; free-pass next-turn cards keep cost order.
  An immediate pass is named only when it is worth it (R1723000): the teammate's best follow-up with
  the card, also tried after the teammate's own best play, beats its best without it by more than the
  card's `hand_card_value` in the seat's hand, `DUOS_PASS_MARGIN_PER_COST` per crystal of `pass_cost`
  and `DUOS_PASS_MIN_GAIN`.
- It knows what its team knows and nothing more: its own duel through `decide`'s `redact` and
  `determinize` (R185), and its teammate's own state, which a team shares (§13.6). An opponent's
  hidden card never changes a decision.
- A seat's life is the engine's `effective_life` (own HP and the team's pool while unspent, §13.4)
  everywhere the AI reads one: the evaluation, `ready_gap`, the reply's `reply_face_value` and the My
  Pawn surcharge. The hero value and `closing` are anchored at `starting_life` (`DUOS_OWN_HEALTH` plus
  the pool's cap). A 1v1 seat reads its hero's health and `HERO_HEALTH`, as before (R964000).
- The tiers are `duos_ai_team(difficulty)` for the team (Easy the humans' rules, Medium free passing,
  Hard free passing and a `DUOS_POOL_HEALTH_HARD` pool) and `AI_DIFFICULTY` for each AI seat's
  handicap, which the caller sets in `DuosCreateOptions`; nothing here reads a difficulty. The AI
  never concedes (R188), so it neither offers nor confirms a team's concede.
- `duos_gate` plays the Easy AI team against one `DuosOpponent`: a team on §10.7's random policy
  (seeds `${series}:${n}`) or a team on the 1v1 gates' greedy baseline (#1760): `duos_greedy_action`
  plays each seat's own duel with `greedy_action`, never passes, and draws only on its seat's stream
  (seeds `${series}:greedy:${n}`). A matchup passes at its opponent's floor, `DUOS_GATE_WIN_FLOOR`
  (0.8) against random and `DUOS_GATE_GREEDY_WIN_FLOOR` (0.7) against greedy, of
  `DUOS_GATE_GAMES_SMOKE` (20) games with no action of either team refused, no AI action off a
  fallback path and no panic an AI seat's search caught (`cargo jackioh gate --duos` plays both);
  `duos_gate_game_at` plays a game at another budget, and `duos_gate_game_with` one with a stand-in
  decider (`DuosGateDecider`) for the AI team, which the tests use to show the gate catching a
  weakened pass.

## Commands

```
cargo test -p jackioh-ai                      # every AI test (the unit tests, tests/ai.rs, ai_slow.rs) but the daily ones
cargo test -p jackioh-ai -- --include-ignored # and the daily ones: the many-game sweeps, `#[ignore = "daily: …"]`
cargo test -p jackioh-ai --test ai_slow       # one binary: CI gives each its own job (#873)
cargo jackioh gate                            # the three gates and the perf gate at smoke size
cargo jackioh gate --full                     # at full size (the daily super run plays it in shards)
cargo jackioh gate --duos                     # the Duos gate: the Easy AI team against a random team and a greedy team (--games N)
cargo jackioh gate --full --shard 1/4 --out ai-gate-shards && cargo jackioh gate merge ai-gate-shards
cargo jackioh trace ai-vs-greedy 3            # one gate game, turn by turn
cargo jackioh sweep core-011 classic-020      # the shadow-ban sweep over these ids (no ids: every card)
cargo jackioh stats --games 50 --from 51 --out dev.jsonl   # a slice of a development run
```
