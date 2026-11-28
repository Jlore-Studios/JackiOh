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
caller passes in. The clock arrives only as a callback (`AiOptions::should_stop` and `wrap_up`,
`SweepOptions`'s `now`); a decision the sweep's clock found slow goes out the same way, to `SweepOptions`'s `retime`
(R977000), which `cargo jackioh sweep` answers against the perf gate's yardstick. A node counter keeps its tallies in `Cell`, which the lint allows.
The Death, the turn-hook, the activation, the attack-hook and the awakening prices of a decision (R2424000,
R2477000, R2564000, R2688000, R3064000), and the evolution prices (R4648000), travel with it as one value, `evaluate::Prices`, passed by reference from `decide`
through the search to the scorers that read them. `decide` takes them from `decision_prices` on its `probe_world`.
Any call outside a decision passes
`Prices::NONE` (plain `evaluate`, the probes, the baselines) and prices nothing. The crate holds no thread-local.

`src/` never depends on `jackioh-cards`. Callers call `jackioh_cards::register_all()` first: the WASM
module, the `jackioh` tools, and this crate's tests (dev-dependency).

Every AI number is a named constant (CLAUDE.md rule 9): `AI_BUDGET`, `AI_GATE_BUDGET`, `AI_SEARCH`,
`AI_DETERMINIZE`, `AI_MULLIGAN`, `AI_MULLIGAN_CURVE`, `AI_MULLIGAN_TRIBUTE_ALLOWANCE`, `AI_DEATH_PROBE_NODES`, `AI_TURN_HOOK_PROBE_NODES`
and `AI_ACTIVATION_PROBE_NODES` in `src/config.rs`, and
the deck, gate, sweep and development-run numbers in their modules (`AI_DECK` in `deck.rs`, `AI_GATE` in `gate.rs`,
`AI_SWEEP` in `sweep.rs`, `AI_DEV_RUN` in `dev_run.rs`), with the Duos gate's `DUOS_GATE_GAMES_SMOKE`,
`DUOS_GATE_WIN_FLOOR` and `DUOS_GATE_GREEDY_WIN_FLOOR` in `src/config.rs`. The tiers are the engine's (`AI_DIFFICULTY`, `AI_TUTORIAL`
and Duos's `duos_ai_team` in `crates/engine/src/config.rs`).

## `decide` reads only `redact`

```rust
pub fn decide(state: &GameState, seat: PlayerId, options: &mut AiOptions) -> Option<Decision>;
pub fn decide_styled(state: &GameState, seat: PlayerId, options: &mut AiOptions, style: &AiStyle) -> Option<Decision>;
pub fn decide_with_pool_loss(state: &GameState, seat: PlayerId, options: &mut AiOptions, pool_loss: i32) -> Option<Decision>;   // R4188000, Duos
pub struct AiStyle { pub temperature: f64, pub missed_lethal_chance: f64 }
pub struct AiOptions<'a> {
    pub rng: Rng,
    pub budget: SearchBudget,
    pub should_stop: Option<&'a dyn Fn() -> bool>,
    pub wrap_up: Option<&'a dyn Fn() -> bool>, // R3498000: the beam's earlier stop
}
```

`decide` is the one entry point. It reads the true `GameState` only through `ai_to_act` and `redact`
(R185): `redact` blanks what the seat may not know (the opponent's hand (its Resources among it,
R2250000) and library, the backrow cards it cannot read, an enemy face-down card its own prompt offers among
them (its option reads the view's `Face-down card`, R177), its own library's order, the cards in
it that came from the opponent or that it was never shown (R312), what changed on its other cards there, each read
as its owner's list shows it, by the definition and face it went in with and its Created mark, unless its own
prompt offers it (R2691000), the seed, the event history, how many ids the game has handed out (its id counter restarts one past the highest id the redaction names, R4824000), every card's hidden Nerfs, its own
hand's too (R1845000), what the opponent bought at the Marketplace (R1853004), the opponent's Created cards' generators (R1956001), every Created card's maker, the game record's alone (R4672001), and what the opponent's play counts say of their face-down plays: a hidden card on their backrow takes its tags and its set off their counts and counts as a Trap among this turn's plays (R4804000)), and strips from the events its queues keep (an interrupted trap firing's under `data.firing` among them) what `view_for` strips of a
hidden card. Every simulation runs on a `determinize`d copy whose hidden cards are resampled from the
non-token cards of every set their own side has not shown (R185, R380, R2826000: the opponent's hidden
cards and a card of the seat's library from the opponent's deck, R73, against what the opponent shows,
every other hidden card of its library against what the seat shows), a hidden backrow card from the
traps alone, minus any whose live aura would change a shown stat (R602), and of the shown cost while
an unseen trap of that cost is left (R351, R762). So two states that differ only in hidden cards give the same decision under
the same rng. The caller reads `options.rng.cursor()` back afterwards.

The AI never concedes and never offers a draw, and it declines every draw offer at once (R188). It
answers its mulligan as soon as the mulligans open (R265, R266): `mulligan_keep_curved` sends back
every card costing more than `mulligan_keep_bound`, `AI_MULLIGAN.keep_max_cost` plus the seat's mana
bonus within its mana cap (R2977000), nothing dearer (R4110000), and keeps the cheap ones to
`AI_MULLIGAN_CURVE` (#993): at most two one-drops, and only the cheapest card when none is an early
play, counted in the seat's crystals (its mana bonus and, going second, The Coin), ranked by
`hand_card_value` under `AI_EVAL`, ties in hand order (R4110000). A card's cost there is its
`mulligan_cost` (R3936000): its mana cost, or, while it has Tribute X (`tribute_cost_of`, Nerfs,
Buffs and Floats counted), the larger of that and X plus `AI_MULLIGAN_TRIBUTE_ALLOWANCE`, so a
Tribute 3 three-drop goes back and a one-mana Tribute 2 card is neither a one-drop nor an early play.
The greedy baseline's `mulligan_keep` reads mana alone. Nothing here reads a difficulty, and a handicap only as the seat's mana (the
mulligan's bonus and cap): the tiers change the AI seat's resources only (R180), and the tutorial's
opponent is the same AI on `AI_TUTORIAL` with the lesson's fixed deck (R290, R291).

It keeps Classic #37 Last Hurrah and Stinkyscape+ #79 A Glimpse into the Endless Sea (`AI_LAST_RESORT`) for a
last resort (R4783040): the candidate pool (`hold_last_resorts`, which every caller of the move list reads)
drops a play of either unless the seat's times are dire (its life at most `AI_LAST_RESORT_DIRE_LIFE`, or
`face_threat` of the enemy's board reaching it) or it has no other play (no card played this turn, and no
legal play of any other card), and keeps A Glimpse (`AI_LAST_RESORT_REFILLED`) too while a card of its own in
its hand or on its field shuffles cards into its own deck by its running face's text (`refills_own_deck`).
The Trade of a Tradable Last Hurrah stays. It reads only the seat's own hand, field and turn and the public
board, so every determinization keeps the same plays.

A question of a sequence (Meditative+ #20 KY's Competition, R1184000) is answered at once, before
any search, with the option its redacted statement works out to (`ky_competition::worked_answer`),
never Stop: the redacted copy judges no answer right (R465), so a search would rank the answers alike
and spend a decision's budget on a guess up to thirty times a turn, and worked out each answer costs
one redaction (R1184002); and so is C+ #42 KY's Test's Easy problem (#2929). Ahead, a simulated step
answers such a question of the AI's own the same way, judged right on its world, a sequence until
`AI_KY_COMPETITION_EXPECTED_STREAK` (6, Radiant 12) and then Stop (`simulate::worked_step`,
R1563005); the greedy baseline's simulation does not.
R4797000: `decide` plays each sampled world with the opponent's automatic turn end off (`search_world`,
R82, R345, as `unstoppable.rs` does), so a line's End turn stops where the opponent's turn begins, the
crystals it left unspent still in the seat's turn log for `passed_score` and `reply_score`, even against an
opponent with nothing to do, and the reply ends that turn (`AI_SEARCH.hold_opponent_turn`).

`decide` runs a bounded lethal solver (`lethal.rs`), then a turn-level beam search, scoring the best
lines one turn deeper after the opponent's reply (`reply.rs`: the cards it watched arrive, the Activates
it can see on the field (the AI's hero first, then dealt over every card and ability at its most valuable
cards, R4515000), then its attacks, a kill priced at what it removes (a unit with Reborn unspent
less the body it comes back as, #1796, the engine's own read ahead of that body, `reborn_body_of`: its
keeper's card on the side it died on and a unit token at its summoned X/X, `reborn_body_read`, R4798000,
#4798, off in `GREEDY_EVAL`, and less what its Death leaves behind, R2424000; plus a Trample
attacker's excess on the hero and a Lifesteal attacker's heal, #1820, which its hit on the hero counts
too, #3858; an attacker a First Strike defender destroys first earning no chip and no shield pop, since
`fight` reads a blow that never lands as none, R4171000), and once none is worth making, or facing lethal
against its best attack (R4187000), the switches to Defense Position its own evaluation prefers, and
in the first case its switches back to Attack Position of units without Deft too, up to
`AI_REPLY.attack_switches`, R4433000; known plays and Activates together take at most
`AI_REPLY.play_steps` of its `AI_REPLY.max_steps` steps, so its attacks keep the rest, and a reply
whose steps run out still ends its turn, in up to `AI_REPLY.close_steps` more, R4810000) on every
determinization, and plays the first action of the best mean. When none
of the lines `shortlist_lines` takes for the reply hands the turn over with an effect of the seat's own
waiting that the root did not hold (a countdown, a next-turn reward), the best line that does is replied
to as well, last, on a slice of the counter that leaves the other determinizations' share of the
reserve alone, and not once the host's wrap-up has fired (`payoff_score`, R3696000). In the reply, a known card guessing an unrevealed secret of the seat's (a
Fortify Mind) is tried with one guess, drawn on its own stream (`one_guess_per_card`, `AI_GUESS_SEED`)
before anything is weighed, and while a known card would be discarded by the opponent's cleanup,
`known_play` judges each play and standing still after its turn ends (R3696001). A kept secret is
part of a position's signature (its owner, choice and revealed flag, never its minted id), so the
three choices of one Mind Games are three lines the beam weighs (R3772000). A line
leaving a countdown waiting (an effect "in N turns" with its public badge, R1181000) is replied to again
on a copy whose countdowns come due at the seat's next turn start, and the scores are blended at
`AI_REPLY.countdown_share` per turn the farthest still waits past its owner's next turn start (#1995);
with none waiting, nothing changes. A Dormant card (R1839000) acts in no line and threatens no face until
it wakes, but the evaluation counts it as the unit it wakes as, `slumber_turn_share` of it per turn left
on its countdown, or `slumber_waiting_share` with none (R2380000). A Unit standing in the backrow
(R1081000) counts once, as the Unit it is, never again as a backrow card (`standing_unit_priced_once`,
R2490000; off in `GREEDY_EVAL`). A Unit in The Gauntlet's lanes threatens no face while The Gauntlet
acts, since it may attack no hero: `face_threat` asks `attack_restriction`, as the lethal solver does
(`gauntlet_bars_face_attacks`, R1849005, #3287; off in `GREEDY_EVAL`). A unit its own Brittle tick
destroys at its controller's next start, before it can attack, threatens no face and makes no stand-down:
`face_threat` and `could_attack_next_turn` ask `crumbles_before_swing`, which reads the engine's
`brittle_due_by`; a side the evaluation reads as swinging first on its own turn keeps its units
(`brittle_crumbles_before_swing`, R3793000, #3793; off in `GREEDY_EVAL`). Nor does one the first tick of
its own Burn and Toxic destroys there: they ask `afflicted_before_swing` too, which reads the tick through
`tick_damage`, the reading `afflicted` prices it by, and keeps a unit whose Reborn body has Charge, or Rush
where only a Unit is the target, or that a Burn alone kills, its body back before its controller's turn (R4154000) (`affliction_kills_before_swing`, R4065000, #4065; off in `GREEDY_EVAL`). A
side the evaluation reads as swinging this turn (the seat at its turn start after the reply, the enemy on
turn in a passed line) counts no attack from a Frozen unit, though `frozen_next_turn` says it thaws at this
turn's cleanup, none from a unit summoning sick without Rush or Charge, and none on a hero from one with
Rush alone; `face_threat`'s end-of-turn read, `can_swing` and `unit_worth` are untouched
(`frozen_sick_before_swing`, R4170000, #4170; off in `GREEDY_EVAL`). A
hit from a source that exiles what it damages (M #72 The Banisher) reads as a Poisonous one in `blow`,
`kills_and_survives`, the Taunt walk and its Cleave splash, `breaks_alone` and `falls_to_first_strike`
(`exiles_what_it_damages`), and `trade_value` prices what it exiles at its whole `unit_worth`, with no Death
and no Reborn body (`exile_on_damage_read`, R4065001, #4065; off in `GREEDY_EVAL`). A unit that can only
attack the Unit directly across from it (S+ #37 Scorch) threatens no face either, and an attacker that Ignores Taunt
(S+ #121 Toxopid) is bound by no Taunt, Defense Position's included: `face_threat` asks
`attack_restriction` of the first, as of a Unit in The Gauntlet's lanes, and `damage_past_taunts_with`
leaves the second unbound (`static_attack_flags_read`, R3550001, R3560007, #3909; off in `GREEDY_EVAL`).
`damage_past_taunts`, which the lethal solver's `ready_gap` reads, takes `AI_EVAL`'s flag. Each Taunt is
walked as the opponent would attack it: the larger of two orders counts, smallest first and the smallest
attacker that destroys it alone first, and an attacker its Armor or First Strike would stop whole waits
until it falls, then goes on to the next Taunt or the hero (`best_attack_order_past_taunts`, R3937000,
#3937; off in `GREEDY_EVAL`). Every static Cleave splash (`cleaved_neighbours`, the walk's and the reply's)
is the engine's `query::cleave_struck` less the target: a GIGA beside the target from either of its lanes,
met once, and beside a Unit standing in the backrow only standing Units (R3546001, R1081000). Where such a
splash through a GIGA meets a later Taunt, the walk tries a third order, that Cleave attacker first
(`giga_cleave_read`, R4257000, #4257; off in `GREEDY_EVAL`, whose walk keeps the one-lane splash).
On a slashed board a Taunt binds only the attackers on its own half: the walk asks `attack_restriction`,
which refuses a Unit across the slash and lets a hero be attacked from either half, so a Taunt across the
slash walls nothing in `face_threat`, `ready_gap` or `wall_switches` (R3554003, #4725; no flag, so
`GREEDY_EVAL`'s walk reads it too).
Each hit in the walk takes the Taunt's strike back (§4.3), read through the §4.4 steps that can stop it as a
First Strike one is: an attacker it destroys still lands that hit but makes none of its later swings, a
Windfury one's second included, and the wounds and popped shield it leaves carry to the attacker's next
swing; a Ranged attacker takes none from a non-Ranged Taunt (R783000). A Frenzy attacker whose hit destroys
the Taunt it attacked, and that survives, has its swings back to `attacks_per_turn`, each with the hit's own
hero permission; an exiled Taunt and one a Cleave splash destroys free none (R3543003)
(`strike_back_survival_read`, R4402000, #4402; off in `GREEDY_EVAL`).
A Cleave splash in the walk lands on every unit it strikes. A unit without Taunt, or a Taunt the walk has
passed, is met as its earlier hits and splashes left it: gone once destroyed, or back as its Reborn body. Each
splash spends the pools in the engine's order, and a Trample attacker's sends what it deals beyond the unit's
health to the hero (§4.4 step 9) (`splash_trample_read`, R4805000, #4805; off in `GREEDY_EVAL`).
A hit on a Taunt, its Cleave splash and a First Strike Taunt's strike back are
each capped after Armor by the struck unit's own guard (C+ #11 Anime Armor, R2179005), as `blow` caps the
static trader's. Past the caps, every static read of a hit takes §4.4 steps 3a and 3b
(`pool_and_conversion_read`, R4064000, R4064001, #4064; off in `GREEDY_EVAL`). A hit from a source whose
damage Burns or is Toxic (S+ #153 Noxious Rounds; #151 Volcanic Rounds through v0.6.13h, whose hits since
issue #4783 land and also Burn, R4783001; `converts_to`) deals nothing, kills
nothing and heals nothing, though a Divine Shield still takes it: the walk's attacker waits at its Taunt, and
the reply prices the counter instead, on a unit by `counter_price` (what `afflicted` takes off it with that
much more on) and on a hero by `hero_ticks_added`. Each side's damage pools (S+ #42 Force Projector, #123
Oct) soak what they have left (`query::damage_pool_of`), one pool a side spent once across a read's hits in
the order it counts them (`Side.pool`, `Blow.soaked`, the walk's `WalkPools`): as they stand now for this
turn's attacks, refilled where that side's turn starts before the hits land. A unit's Death
counts by what it leaves behind (R2424000): at the root `death_prices` prices, within `AI_DEATH_PROBE_NODES`, each live
Death on the field and in the seat's hand (spawned first, R923001) as its controller's evaluation after
the sandbox's destroy less after its remove (R923003), once for each controller, so each side's copy reads
the price of its own board (R4024000); `unit_worth` adds `AI_EVAL.death` of it,
`kill_worth` takes all of it off a kill. A Starship Piece is priced the same way, by the copy its death
fuses into its owner's Starship, its probe after every Death's, so pieces never take a Death's nodes
(R2566000). A backrow card's Death counts the same way (R2687000): after the units, `death_prices`
probes each backrow card the seat can read on either side (never a face-down enemy card nor a Unit
standing there), then the hand's Units and Field Spells (set first), within the same budget, and
`material` adds `AI_EVAL.death` of it outside the Brittle share, a crumble being a destroy. A destroy is
read by the card having left the field (`left_field_after`), so a card its Death returns is priced.
A card's start-of-turn, end-of-turn and start-of-the-opponent's-turn hooks count
by one firing each (R2477000, R2565000, R2689000): at the root `turn_hook_prices` fires, within
`AI_TURN_HOOK_PROBE_NODES`, each such hook of the units on either field, of the backrow cards the seat can
read on either side (a face-down enemy card is never probed) and of the seat's hand's Field Spells and
Units (set or placed first, R923001) through the engine's trigger queue, and prices it as its controller's
evaluation after less before, each crystal gained at `next_turn_mana`, once for each controller (R4024000);
the first two count once for each
time its controller's turn runs them (`multipliers::extra_runs`, Joint Filing's extra runs, R821), read on
the state evaluated, and the third once; `material` adds `AI_EVAL.turn_hook` of it for a readable
backrow card, and `unit_worth` for a unit, so an enemy engine is worth more to answer and an own one more to keep.
A unit's Activate counts by one use (R2564000), and so does a face-up backrow card's, a Field Spell's
(R2603000): at the root `activation_prices` ends the seat's turn and then the enemy's, within
`AI_ACTIVATION_PROBE_NODES`; at the start of each player's next turn it tries that player's units' uses,
then its face-up backrow cards' (`activate_sources`), as the reply does (`visible_activation`), and
prices the best as the controller's evaluation after less before, each crystal at `next_turn_mana`;
`unit_worth` adds `AI_EVAL.activation` of it for both sides' units, and `material` for each readable
backrow card, inside its Brittle share. `activate_sources` lists each Dormant card acting for its
awakening alone too (R3575404, R1839002: a Dormant Starship's launch, and the `wakesBy` ability a hull's
text names, R3539000), and for that use alone the card need not be the instance left standing, since a
shipyard's launch exiles the hull and stands a fresh Starship in its place. `slumber_worth` counts
that awakening use in full: it puts back what the awake copy's own Activate does not count of the
sleeper's, so a Dormant Starship whose pieces print only keywords, which has its launch only while it
slumbers, counts the launch whole (R4799003). Then, within
`AI_HAND_ACTIVATION_PROBE_NODES` of its own, each Unit and Field Spell in the seat's own hand with an
ability, and each Dormant card of the seat's acting to awaken, is priced the same way and keyed by the
card as the seat holds it now (R3575400, R923001: a hand card is spawned where a play would put it
first), so a play worth the Activate it leaves reads it the moment it lands, not a turn later; the
enemy's hand is never probed. At the seat's own open prompt whose card acts on its side with an Activate
its face declares (#98 Heroic Power's Discover of its power, R3134002), `prompt_activation_prices`
simulates each answer once, in `action_key` order, within `AI_PROMPT_ACTIVATION_PROBE_NODES`, and prices
the power an answer leaves that card the same way; every price is keyed by the power its card holds
(`memory.power`, R43), so a lasting pick reads what each power keeps giving (R4197000). A shipyard's turn-start fuse (R3539004) is no hook of its script, so
`turn_hook_prices` prices it by one run of the engine's own yard step, for each Dormant shipyard on
either field and each in the seat's hand, and `slumber_worth` adds `AI_EVAL.turn_hook` of it outside the
slumber share, which that probe's own evaluation already applied (R3575402).
A unit's "when this attacks" and "after this attacks" hooks count by one attack (R2688000; Frenzy is
none, since the twin below holds it too, R4799002):
`attack_hook_prices` walks the same two turns, within `AI_ATTACK_HOOK_PROBE_NODES`; at the start of each
player's next turn it plays each of that player's units with a hook at the enemy hero, or at the first
target `legal_actions` lists, beside the same attack by the unit's hookless twin (Vanilla, granted its
printed keywords and every other keyword the unit holds, a lent one among them, R4811000; a unit already
Vanilla is its own twin; its text given back before scoring), and prices the controller's evaluation
after the one attack less after the other, each crystal gained at `next_turn_mana`; `unit_worth` adds
`AI_EVAL.attack_hook` of it for both sides' units, inside the Brittle share, while the unit may attack.
A Dormant card's "When this awakens" counts by its awakening (R3064000): `awaken_prices` wakes each Dormant
card with a wake trigger on either field, and each Unit with one in the seat's own hand, placed first
(R923001), on a sandbox copy within `AI_AWAKEN_PROBE_NODES`, beside the same wake of its hookless twin, and
prices the controller's evaluation after the one wake less after the other, each crystal gained at
`next_turn_mana`; `slumber_worth` adds `AI_EVAL.awaken` of it for both sides' sleepers, inside the slumber
share, and `GREEDY_EVAL` keeps 0.
An unevolved card's Evolve text counts by its evolution (R4648000): `evolve_prices` evolves each unit on
either field that can still evolve, and each such Unit in the seat's own hand, placed first (R923001), on a
sandbox copy within `AI_EVOLVE_PROBE_NODES`, and prices the controller's evaluation after it less before
it, each crystal gained at `next_turn_mana`, split over the runs its text took (R3536003). The same root
reads the card's road there (`EvolveCondition`) on throwaway copies: its "Evolves when" holding once its
side's empty unit zones are filled, once the seat's own hand gains (0) Cost cards, or once it is left on 1
health, and an event otherwise (its own trigger, a Stack, an enemy's hand). `unit_worth` adds
`AI_EVAL.evolve` of the price, once for each run its text would take now, times how close the card stands:
the share of its side's zones filled, of the free cards it needs that the hand holds, or of its health
lost, and `evolve_event_share` for an event, inside the Brittle share, for both sides' units. A card its
own turn or attack text evolves (the Wasp, Space Invader, Lantern Distiller) counts through that text's
price, the enemy's hand is never probed, and `GREEDY_EVAL` keeps 0.
A passed line's Float counts toward the seat's own "Float reduces cost" cards (R3695000): the engine's
`float_cuts_in_hand` reads what the Float took off each of them, and `float_credit` pays
`AI_EVAL.float_discount` for each crystal that went toward one still above the seat's crystals (its
`mana.max`, never the handicap's), in `passed_score` and `replied_score` alike; `GREEDY_EVAL` keeps 0.
An unconditional price hike on the coming turn (IA #123 Cost Injection, IA #144 Total Compromise, IA #225 Root Kit, C+ T-AI-7 Alignment Tax) counts in `coming_refreshes` as harm against that player's next-turn crystals (`coming_cost_hike`, R5122000): each crystal of the hike is priced at `next_turn_mana` times `AI_TAXED_PLAYS` (1), weighted by `cost_hike` in `AI_EVAL` (1.0), and preserved across reply scoring in `replied_score`; `GREEDY_EVAL` keeps 0.
A Nerf on a card in the enemy's hand counts against that player in `material` at `enemy_hand_nerf` (0.5) a step
(`hand_nerfs`, R5246000): a crystal its `costMod` made it dearer, a `TUNE_STAT_TOTAL` of stats below its face, a
keyword lost or a harmful one gained, a step a number moved the worse way, never a secret Nerf (R1845000) nor what
an Upgrade wrote. Each live `RandomTargetsNextSpell` on a side (IA #130 Prompt Injection) counts against it at
`random_targets` (1.0) (`coming_random_targets`), beside `coming_refreshes` rather than in it, so a reply that
spends one keeps no credit. `GREEDY_EVAL` keeps both at 0.
A card beneath one of the seat's own Stack piles counts `AI_EVAL.dormant_share`, a half, of its
`unit_worth` (`dormant_worth`, R3695001), so the Drone Battleship counts its two lower layers; the
enemy's count `dormant_card` each, and `GREEDY_EVAL` keeps 0.
A card in either graveyard that its owner may still play from there (`playable_from_graveyard`: its own
Flashback, R783012, Sanctum's lent one, R1869003, or a field permission such as Second Wind's, R454)
counts `AI_EVAL.graveyard_playable` of its hand price, the enemy's too, since a graveyard is public; a
spent Flashback counts nothing (R783013). `keyword.flashback` stays 0, so the play is priced once, and
`GREEDY_EVAL` keeps 0 (#2928).
Burn and Toxic count by the ticks still to come over `AI_AFFLICTION_TICKS` of their controller's turn starts
(R3822000, R3540001). `afflicted` takes `AI_EVAL.unit_affliction` of a `health` point off a unit for each
point they will deal it, up to its health, and a Divine Shield they spend costs that share of its keyword
worth; Armor meets Burn, Toxic pierces it, the unit's own cap holds both, and an Indestructible unit takes
none. A unit the nth tick kills keeps `1 − unit_doomed / n` of itself, or its Brittle share if that is
smaller. `coming_hero_ticks` reads a hero's ticks through `projected_hero_damage` and its side's refilled
pools (R4064000), as `afflicted` reads a unit's, leaving it at least 1 life, and `hero_side` loses `hero_affliction` of that life's cost on the hero curve; the enemy's also counts
at `enemy_health`. The reply already plays one tick a side. `GREEDY_EVAL` keeps all three at 0, and the
learned features leave both counters out until a refit (R3535002).
While a Feng Shui acts on either side (Meditative #40, R984), the seat's next face-up play counts by the
verdict its last element gives it (R3951100): `feng_shui_outlook` takes, of the cards in the seat's own hand
a Feng Shui would judge, the best verdict `subsystems::feng_shui::verdicts` gives one now, a reward at what
the card would gain in hand as Radiant (`hand_card_value` of its Radiant copy less its own) and a punishment
at every judge's hit on the hero curve, life held at 1 or more, and `material` adds `AI_EVAL.feng_shui_outlook`
of it. The best alone, since only the first play is judged by that element, so a hand of rewarded cards is no
reason to hold them back; the turn's own verdicts the search already plays through `reduce`. In tier 2 of the
move order a play its card would be rewarded for comes first and one it would be punished for last, each group
by cost (`Auspice`), so a level's `branching` cut keeps the play that chains the elements, and the seat's last
element is part of `search_signature`, so two orders of the same plays that end on different elements stay
two positions; with no Feng Shui acting neither changes. `GREEDY_EVAL` keeps 0.
While a Feng Shui of the seat's own acts, its Activate (a free Auspicious Rock) is tier 2's first source,
before the rewarded plays (`Auspice::Judge`, R4154403), so a busy turn's `branching` cut never drops it.
While a Feng Shui of the seat's own acts, `feng_shui_outlook` also counts the Auspicious Rock its next turn's
Activate adds (`coming_rock`, `AI_FENG_SHUI_ROCK`, Radiant on a Radiant Feng Shui) among the cards the next
play may be, so a hand the element punishes costs nothing while that Rock is not punished, and a play no
Feng Shui judges whose element generates another hand card's is a bridge (`Auspice::Bridge`), tried after
the rewarded plays and before the other plain ones (R4783041).
A Tradable card in the seat's own hand whose own cost is above the crystals its next refresh brings (this
turn's `mana.max`, one more while its crystal count under the cap, `mana.max` less `perm_mod` and the crystals
stolen past the cap, is under `MAX_MANA` (R4812000), moved by the rider; never the handicap) is dead
(`dead_in_hand`): `material` holds it at `AI_EVAL.dead_tradable_discount` off its hand price, and its Trade
is tried before the plays, so a line can play what it draws (R4154400). S #89 Spice in the seat's own
hand is priced as any (1) card, and the Credits it pays as it is played or discarded count at
`market_credit` once gained, as a Resource's do (R4410001). A permanent a Living Bomb (`AI_PLAGUE_BOMBS`) will destroy at its controller's next
turn start (`plague_doomed`: a Plague Counter, not Indestructible, and the bomb's hook that fires then
registered where the engine's `trigger_holders_with_hook` finds it) keeps Brittle 1's share, its counters
banking nothing, and a placement prompt under one tries the enemy's permanents first, every answer when
auto-answered (`plague_aims_first`, R4154402). `GREEDY_EVAL` keeps all three off (its `brittle_discount`
is 0).
It re-plans after every action, so a
lethal the solver proves is played trimmed to the actions it needs
(`trim_lethal`: its shortest suffix that still wins on every determinization, then less each other action
it still wins without, R1965000; none, when the budget or the clock stops the trim before it has that
suffix). A free play (a (0) Spell back from the graveyard under In Too Deep's reward L, R454, or The Coin)
could otherwise open the same lethal at every decision. Each line tries its moves in move order
(`candidates.rs`): attacks on the enemy hero, kills the attacker survives and each attacker's best kill
(its target, or with Cleave a neighbour, #1821) at an even trade or better by the opponent model's own
read (`reply.rs`'s `trade_value`, #1779), these three ranked together by that model's values, best
first, ties in that order (#2825): a kill by `trade_value`, a hit on the hero by `face_value`
(`reply_face_value` of what it deals through the hero's Armor, plus a Lifesteal attacker's heal of it
unless the hit is lethal, #3858), and every hit on the hero first while
the seat's ready attacks reach the face together (`ready_gap`); then the Trades of dead Tradable cards (R4154400), the plays round-robin, answers, the
other Trades of Tradable hand cards (R1838003: on a determinized copy, so the card one would draw never steers it), the
buys at the Marketplace (R1853004), the other attacks, position switches and End Turn; the root expands its first `root_branching` and each deeper step
its first `branching`, End Turn always among them. Inside one card's plays, each way of being played (its
X, Embiggen, Equip, face-down set, Tribute set) gets its first variant before any way gets a second lane or
target, the dearest way first, then the cheapest, then the rest dearest first, Tribute sets cheapest units
first (#1778). Its root tries a play in every distinct placement (the zones beside its lane and
across it, a zone a GIGA covers read as the GIGA, and the half of a slashed board its lane is on,
R4799001), each in its lowest lane, or its midlane lane when it has one, which a Cry may read (R2203001);
deeper levels, the lethal solver and the reply try the first `AI_SEARCH.zone_variants` (#1635). The beam
drops an open line whose position an earlier one reached (two orders of the same moves into the same lanes
are one position). A position also holds the last Spell played while a card that copies it (C #57 Echo) is
in the seat's hand or graveyard, and each unit's Churnwalker chain; its units' lanes include a Dormant card's
lane and an animated card's home zone (R4808000). A card off the field signs its buffs and granted
keywords, each only when it holds some and the keywords sorted (`grants_signature`,
`AI_SEARCH.hand_grants_signed`), so which hand card an S+ #100 Plesiosize or an S+ #23 GIGA-ify went on is
part of the position, for the beam, the lethal solver and the concession check's walk (R4793000). A line that differs from an earlier one only in where its cards stand is a lane variant
(#1797; R4132000: a backrow card's lane, and which side of a standing unit a new one goes, included, read off
`lane_free_signature`, which sorts each side's units and backrow cards and leaves out its empty zones): the
evaluation reads no lane but through a unit's stats, which that signature keeps, so it takes no place in the
beam, but when the beam keeps the earlier line, the first `AI_SEARCH.zone_variants` - 1 of them end their
turn where they stand after it, below that line on a tie. So the placements the beam tries reach the reply,
which prices a Cleave into a unit's neighbours; a lane the reply scores the same as a better one of that
play is not a finalist of its own. The
lethal solver's visited set reads a position with its units' lanes, as the concession check's walk does
(`simulate::position_key`, R4806000): a play tried in two lanes reaches two positions, so a lethal that
stands only in the second (across a slash, outside The Gauntlet) is found. Position
switches are the last tier of the move order, so a level's `branching` cut drops them from a busy board;
the first `AI_SEARCH.position_switches` stand-downs (Attack to Defense of a unit an enemy unit that could attack it next turn would kill and survive, `kills_and_survives`; R2021000) it cuts are
kept (#1759), so a unit that cannot win its next fight can stand down however many earlier moves the board offers. While the
opponent's board threatens lethal (`face_threat` reaching the seat's `effective_life`, less the pool's coming
loss in a Duos duel, R4188000, where the evaluation's `lethal_threat` applies), up to `AI_SEARCH.position_switches` more switches to Defense Position that lower that
threat are kept after them, stand-downs or not (`wall_switches_priced`: each tried on a copy of the state with the unit's
position set, the least threat left first; #2976, R2976000). An action the
true state refuses (a cost no determinization could see) is never submitted: `submitted` sends the
first action of the next finalist the true state accepts, best mean first, with the search's reason, and
only when none is accepted falls back (`DecisionReason::Fallback`) to `replacements_for`'s endTurn, answer
or other legal action (#1575).

R4792000: while a Mayor acts (R1200) `legal_actions` lists one attack per attacker, naming
`RANDOM_ATTACK_TARGET`, and the engine draws its target. Such an attack is worth the mean of a hit on each
target it may draw (`candidates::drawn_attack`: `attack_targets`, each as likely, a hit on the hero by
`face_value` and on a unit by `trade_value`), in the reply's `ranked_attacks` and the move order's
`front_value` alike. The move order puts it with the attacks on the hero when the enemy hero is its only
target, otherwise with the even trades when some draw hits the hero or kills and the mean is 0 or more
(`Drawn::even`), otherwise among the other attacks. `ready_gap` counts it only when the hero is its only
target (`draws_only_hero`), and only then does `face_is_lethal` make its hit the win. A Radiant Mayor's
Lucky (R1201) is not read. `AI_REPLY.drawn_attacks` off prices it at nothing and counts it on the hero,
as before.

`decide_styled` is `decide` under an `AiStyle` (R1452), the server's ladder bots' strength knob beside
their budget: a `missed_lethal_chance` to pass over the lethal line the solver found (the search then
keeps off every line that wins this turn, `lines_missing_lethal`, or it would find the same lethal),
and a softmax
`temperature`, in evaluation points, over the finalists' means (`pick_finalist`) instead of always the
best. Each takes one draw from `options.rng` when it is above zero and none at zero, so `decide`,
which is `decide_styled` at `AI_STYLE_EXACT`, draws only for its determinizations, as it always has.

R4608000: a finalist whose line won on determinization 0, the one searched, and on no other scored world
(a score above `AI_WON_SCORE_SHARE` of `AI_EVAL.win`) takes its mean on those others (`finalist_means`).
Every other finalist keeps the plain mean, added in world order.
The solver, the hint and the concession check never read the means.

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
what shows: the opponent's hand, face-down cards and unrevealed secrets are taken out, so no unseen
trap stops the seat's answer and no unseen card or guessed secret makes the lethal (a secret's reward
finds no choice and does nothing, R4796000). It fills the rest in as
`UNSTOPPABLE_LETHAL_DETERMINIZATIONS` worlds from `UNSTOPPABLE_LETHAL_SEED`'s stream and, on each,
plays every line of the seat's turn through the engine (every legal action but R84's and the
mulligan, "end turn" first, a position once), and for each line that hands the turn over asks
`find_lethal` for the opponent's lethal on the board alone, the card it drew dropped, then plays that
line again. It holds only when every line on every world ends lost. Every doubt answers false: a cut
walk (`UNSTOPPABLE_LETHAL_NODES`, `should_stop`), a failed step, a step that rolls the game's random
numbers or moves a card of the seat's library (one outcome of a roll, one guessed order, could hide
the escape), a step that returns the board to a snapshot (its `rolledBack`: the worlds' board history
is one stand-in snapshot, the field as it shows, for the one `redact` drops, so a seat that can play a
Rollback plays on, R4796000), an opponent's lethal that rolls, moves a card of its own library or opens a prompt for
the seat, a panic, and every moment but the seat's own main phase with no prompt open. A turn that ends with the
Marketplace open ends without it, so the opponent's turn start rolls no offers (R1853000), and a lethal
that needs the opponent's turn to end is not counted: both err only towards playing on. `decide` never
calls it; the ladder bots ask it before conceding.

## Budgets

A decision's cost is counted in nodes: one node is one `reduce` call the AI makes, the reply's
included, or one turn hook its root fires (R2477000, R2565000). A step whose `reduce` does far more work than most is charged extra nodes up front
(`surcharge_nodes`: a play that ranks every shipped card, an attack an armed My Pawn would answer, one
its own projection calls lethal (`subsystems::is_lethal`, R4809001), a fused card holding either, and a
play the other side's Pareto Optimality judges, per card held; #1240),
and once a step has run, for each such card it cast without playing it, from a graveyard or at random
(`announced_surcharge_nodes`, read off its `cardAnnounced` events; #1347).
A step whose surcharge the counter cannot pay is refused in the beam (`simulate` answers `Some(Err)` and
the beam tries the next), but cuts the opponent's reply (`simulate_reply` answers `None`, as for a spent
counter: `reduce_for`, `best_try`), since a skipped reply step reads as an opponent who passed it up
(R4809000). The lethal solver's first walk, depth-first on `AI_SEARCH.lethal_quick_nodes`, skips such a
step too (`simulate`'s refusal is `UNPAID_SURCHARGE`), and a line whose replay on another
determinization meets one, but then ends cut rather than searched, so the best-first walk still runs on
the rest of the solver's allowance (R4807000).
Such a step, once run, is kept for the rest of its decision (`SimMemo` on the root counter, at most
`AI_SEARCH.memo_steps`), and the same step from the same state, nonce records aside, is answered from
it without the engine, its nonce record rebuilt as the engine builds it and its nodes charged as before,
so the search and its decision are unchanged and only the time it takes falls (R3575603).
A prompt answered ahead inside a simulation (`best_auto_answer`: the other seat's in a step, the
reply's, and the beam's look past the seat's own; #891, #1112), but the seat's own question it works
out (`worked_step`, #2929), tries at most
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
which ends the search early with the best line found. R3498000: every host with a clock also passes
`wrap_up`, fired at `AI_WRAP_UP_SHARE` of its limit (`wrap_up_after_ms`), which stops the beam's slice
of the counter and skips the payoff line R3696000 adds: the beam's open lines end their turn and the reply and the other determinizations
run on until `should_stop`. R4801000: it is first polled once the beam has tried every move at its
root (`NodeCounter::end_root`), however many nodes those cost (a surcharged play, its auto-answers),
so a wrap-up that fired before the beam still leaves it lines, the root's End turn among them.
R4604014: so is the slice's own limit (`create_beam_counter`): the root's moves draw on the decision's
counter however many nodes they cost, so a root move dearer than the whole slice still closes a line;
the decision's own limit and clock still stop it.
R4791000: a beam whose slice refused a node, to its budget or the wrap-up, closes on its stop only the
frontier line the stop cut short, those after it and the children (a line it finished expanding filed its
End turn already), and ranks a line whose End turn was refused, open in the seat's main phase, as a passed
one, its hand eval less `AI_EVAL.unspent_mana` a crystal (`closed_score`); `decide` then ends the turn of
each such shortlisted line on the decision's own nodes before `reply_score`, and a node refused there ends
the replies. A beam its slice never refused decides node for node as before (`AI_SEARCH.close_cut_lines`).
`SearchStats.replied` and `worldsScored` say how far they got, and
`Decision::fell_short` names a decision that left its finalists unscored on a sampled world. With no
clock, or a wrap-up that never fires, a decision is node for node what it was.

R4788000: on a decision whose budget was scaled down (a width past `AI_SEARCH.fat_reference`, R2575000),
the lethal solver has a reach, the caller's own `lethal_nodes` (at most its `nodes`, R4802000): its walks
spend the scaled allowance as before, and past it the best-first walk goes on only from a position whose
ready attacks already reach
lethal (`ready_gap` at or under 0), and the proof on the other determinizations of a line that won on the
first is always finished (`find_lethal_reaching`). What the walk spends past the scaled allowance is added
to the searched section, so the beam and the replies keep their nodes, and no decision passes its caller's
`nodes`. A state of ordinary width, and a wide one whose walk never reads as lethal, decide node for node
as before; `lethal_hint` and `unstoppable_lethal` call `find_lethal` and have no reach;
`AI_SEARCH.lethal_reach` off stops every walk at its scaled allowance.
R5119000: among open positions of the same gap above 0 and the same depth, the best-first walk takes
first the one whose ready attacks leave the enemy hero least were no Taunt standing (`Gaps::taunt_free`,
read through an open prompt, whose answer spends no attack; `AI_SEARCH.lethal_keep_attacks`), and once
it has tried `AI_SEARCH.lethal_dive_moves` (75) moves at and below one child, from positions no closer
than that child's parent, it sets the child aside: the open positions below it come after the others of
their gap. Both off are the walk before it, node for node.

R4447000: `decide` replays each finalist's world-0 line on the other determinizations with
`replay_line`, which says whether the line was cut there: whether it met a step that world does not
offer (a card the line drew is another card there, since each world shuffles the seat's own library,
or a random effect rolled otherwise), an answer to the seat's own prompt (#1636) aside.
`SearchStats.replaysCut` counts those cuts over the worlds every finalist was scored on. With
`AI_SEARCH.replay_fill_steps` above 0, a cut replay drops the rest of its line and plays that world's
turn on greedily (`fill_turn`: up to that many steps, each the best by `static_score` of the first
`branching` candidates that are not endTurn, taken only when it beats standing still, never one that
leaves the seat's own prompt open), on a slice of `steps × (branching + 1)` nodes, which `decide` keeps
back beside each other world's reserve. It ships at 3 (R4697000), so `decide` keeps up to 126 more of
`AI_BUDGET`'s 1,200 nodes back from the beam (21 for each of three finalists on two other worlds); at 0 a
cut replay ends its turn where it stands and a decision is node for node what it was before R4447000.

## Decks and the shadow ban (R184, R186, R390, R1370, R4784000, R1390, R817000, R976000)

`build_ai_deck(rng, size, &AiDeckOptions)` deals `size` distinct non-token ids from every set by
weighted sampling without replacement (`AI_DECK`), leaving out `SHADOW_BAN_IDS` unless `banned` says
otherwise. `lean_set` (R1370) names a set at least `AI_DECK.lean_min_share` of the deck comes from, a
hard floor kept together with the units' and the theme's; absent, the draw is the one it always was,
except that a card which would leave the theme's and the units' floors out of reach together is drawn
again from the same roll (R4784000), so only a deck that lost the units' floor deals differently.
`build_ai_deck_traced` also counts the picks that floor forced, the measurement `lean_boost` was
chosen by. Every deck holds at most `AI_DECK.max_quickdraw` (2) Quickdraw cards, an `include`d one
counted and always dealt, and Quickdraw is never a theme (R817000). `gated_sets` (R1390) names the sets the draw never deals from: its default is the soft gate
`AI_DECK_GATED_SETS` in `src/config.rs`, so every AI deck holds none of them whatever ships or a
thread previews, and a lean on a gated set leans on the newest ungated set that ships (`ungated_lean`).
A player's random deck passes `Some(vec![])` beside `banned: Some(vec![])`; an `include` still forces a
gated card in by name. Emptying the list lifts the gate and deals every deck as before it, which issue
#732 did once it had measured the AI on the Meditative set, the list's one set until then. It held the
Stinkyscape set from its release (issue #1884) until issue #1885 measured the AI on it: gating a set that
ships before the AI is trained on it is listing it there. It held the Stinkyscape+ set from its release
(issue #3573) until fullsend 2's sweep of record measured the AI on it (issue #3574, R3574000), and is
empty since. No deal is shaped: the unban lane's dealt-quality table and the option that read it are
removed (R4702001), so practice, the quality gates and both seats of an arena deal by one rule (R976000).
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
set held to its own half (R1186000), and since #1885 Stinkyscape cards, at most a fifth of them
(R1885000), and since #2203 Training Grounds cards, at most half of them (R2203000), each set's share
written in its row of the set table and its ceiling that share of its non-token cards (R3114001): no
sweep, lane or person adds a card of an
earlier set, and the unban lane, retired by #4701, only ever removed entries.

**When it is swept** (R1678000): every push to `main` that moves `generation.json`'s `generation`
(every promotion writes it) or adds a set to `SHIPPED_SETS` starts a sweep of record on GitHub's
runners, `.github/workflows/sweep.yml`. `cargo jackioh sweep --due <base>` names the cards: the bans and
the watched cards of the sets the ban may hold when the AI changed, every non-token card of a set that
ships now and did not. Its report goes under `training/history/`, `--write` reads it last, and a
changed table arrives as a `Patch: the shadow ban swept again after …` pull request with auto-merge
on, `generation.json`'s `shadowBan` and the practice-claim fixture with it. So the sweep after a
promotion may ban a card of those four sets again when the new AI is slow with it, within each
set's ceiling; a record `--write` refuses opens an issue for a person instead. A change that retires or replaces part of what the generation names (its evaluation or its search) moves the generation by hand, in lane `hand`, under an issue and a ruling of its own and with no gate numbers (generation 14, R4921000; `training/README.md`, Reading the history); an ordinary fix to the AI keeps the generation.

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
needs `gate_needed(matchup, n)` wins (the rule spec §9.9 proposes); only wins count. Hard against
Easy's floor is 75%, below the brief's 80%, by the user's decision (R4132001).

| Matchup | Subject | Opponent | Full run | Smoke run |
| --- | --- | --- | --- | --- |
| `ai-vs-random` | the AI on Easy | random policy on Easy | 95 of 100 | 19 of 20 |
| `ai-vs-greedy` | the AI on Easy | greedy baseline on Easy | 35 of 50 | 14 of 20 |
| `hard-vs-easy` | the AI on Hard | the AI on Easy | 38 of 50 | 15 of 20 |

The perf gate times every decision of one ai-vs-greedy game in a smoke run and six in a full run;
both also time two hand-built wide boards. It holds them within the node budget and under
`AI_GATE.max_decision_ms`, and all three pinned fixtures (`tests/fixtures/pinned-*.json`, #1240)
under `AI_GATE.pinned_decision_ms`. Tuning changes the weights in `src/config.rs`, never the floors,
the seed counts or the brief's rates. The gates see only a broken AI; whether a change made it
stronger is a screen's question (`training/improve.md`, How you measure).

Every gate game is clean (B31): nothing refused or thrown, no fallback, a result, a log that folds to
its hash, and no panic caught in an AI's search (`SearchStats.sim_panics`, summed in `MatchRecord`,
#1429). In the browser such a panic is a WebAssembly trap that loses the whole decision. A failure
names the seed, the seat and the first message (`Decision::first_sim_panic`, native only).

A seat the AI plays has a skill (R747000): `SeatController::Ai { budget, skill }`, the chance in
`[0, 1]` that each decision follows the AI, the rest being the skill miss (`skill::skill_miss_action`,
R2214000): a softmax at `AI_SKILL_MISS_TEMPERATURE` over the greedy baseline's one-ply scores of the
candidates the probe world lists (`decide::probe_world` of `redact(state, seat)`, as `decide` lists
its own, R4216000), read on one determinization of what the seat sees at the shown costs (R762). The
greedy baseline and the miss simulate each candidate on a counter of its own, sized to its one node, its
surcharge and `max_auto_answers` answers (`one_ply_step`, R4799000), so a surcharged play (a Zephrys, a
play Pareto Optimality judges) is simulated whatever the candidates before it took.
`None` is `AI_SKILL_MAX`: full skill, which takes no draw, so a seat without one plays as it did
before the knob; `AI_SKILL_MIN` is the miss at every decision, with no search. In
between, `skill::plays_best(skill, rng)` rolls the seat's own seeded stream once per decision
(`skill::clamp_skill` pins the range, a NaN reading as full), so a seeded run reproduces. The knob
knows nothing of cards or sets: it keeps meaning the same when the AI improves or a set ships. The
arena (`crates/tools/README.md`) and the server's ladder-bot seat (`crates/server/README.md`) roll it
the same way, and the skill-to-Elo curve is measured with the arena.

`MatchConfig.setup` (`MatchSetup`: `dealt`, `lastBoards`, `glitchBoards`, `deckMarks`, written beside
the config's other keys) carries the setup inputs `create_game` takes beyond the decks and handicaps
(R433, R417, R564, R678, R2702000), and `MatchConfig::fold_args` folds a record with all of them
(#1697). Unset, a game is created, played and folded exactly as before. No gate sets it.
`SweepOptions.face` (`SweepFace`, base when absent) picks the face the forced card is dealt on: the
Radiant one sets a `deckMarks` entry for it on the AI's seat (R2702000), and `cargo jackioh sweep`
plays both (#4466). `cargo jackioh run-sim` sets `deckMarks` for a dungeon run's fight (#3065).

## Development runs for the card statistics (R378)

`dev_game_config` and `dev_game_record` deal and play game n of a development run as All Random
deals a live game (R258), both seats AI on the spec's resources at `AI_BUDGET`, filed as a game
record of source `dev` under the patch the run tests, id `dev:<patch>:<seed>` (`dev_record_id`).
`cargo jackioh stats --out <file>` writes a run; `jackioh-server stats-import <file>` loads it.
`forced_dev_game_config`, which `DevRunOptions.force` selects, deals the same game with one card in one
seat's deck, p1's in odd games and p2's in even ones (R1284000). Both decks are dealt with the card
banned, and the holder's card at the slot `${seed}:force` picks is replaced by it.
`dev_run_config` deals game n of the run its options describe, so a record stays a pure function of
its number.

R4552000, R4552002: `swing_turns` (`src/swing_turns.rs`) replays a finished game's log from the engine's
own start and steps (`recorded_start`, `read_steps`, as `summarize_game` does) and writes, at the start of
every turn, one row for its seat: the turn counter, `evaluate` of the seat (`NextSwing::Seat`) less its
opponent's (`NextSwing::Enemy`) in thousandths, read off the true state, and the cards acting on each
field, a Created card with the card that made it (none when a hand held it; a copy an effect cast names
the card that cast it). `attach_swing_turns` files the rows in the record (`SeatSummary.swing_turns`).
The writers read the summary and the rows off one fold of the log (#2213), `summarize_with_swing_turns`
(the engine's `summarize_game_with` with a `SwingReader` beside it): `dev_game_record` and the
server's record filing (`crates/server/src/api/game_records.rs`) both call it.
Decisiveness is read off them (`crates/engine/src/wire/stats/swing.rs`).

## The learned evaluation and the policy prior (retired)

The learned evaluation (#466, #1010) and the policy prior (#1171) are retired (#4159, R4159000): the
search scores every line and finalist with the hand eval (`evaluate_priced` over `AI_EVAL`) and expands
each line's moves in `candidate_actions`' order. The crate has no build script, compiles no weights and
extracts no features (#4702, R4702000). `git show d40d3a45:docs/learned-eval.md` is the record.

## Duos (§13.7)

`src/duos.rs` plays a seat of a Duos game (`jackioh_engine::duos`): one AI plays both seats of team B,
and any human seat the server hands it after a disconnect (§13.8).

```rust
pub fn duos_decide(state: &DuosState, seat: DuosSeat, options: &mut AiOptions) -> Option<DuosActionBody>;
pub fn duos_decision(state: &DuosState, seat: DuosSeat, options: &mut AiOptions) -> Option<DuosDecision>;   // { body, fallback, sim_panics, first_sim_panic }
pub fn duos_passes(state: &DuosState, seat: DuosSeat) -> Vec<String>;
pub fn duos_pool_loss(state: &DuosState, seat: DuosSeat) -> i32;   // R4188000
pub fn duos_gate_game(n: usize, series: &str, opponent: DuosOpponent) -> DuosGateGame;
pub fn duos_gate_report(opponent: DuosOpponent, played: &[DuosGateGame]) -> DuosGateReport;   // { opponent, games, wins, draws, rejected, fallbacks, sim_panics }
pub fn duos_greedy_action(state: &DuosState, seat: DuosSeat, rng: &mut Rng) -> Option<DuosActionBody>;
```

- `duos_decide` is `decide` on the seat's own duel (`decide_with_pool_loss`, given `duos_pool_loss`): its mulligan and its prompts as a 1v1 seat answers
  them, and its own turn by the same search. When `decide` would end the turn, it first sends the
  passes `duos_passes` names, one action at a time, then the `endTurn`. `None` when the seat owes
  nothing. The same state and options give the same action.
- `duos_decision` is `duos_decide` with where its action came from: `fallback` is true when the
  wrapper's own fallback picked it (`decide` answered nothing the seat may send) or `decide` itself
  fell back (`DecisionReason::Fallback`); `sim_panics` and `first_sim_panic` are the panics `decide`'s
  search caught and the first one's message (#1429).
- `duos_passes` names the hand cards to pass now, best first: a card its teammate can play this
  turn whether or not the seat can play it itself (a round seat that has not ended, can afford it
  and has a place for it: an open zone of its row or, failing that, any play of it the engine
  lists, onto a Stack, into the backrow with Flying, onto an Equip host or a stack base, or into a
  zone its Tribute frees), when the seat can pay the pass; with `pass_cost` 0 also a card the
  teammate can afford next turn and the seat cannot, each side's next refresh read as its turn's
  start gives it, a "+N mana per turn" (R4440052) and a credit line's instalment (R1224) counted;
  never more than the passes left or the teammate's free hand slots. Immediate passes rank by the teammate's best bounded follow-up
  (R1127000; at most `DUOS_PASS_MAX_FOLLOWUPS` plays on a fixed-seed probe, past it the first plays
  of the teammate's best-valued hand cards, R4932000), ties by cost then hand order; free-pass
  next-turn cards keep cost order. An immediate pass is named only when it is
  worth it (R1723000): the teammate's best follow-up with the card, also tried after the teammate's
  own best play (each a real engine step under a nonce of its own, a step that changed nothing
  skipped), beats its best without it by more than the card's `hand_card_value` in the seat's
  hand, `DUOS_PASS_MARGIN_PER_COST` per crystal of `pass_cost` and `DUOS_PASS_MIN_GAIN`. A card the
  seat can play itself is named only when that gain also beats what the seat's own best bounded play
  of the card gains it in its own duel (R2827000): gain against gain, never one duel's score against
  the other's.
- It knows what its team knows and nothing more: its own duel through `decide`'s `redact` and
  `determinize` (R185), and its teammate's own state and duel as the teammate sees them (`redact` from
  the teammate's seat), which a team shares (§13.6). An opponent's
  hidden card never changes a decision.
- A seat's life is the engine's `effective_life` (own HP and the team's pool while unspent, §13.4)
  everywhere the AI reads one: the evaluation, `ready_gap`, the reply's `reply_face_value` and the My
  Pawn surcharge. The hero value and `closing` are anchored at `starting_life` (`DUOS_OWN_HEALTH` plus
  the pool's cap). A 1v1 seat reads its hero's health and `HERO_HEALTH`, as before (R964000).
- `duos_pool_loss` is what the enemy board in the teammate's duel is about to take off the pool next
  round (R4188000): `face_threat` on that duel redacted for the teammate, less the teammate's own HP
  while unspent, at least 0, at most the pool, and 0 unless that duel is on the teammate's turn with no
  extra turn owed to it (R846). `decide_with_pool_loss` carries it in `Prices::pool_loss`, keyed to the
  deciding seat (`Prices::pool_loss_of`), and only two places subtract it from the seat's life: the
  evaluation's threat term and `wall_switches_priced`'s gate. The lethal solver, the hero value, the
  race, `closing` and the reply read the whole life, and a 1v1 decision carries no loss.
- The tiers are `duos_ai_team(difficulty)` for the team (Easy the humans' rules, Medium free passing,
  Hard free passing and a `DUOS_POOL_HEALTH_HARD` pool) and `AI_DIFFICULTY` for each AI seat's
  handicap, which the caller sets in `DuosCreateOptions`; nothing here reads a difficulty. The AI
  never concedes (R188), so it neither offers nor confirms a team's concede.
- `duos_gate_game` plays game n of the Easy AI team against one `DuosOpponent`: a team on §10.7's random policy
  (seeds `${series}:${n}`) or a team on the 1v1 gates' greedy baseline (#1760): `duos_greedy_action`
  plays each seat's own duel with `greedy_action`, never passes, and draws only on its seat's stream
  (seeds `${series}:greedy:${n}`). A matchup passes at its opponent's floor, `DUOS_GATE_WIN_FLOOR`
  (0.8) against random and `DUOS_GATE_GREEDY_WIN_FLOOR` (0.7) against greedy, of
  `DUOS_GATE_GAMES_SMOKE` (20) games with no action of either team refused, no AI action off a
  fallback path and no panic an AI seat's search caught (`cargo jackioh gate --duos` plays both);
  `duos_gate_game_with` plays a game at another budget or with a stand-in decider (`DuosGateDecider`)
  for the AI team, which the tests use to show the gate catching a weakened pass.

## Commands

```
cargo test -p jackioh-ai                      # every AI test (the unit tests, tests/ai.rs, ai_slow.rs) but the ignored ones
cargo test -p jackioh-ai -- --include-ignored # and the daily ones: the many-game sweeps, `#[ignore = "daily: …"]`
cargo test -p jackioh-ai --test ai_slow       # one binary: CI gives each its own job (#873); the fast tests per PR (`rust (test ai slow)`), the many-game and sweep tests are `#[ignore = "daily: …"]` (#4093), run by `-- --ignored`
cargo jackioh gate                            # the three gates and the perf gate at smoke size
cargo jackioh gate --full                     # at full size (the daily super run plays it in shards)
cargo jackioh gate --duos                     # the Duos gate: the Easy AI team against a random team and a greedy team (--games N)
cargo jackioh gate --measure                  # measure the AI's win rates across all matchups on fresh tuning deals (--games N)
cargo jackioh gate --full --shard 1/4 --out ai-gate-shards && cargo jackioh gate merge ai-gate-shards
cargo jackioh trace ai-vs-greedy 3            # one gate game, turn by turn
cargo jackioh sweep core-011 classic-020      # the shadow-ban sweep over these ids (no ids: every card)
cargo jackioh stats --games 50 --from 51 --out dev.jsonl   # a slice of a development run
```
