# Felinors vs Humans: the plan of record

2026-11-28 · written from the designer's list of the same day (`JackiOh_FelinorsVsHumans_Cards.md`), tracker #5220

> **Status: the plan of record for the build.** SPEC is still the only source of rules (CLAUDE.md). This
> file reads the designer's list against the engine on `main`, freezes the engine surface the new cards
> code against, gives the reading the build takes wherever the list is silent or ambiguous, and splits the
> work into the three waves the owner asked for:
>
> 1. **Wave 1 (#5222):** the engine's new mechanics and every card. The set ships, players can play it, and
>    the AI deals itself none of it (R1390's soft gate). Its release turns the set's row `Shipped` and puts
>    the set in `AI_DECK_GATED_SETS` (R5222000), with the 63 card intros (R5222001).
> 2. **Wave 2 (#5223):** the AI deals itself at least 80% of the set: a sweep of record bans at most 1/5 of
>    its 197 non-token cards (39).
> 3. **Wave 3 (#5224):** the AI deals itself the whole set: no FvH card in `SHADOW_BAN` or any AI exclusion.
>
> Each wave is built in a Claude session with the fullsend skill and subagents (the owner's call of
> 2026-11-28), lands on `main` in one pull request, and is promoted to staging by hand. A part that builds
> a card or mechanic may change a reading here; it records its choice as a ruling note in its own issue's
> block (CLAUDE.md rule 3), and the ruling is what binds. `docs/stinkyscape-plus-set.md` is the model.

## Contents

- [FV0. What the designer asked for, and the owner's calls](#fv0-what-the-designer-asked-for-and-the-owners-calls)
- [FV1. The set in numbers, and its ids](#fv1-the-set-in-numbers-and-its-ids)
- [FV2. Readings](#fv2-readings)
- [FV3. The engine surface (frozen)](#fv3-the-engine-surface-frozen)
- [FV4. The Plagiarized reprints](#fv4-the-plagiarized-reprints)
- [FV5. The parts](#fv5-the-parts)
- [FV6. The road to staging](#fv6-the-road-to-staging)

---

## FV0. What the designer asked for, and the owner's calls

- **A new set, Felinors vs Humans (FvH):** 115 numbered cards, 22 sub-numbered tokens, Jilliax Deluxe's 13
  modules, and 82 **Plagiarized** reprints of Infinite Assault cards.
- **New mechanics:** Durability X, Fragile and Unlucky X; Endless Evolve appears on #111. "Reword all past
  cards to have this" (Durability, Fragile): only Core #79 Twinspell's base face is an exact Durability 1;
  no shipped card dies to any damage, so none rewords to Fragile.
- The "Supplementary Balance Changes" (`Card`) and "Misc Changes" (`aa`) sections are placeholders and
  ask for nothing.

**The owner's calls of 2026-11-28** (in the session that opened #5220):

1. A Claude session builds every wave in-session with fullsend and subagents; the issues are labelled
   `human` so no bot takes them.
2. The reprints are numbered **FvH #116 to #197**, in the list's order.
3. After wave 1 the set **ships** and players can use it on staging, while the AI deals itself none of it
   until wave 2 lifts the gate (the Stinkyscape+ precedent).
4. #72 Otherworldly Naturalism's Radiant face reads "Add a Radiant Wild Growth, Swipe and Savage Roar"
   (the list repeats #57's). #33 Unstoppable Force is a **Unit**. #100 is a **Spell**; #101 is a **Unit**.

## FV1. The set in numbers, and its ids

| | Count | Notes |
| --- | ---: | --- |
| Numbered cards of the set's own | 115 | #1 to #115 |
| Sub-numbered tokens | 22 | #16.1, #16.2, #19.1, #20.1, #49.1, #57.1–#57.6, #60.1, #60.2, #72.1–#72.3, #74.1, #78.1, #98.1, #98.2, #100.1, #100.2 |
| Jilliax Deluxe modules | 13 | #112.1 to #112.13 in the list's order (Recursion to Indestructible), tokens |
| Plagiarized reprints | 82 | #116 to #197 |
| **Catalog entries** | **232** | 197 non-token cards; the shadow ban's ceiling is 39 (1/5) |

**Ids.** `SetName::FelinorsVsHumans`, written `"Felinors vs Humans"`, segment `felinorsvshumans`,
abbreviation `FvH`, `shadow_ban_share` `(1, 5)` as Stinkyscape+'s. Cards are `felinorsvshumans-NNN`, a
sub-numbered token `felinorsvshumans-NNN-k` (index `"16.1"`), a module `felinorsvshumans-112-k` (index
`"112.k"`). Scripts live in `crates/cards/src/scripts/felinors_vs_humans/cNNN_slug.rs` (a token
`cNNN_k_slug.rs`; #84 𓃠's slug is `c084_cat_glyph`). The set's row goes at the end of `set_table!` as
`stage: Held` and ships when the release turns it to `Shipped`. Its §8 note is
`spec/08-catalog/8.14-felinors-vs-humans.md`.

**Every sub-numbered entry is a token** with `"token": true`, `"rarity": "Token"`, the `Token` tag and its
printed rarity (as S+ #4.1 Profe Yates Prime), whether or not the list prints "Token": each is made by its
parent card. **New tags:** Plagiarized, Roblox-Tool, Golem. Every other tag the list names exists (Human,
Felinor, KY, Book, Fruit, Potion, Prime, Catalyst, Acclaimed, Quickdraw, Terminus, Otherworldly,
Other-Origins, Jlockeed, Swarm, CN, Wincon, Call to Chaos).

## FV2. Readings

Each is the build's reading, written as a ruling note by the part that builds it. Hearthstone semantics
fill anything not listed (CLAUDE.md rule 3).

**Mechanics**

- **Durability X** (E1). One counter per card, shared by every Activate on it (granted ones included).
  A use spends 1: the `activate` action, and an effect that spends a card's use (`run_activation` with
  `spend_use`) spends exactly 1. The engine's own abilities (Cloaked's Reveal, a Starship's launch) spend
  none. A countered Activate keeps its Durability spent (as it keeps its use, R3550005). At 0 the card's
  Activates are refused ("No Durability left"); after the activation resolves, at the settled state check,
  a Field Spell (any non-Unit permanent) at 0 goes to its owner's graveyard by a plain move: no Death, not
  stopped by Indestructible (as Twinspell's, R3044000). A Unit at 0 lives on with no Activate. Refresh
  never restores Durability. The count resets when the card leaves the field (R78: a returned card is a
  new one, as Hearthstone's returned weapon). A Buff raises X by 1 and a Nerf lowers it by 1, never below
  1 (as Activate X, R386).
- **Fragile** (E1). Any hit that lowers the Unit's Health (≥ 1 dealt), with or without a source (Burn and
  Toxic ticks included), destroys it at the state check: all incoming damage is Poisonous. A hit dealing 0
  (Divine Shield, an immunity, Armor soaking all of it) does not; Indestructible still saves it. A Nerf may
  add Fragile and never removes it; a Buff removes it. Not a combat keyword (not in R21's pool).
- **Unlucky X** (E1). Net luck is Lucky − Unlucky. A negative net rolls |net| extra times and keeps the
  worst; a coin keeps tails if any. A Nerf may add Unlucky 1 or raise it; a Buff lowers it. Live only on a
  card with a luck roll (R3888000).
- **Endless Evolve** (E1). An Evolve that may happen any number of times: each time its condition is met,
  it evolves again (one evolution each, the multiplier as ever, R3536003).
- **"for a turn" / "for two turns"** (#7, #8): until the caster's next turn starts / until the caster's
  second turn from now starts.
- **A standalone "Overload X" line on a Unit** (#40, #63, #70, #74.1): paid as the card is played, folded
  into its Cry (Core #2's shape, R4353021); a summon pays nothing.

**Cards** (only those with a reading; every other card plays by its text)

| # | Reading |
| --- | --- |
| 1 | End of turn (base) / Start of turn (Radiant): a Nerf on itself, the usual Nerf (R386), which may now add Fragile or Unlucky. |
| 2 | The chosen card takes (3) less, the cards beside it (2) less, then (1) less, outward on both sides; Radiant (5), (4), … |
| 4 | The set's stats are an overwrite now (all friendly Units on the field) and a rest-of-game summon watcher (`watch_summons`) for every Unit you summon later. |
| 5 | A random Unit whose base script has a Death; its Death then runs once (`trigger_death`-style, as it would on dying). Radiant: Discover from Radiant ones. |
| 7, 8 | Grants last "for a turn" (FV2 above). |
| 10 | "a permanent" is a Unit (R3557002). |
| 16 | Base: one Travertine Stack with Temporary 2. Embiggened: six Stacks with Temporary 1 to 6. A Stack (9999) is played only at a lowered cost (#3); "when discarded or played" covers Temporary's discard. |
| 16.2 | Attack and Armor equal to your Float last turn (Radiant: thrice), at most 15. |
| 18 | Your mana cap rises by 2 (Radiant 4) for the rest of the game. |
| 19, 20 | Print no Durability: an Activate 1 with no lifetime cap. |
| 24 | A generic Dormant Starship with 2 random Starship Pieces fused (Radiant pieces on the Radiant face). |
| 25 | Radiant adds Radiant Auspicious Rocks. |
| 26 | A random Call to Chaos-tag card, twice. |
| 32 | Each card in hand is replaced by a different random card (never the one it replaces); base: one of them Radiant, Radiant: all Radiant. |
| 33 | A Unit (FV0). |
| 37 | Stays summoning-sick until it has come back by Reborn. |
| 39 | A copy of every Spell in your deck, with Cast on draw; the deck cap (R80) holds. |
| 42 | May be played onto either side's open zone; on its own side (base) / the opponent's (Radiant) it becomes a 3/3. Its Cry crushes only the cards beneath it. |
| 45 | Draw Traps from your deck up to 5, then make up the shortfall with random Traps (Radiant: summon). |
| 46 | Two random (2)-cost Units, left and right. You choose one in secret to gain Fragile; the opponent sees Fragile on both until the truth shows. |
| 49.1 | Radiant: deal 2. |
| 50 | "Deal 2 damage" to a chosen enemy. |
| 60.1, 60.2 | "the right-most / left-most card in your hand": the card's place as it left your hand (a one-card hand is both). |
| 61 | Hearthstone's Devolve: a random Unit of the closest cheaper price (a (0) becomes a random (0)). |
| 72 | Radiant face per FV0. |
| 72.1 | A mana crystal past the cap (`change_max_mana`). |
| 73 | Rest-of-game +2/+2 (+4/+4) to your Humans, later arrivals included. |
| 79 | The granted "End of turn" runs as the Human's own. |
| 82 | When you control no Shrinking Felinor after one dies, you lose. |
| 88 | Prints the Felinor tag; harmful effects that name Felinor pass it by (`harm_ignores_tags`). |
| 91 | Base: deal ⌊X·√log₂X⌋ to an enemy, or to every enemy when X is a power of 2. Radiant: X rounds up to the next power of 2 (at least 2), always every enemy. Values from `KY_FFT_DAMAGE`. |
| 92 | Aura: every card's Attack, Health and own cost (never X) counts as its number of binary digits (`⌊log₂N⌋+1`, N ≤ 0 unchanged). Radiant: only where that helps you (your costs, the enemy's Attack and Health). Two do not stack. A damaged Unit whose Health falls to 0 dies. |
| 100 | One of ten random effects (base), three (Radiant), the Call to Chaos edition pattern (`subsystems::call_to_chaos_fvh`). |
| 105 | Attacks every other Unit (Radiant: every enemy Unit) one at a time, in a random order (the owner's call of 2026-11-28); it stops when it dies or leaves the field. |
| 107 | At the end of your turn every swapped card still in a hand goes back. |
| 111 | Endless Evolve (FV2 above). |
| 112 | X Unit 0/1 on both faces plus its two modules (the designer's 0/0 given 1 Health so a state check before its Cry leaves it standing, R5262001); the module offer is the 4 dearest affordable, never the same module twice. |
| 114 | "simple" = a script of at most 10 code lines (`SIMPLE_CODE_LINES`). |

**Data kept as the designer wrote it:** #74's Radiant face keeps 2/6; #89's Radiant is 48/48.

## FV3. The engine surface (frozen)

The names, argument shapes and fields below are what every card script codes against. A part may add
private helpers; it may not rename these. "Exists" points at code on `main` today.

### E1. The new keywords (#5236)

- `Keyword::Durability { n: i32 }`, `KeywordKind::Durability = "Durability"`; catalog `{ "kind":
  "Durability", "n": X }`. Numbered (`NumberedKey::Durability`, more is better), in the tuning tables.
- `CardInstance.durability_spent: Option<i32>`, cleared by `zones::reset_instance`.
- `pub fn durability_left(state: &GameState, card: &CardInstance) -> Option<i32>` (`None` without the
  keyword; tuned n − spent, floor 0), in `subsystems/activate.rs`, in the prelude.
- `pub fn spend_durability(sink: &mut EngineSink<'_>, card_id: &str)`, called once at each real use
  (FV2); emits `GameEvent::CounterChanged` with the new `CounterKind::Durability`.
- `why_ability_unusable` refuses at 0; `ActivationView.durability_left: Option<i32>`.
- The settled state check moves a non-Unit permanent at Durability 0 to its graveyard (`send_to_graveyard`).
- `Keyword::Fragile`, `KeywordKind::Fragile = "Fragile"`: `damage.rs` step 7's target-side twin of
  Poisonous, `note_death_via(.., KeywordKind::Fragile)`; in `TUNE_HARMFUL_KEYWORDS`, `TUNE_BUFF_REMOVES`
  and the Nerf-adds list for Units.
- `Keyword::Unlucky { n: i32 }`, `KeywordKind::Unlucky = "Unlucky"`, numbered (less is better).
  `query::lucky_on` returns Lucky − Unlucky; `Rng::lucky` treats a negative count as worst-of; every
  `lucky > 0` / `== 0` / `.max(0)` short-cut becomes `!= 0`.
- `StaticFlags.endless_evolve: Option<bool>`, read by `evolution::can_evolve`.
- Core #79 Twinspell's base face prints Durability 1 and plays as before (a pending fragment claims the
  text change).

### E2. Lasting effects, watchers, grants and auras (#5237)

- `effects::delay::watch_summons(WatchSummonsArgs { step: String, label: String, hook: Option<String>,
  data: Option<IndexMap<String, Value>>, units_only: Option<bool> }) -> Effect` → `ModifierKind::SummonWatch
  { resume: Resume, label: String, units_only: Option<bool> }`, expiry never, offered like the lane
  watches for each Unit its owner summons later (data `{ instanceId, watcher }`).
- `effects::turn_buff::grant_for_turns(GrantForTurnsArgs { target: TargetSpec, keywords: Vec<Keyword>,
  attack: Option<i32>, turns: i32 }) -> Effect` → `CardInstance.timed_buffs: Option<Vec<TimedBuff>>`,
  `pub struct TimedBuff { pub player: PlayerId, pub until_turns_started: i32, pub attack: i32, pub
  keywords: Vec<Keyword> }`, read in the layers beside `turn_buff`, cleared by `reset_instance`.
- `PlayerState.last_float: Option<i32>`, set in turn cleanup beside `unspent_at_end`, never reset;
  `pub fn float_last_turn(state: &GameState, player: PlayerId) -> i32` in `query.rs`. Kept only in a
  game holding a Felinors vs Humans card (`fvh_kept`), so no other game's hash moves.
- `effects::mana::raise_mana_cap(RaiseManaCapArgs { amount: i32, player: Option<PlayerSpec> }) -> Effect`
  → `PlayerState.mana_cap_bonus: Option<i32>`, added to the cap wherever `Handicap.mana_cap` is read.
- `Script.stays_sick: Option<HasHook>`, OR-ed into `combat::is_sick` (Old Grum: `|a| !returned_by_reborn(a.self_)`).
- `pub struct HandPlace { pub index: i32, pub of: i32 }` (0 = leftmost; `of` = hand size before the play)
  and `GameEvent::CardPlayed.from_hand: Option<HandPlace>`; `pub fn played_leftmost(event: &GameEvent) ->
  bool` and `played_rightmost` in `query.rs`. Kept only in a game holding a Felinors vs Humans card
  (`fvh_kept`), so no other game's hash moves.
- `Grant.on: Option<GrantOn>`, `pub enum GrantOn { Death, EndOfTurn }` (absent = Death);
  `GrantAbilityArgs.on: Option<GrantOn>`; `grants::end_of_turn_hook_of(state, card) -> Option<Hook>`;
  a field card with an EndOfTurn grant is an end-of-turn holder.
- `ModifierKind::UnitsStats { attack: i32, health: i32, tags: Option<Vec<Tag>>, from_spell: Option<bool> }`
  and `effects::player_mods::give_units_stats(GiveUnitsStatsArgs { player: Option<PlayerSpec>, attack: i32,
  health: i32, tags: Option<Vec<Tag>>, expiry: ModifierExpiry }) -> Effect`.
- `ModifierKind::CardsHaveKeyword { keyword: Keyword, from_spell: Option<bool> }` and
  `effects::player_mods::give_cards_keyword(args: GiveUnitsKeywordArgs) -> Effect`: every card its player
  controls; `pub fn cards_have_aura(keyword: Keyword) -> AuraHook` in `layers.rs`.
- `StaticFlags.harm_ignores_tags: Option<Vec<Tag>>`, read by `restrictions::tribal_hate_cannot_reach`.
- `StaticFlags.immune_to_tag: Option<Tag>` and `pub fn tag_cannot_reach(state: &GameState, source:
  Option<&CardInstance>, card: &CardInstance) -> bool` in `restrictions.rs`, at each `spell_cannot_reach`
  / `unaffected_by` / `spell_damage_cannot_reach` site.
- `StaticFlags.log2_utility: Option<Log2Utility>`, `pub enum Log2Utility { All, Helpful }`;
  `pub fn log2_utility(n: i32) -> i32` in `numbers.rs`; applied in `layers::unit_view` (Attack, max Health)
  and `mana::effective_cost` (own cost, never X) while a #92 acts.
- `recursion_log::kept` also holds when any definition in the game carries `Tag::Plagiarized`
  (`RECURSION_LOG_TAGS` in `config.rs`).

### E3. Play, board and hand verbs (#5238)

- `AddToHandArgs.temporary_count: Option<i32>` (and on `AddRandomFromCatalogArgs`).
- `effects::secret_grant::grant_keyword_secretly(SecretGrantArgs { target: TargetSpec, decoys:
  Vec<String>, keyword: Keyword }) -> Effect` and `CardInstance.shown_to_opponent: Option<Vec<Keyword>>`,
  unioned into the keywords the other seat's view (and the AI's redacted copy) shows.
- `StaticFlags.plays_on_either_side: Option<bool>` and `ZoneChoice.player: Option<PlayerId>` (absent =
  own side), accepted only for such a card; `legal_actions` lists both sides' open zones.
- `CrushArgs.beneath_only: Option<bool>`.
- `ShuffleIntoArgs.enchantments: Option<Vec<Enchantment>>`.
- `RecruitFilter.set: Option<OneOrMany<SetName>>`.
- `ReplaceHandWithRandomArgs.exclude_replaced: Option<bool>`.
- `DiscardLibraryArgs.instance_ids: Option<Vec<String>>`.
- `effects::delay::allure_unit(AllureUnitArgs { instance_id: Option<String>, random_enemy: Option<bool> })
  -> Effect`.
- `effects::delay::watch_deaths(WatchDeathsArgs { tag: Tag, step: String, label: String, hook:
  Option<String>, data: Option<IndexMap<String, Value>> }) -> Effect` (a never-expiring `DeathWatch`).
- `transform_cost_step` with a negative step takes the highest price at or below `start + step`.
- `effects::player_mods::gain_woozy(GainWoozyArgs { player: Option<PlayerSpec>, amount: i32 }) -> Effect`
  (the body of `woozy::add_on_play` factored into `woozy::raise`).
- `ForcedAttackArgs.allies: Option<bool>`.
- `SwapHandsArgs.until_end_of_turn: Option<bool>`.

### E4. Discovery, catalog filters and damage riders (#5239)

- `pub fn added_to_hand_so_far(ctx: &EffectContext<'_>) -> Vec<String>` in `prompts.rs`.
- `SummonRandomWithArgs.dormant_turns: Option<i32>`.
- `CatalogQueryArgs.hooks: Option<Vec<HookFilter>>` (any of), `pub enum HookFilter { Death, Aura, Cry }`,
  and `CatalogQueryArgs.any_of: Option<Vec<CatalogQueryArgs>>`.
- `SplitAmong::Units` (every standing Unit, both sides, no hero).
- `effects::choose::discover_keyword(DiscoverKeywordArgs { step: String, pool: Option<Vec<Keyword>>,
  count: Option<i32>, lacking_on: Option<TargetSpec>, data: StepData }) -> Effect`; the options are drawn
  inside `apply` and read back with `chosen_options`.
- `DiscoverFromCatalogArgs.order: Option<DiscoverOrder>`, `pub enum DiscoverOrder { Random, Dearest }`, and
  `EquipPicksArgs.order`.
- `StaticFlags.flashback_without_cry: Option<bool>`.
- `StaticFlags.damage_also_toxic: Option<bool>` beside `damage_also_burns`.
- `DiscoverFromCatalogArgs.past_options: Option<i32>` and `pub fn discovered_card(ctx: &EffectContext<'_>)
  -> Option<String>`.
- `pub const SIMPLE_CODE_LINES: usize = 10;` and `pub fn is_simple(def_id: &str) -> bool` (prelude).
- `pub const KY_FFT_DAMAGE: &[i32] = &[0, 0, 2, 3, 5, 7, 9, 11, 13, 16, 18, 20, 22, 25, 27, 29, 32];` and
  `pub const KY_FFT_MAX_POINTS: i32 = 16;` in `config.rs`.

## FV4. The Plagiarized reprints

Each reprint is a **frozen copy** of its Infinite Assault card's script (the Legacy cards' precedent,
R2195003): `crates/cards/src/scripts/felinors_vs_humans/cNNN_slug.rs`, its `ID` its own, so a later IA
patch never moves it and its code metrics are its own. Its name is `Plagiarized: <IA name>`, its only tag
Plagiarized (a token it makes stays the IA token), its faces and stats the IA card's but for the changes the
list names (the IA card's cost, stats, rarity otherwise kept):

| List | Change |
| --- | --- |
| 4 Scrap Prototype | 2/2 (Radiant 4/4) |
| 6 Compute Shard | its Start of turn becomes End of turn |
| 11 Generated Filler | its pool is Plagiarized cards (two random (1)-cost Plagiarized Units) |
| 15 Packet Sniffer | costs (0) |
| 27 Core Dump, 31 Emergent Behavior, 40 Patient Machine, 42 Prompt Injection, 48 Scrape the Graveyard | cost (1) |
| 29 Doomsday Clock | deals 20 (Radiant 40) |
| 34 Inevitable, 37 Mana Leech | cost (3) |
| 50 Synthetic Data | cost (1); its pool is Plagiarized cards |
| 51 Thought Accelerator | gains Tradable |
| 54 DDoS, 60 Ouroboros Engine | cost (2) |
| 81 Singularity | Mythic; tags Plagiarized and Wincon (so M+ #1 Bob's Plan's pool holds it); gains Immutable |

The list's misspellings are read as the IA names: Doppelganger Protocol, Packet Sniffer, Thought
Accelerator, Phantom Army. Plagiarized is a provenance tag, like Legacy, so it is never a random-deck
theme (`crates/ai/src/deck.rs`).

## FV5. The parts

**Wave 1, stage A** (engine and set plumbing, built in parallel, then one green pass, pushed before stage B):
F #5235, E1 #5236, E2 #5237, E3 #5238, E4 #5239. Each owns its ruling block (R<n>000–R<n>999) and
`spec/pending/<n>.md`.

**Wave 1, stage B** (fullsend: card slices built blind against the landed engine, then contact, reconcile,
green and cull), each part its own issue and ruling block:

| Part | Issue | Cards |
| --- | --- | --- |
| S01 | #5251 | units on printed keywords and simple Cries: #1, #11–#15, #33, #35, #37, #43, #47, #48, #62, #89, #90, #93 |
| S02 | #5252 | spells on existing verbs: #7–#9, #21, #22, #38, #51, #55, #56, #61, #77, #104 |
| S03 | #5253 | the Books: #23–#30 |
| S04 | #5254 | Activates, Durability and the Roblox-Tools: #19, #19.1, #20, #20.1, #32, #36, #41, #49, #49.1, #50, #53, #80, #85, #86, #115 |
| S05 | #5255 | the Otherworldly cards: #57, #57.1–#57.6, #58, #72, #72.1–#72.3 |
| S06 | #5256 | KY, Travertine and the Float cards: #3, #4, #16, #16.1, #16.2, #17, #18, #31, #39, #91, #92, #107 |
| S07 | #5257 | the Felinors: #44, #65, #66, #69, #70, #71, #81, #84, #88, #94, #96, #97, #105 |
| S08 | #5258 | Human auras, Catalysts and Primes: #52, #54, #63, #64, #67, #68, #73, #74, #74.1, #78, #78.1, #79, #95 |
| S09 | #5259 | the rule benders: #2, #5, #6, #10, #34, #40, #42, #45, #46 |
| S10 | #5260 | readers, tools, graves and portals: #59, #60, #60.1, #60.2, #75, #76, #82, #83, #87, #98, #98.1, #98.2, #99 |
| S11 | #5261 | transforms, draws and the last Humans: #101–#103, #106, #108–#111, #113, #114 |
| S12 | #5262 | Call to Chaos (FvH Edition) and Jilliax Deluxe: #100, #100.1, #100.2, #112, #112.1–#112.13 |
| P1 | #5263 | every entry's card text, voice lines, Chinese, audio entry and radiant-audit row |
| P2 | #5264 | every entry's art and FX, the keyword visuals, and the client pieces (#42's enemy zones, the end-of-turn voice hook, cat ears, #84's glyph) |

The 82 reprints were completed in stage A (F #5235). The release (the set ships, gated from the AI's own
decks) closes wave 1.

**Waves 2 and 3** are cut once the sweep of record names the flagged cards: one slice per family.

## FV6. The road to staging

Each wave lands on `main` through one pull request with every required check green (`rust`, `web`,
`e2e smoke`, `db`, `bot selftest`; CLAUDE.md rule 12), then `promote staging` is dispatched by hand. A wave
is on staging when its merge commit is an ancestor of `_staging` and staging.jackioh.com's `/api/catalog`
reports that commit. The tracker #5220 records each step.
