//! What a play may choose, what its choices then do, and what `legalActions` offers (SPEC §2.5, §8
//! Conventions, §10.2, R43, R81, R90, R102, R151, R211). Found by the polish-4 edge-case hunt,
//! round 2 (docs/polish/4-edge-cases.md, lenses L7 and L9); every case here failed before its fix.
//!
//! The agreement between `legalActions` and `reduce` held across seeded random-policy games and
//! millions of mutated candidate actions. What failed is below: a power SPEC makes usable that neither
//! side offered, choices a fused card was offered and then ignored, a mode SPEC's conventions require
//! a target for that could name none, and a concede `reduce` accepts while a prompt is open that
//! `legalActions` did not list.
//!
//! Round 9, lens "legality agreement": the bound on `legalActions`' enumeration dropped whole picks —
//! a Lava Golem's all-enemy Tribute, a crafted card's first declaration's later picks — so the client,
//! which builds a play only out of the plays listed (CLAUDE.md rule 7), could not make them. A Tribute's
//! sets are listed whole and a cut keeps every pick of every declaration (R90, amended).
//!
//! Port of `packages/cards/test/play-choices.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_cards::register_all;
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

const MR_VANILLA: &str = "core-008";
const MIDRANGE_MENACE: &str = "core-019";
const EFFICIENCY_DIVIDEND: &str = "core-024";
const ARCHIVIST: &str = "core-030";
const PREM_PANTHER: &str = "core-032";
const RENO: &str = "core-053";
const SILLY_SILAS: &str = "core-052";
const BIGOT: &str = "core-002";
const TWISTED_SORCERER: &str = "core-068";
const HEROIC_POWER: &str = "core-098";
const CALL_TO_CHAOS: &str = "core-095";
const CRAFT_A_CARD: &str = "core-099";
const KYS_TUTOR: &str = "core-051";

/// TS `let nonce = 0`: the file's action counter, so every action this file sends has its own nonce.
static NONCE: AtomicU32 = AtomicU32::new(0);

/// TS `act(state, { ...body, playerId })`: one action through `reduce` with a fresh nonce.
fn act(state: &GameState, body: ActionBody, player: PlayerId) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    reduce(state, &Action::new(body, player, format!("play-choices-{nonce}")))
}

/// An action body written as the TS object literal (`{ type: "play", … }`).
fn body(literal: Value) -> ActionBody {
    json_as(literal)
}

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected {what}"),
    }
}

/// The `play` actions `legalActions` offers the card's controller for this card.
fn plays_of(state: &GameState, card: &CardInstance) -> Vec<ActionBody> {
    legal_actions(state, card.controller)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id))
        .collect()
}

/// A play's `targets`, empty when it names none (TS `play.targets ?? []`).
fn targets_of(action: &ActionBody) -> Vec<Selection> {
    match action {
        ActionBody::Play { targets, .. } => targets.clone().unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// A play's `modes`, empty when it names none (TS `play.modes ?? []`).
fn modes_of(action: &ActionBody) -> Vec<String> {
    match action {
        ActionBody::Play { modes, .. } => modes.clone().unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// A play's `tributes`, empty when it names none (TS `play.tributes ?? []`).
fn tributes_of(action: &ActionBody) -> Vec<String> {
    match action {
        ActionBody::Play { tributes, .. } => tributes.clone().unwrap_or_default(),
        _ => Vec::new(),
    }
}

mod r43_r151_a_heroic_power_created_on_the_field_rolls_its_power {
    use super::*;

    #[test]
    fn r151_a_heroic_power_95_call_to_chaos_summons_straight_into_the_backrow_rolls_a_power_so_activatepower_is_offered_and_accepted_for_it_r43()
     {
        register_all();
        // R43: "one created later rolls when it is created"; R151: it rolls "as it arrives anywhere a
        // card can be looked at". #95's "summon 5 random Field Spells or Traps into your backrow" puts a
        // Heroic Power straight onto the field — never through a hand or a library, the only arrivals
        // that roll. (Carnivorous Cube's copies were this test's route until R428 took backrow meals
        // away from the Cube.) The roll is pinned: the play's first rng draw is #95's roll, and the first
        // backrow pick follows it, from the pool of every set (R380).
        let seed = "r151-chaos";
        let pool = catalog::query(&json_as(json!({ "type": ["Field Spell", "Trap", "Field Trap"] })));
        let backrow = subsystems::CHAOS_EFFECTS.iter().position(|effect| effect.name == "backrow");
        let mut cursor: Option<u32> = None;
        let mut at: u32 = 0;
        while at < 200_000 && cursor.is_none() {
            let mut rng = Rng::new(seed, at);
            let rolled = rng.int(subsystems::CHAOS_EFFECTS.len() as i32);
            if Some(rolled as usize) == backrow {
                let picked = rng.int(pool.len() as i32);
                if pool.get(picked as usize).map(|def| def.id.as_str()) == Some(HEROIC_POWER) {
                    cursor = Some(at);
                }
            }
            at += 1;
        }
        // TS `expect(cursor).toBeGreaterThanOrEqual(0)`: a cursor was found.
        assert!(cursor.is_some());
        let cursor = must(cursor, "a cursor that rolls the backrow effect and picks a Heroic Power first");

        let mut g = scenario(json!({
            "seed": seed,
            "p1": { "hand": [CALL_TO_CHAOS, RENO], "mana": 10 },
            "p2": { "hand": [RENO] },
        }));
        g.state_mut().rng_cursor = cursor;
        g.play(CALL_TO_CHAOS, json!({}));
        while let Some(pending) = g.state().pending.clone() {
            if pending.player_id != P1 {
                break;
            }
            let Some(first) = pending.options.first() else {
                break;
            };
            g.answer(json!(first.key));
        }

        let powers: Vec<CardInstance> = [1, 2, 3, 4, 5]
            .into_iter()
            .filter_map(|lane| g.backrow(P1, lane))
            .filter(|card| card.def_id == HEROIC_POWER)
            .collect();
        assert!(!powers.is_empty());
        // Each one rolled its power as it arrived (R151).
        for power in &powers {
            assert!(power.memory.get(subsystems::POWER_KEY).is_some_and(Value::is_string));
        }

        // R752: each power is the card's Activate ability, listed as an `activate` (once per target for Ping).
        let listed_for = |state: &GameState, power: &CardInstance| -> Vec<ActionBody> {
            legal_actions(state, P1)
                .into_iter()
                .filter(|action| matches!(action, ActionBody::Activate { instance_id, .. } if *instance_id == power.id))
                .collect()
        };
        let mana = g.state().players.p1.mana.current;
        // TS `(powerOf(power)?.x ?? Number.POSITIVE_INFINITY) <= mana.current`.
        let affordable: Vec<CardInstance> = powers
            .iter()
            .filter(|power| subsystems::power_of(power).is_some_and(|rolled| rolled.x <= mana))
            .cloned()
            .collect();
        assert!(!affordable.is_empty());
        for power in &affordable {
            assert!(!listed_for(g.state(), power).is_empty());
        }

        let first_power = must(affordable.first(), "an affordable power");
        let first = must(listed_for(g.state(), first_power).first().cloned(), "a listed activation");
        let used = act(g.state(), first, P1);
        assert_eq!(used.error, None);
    }
}

mod r90_r102_a_fused_card_s_declarations_each_read_their_own_slice_of_the_play_s_choices {
    // Craft a Card fuses two Discovered Units into one hand card whose declared targets and modes are
    // the ingredients' lists concatenated in ingredient order (R102), so `legalActions` offers — and
    // `reduce` validates — one flat list read declaration by declaration (R90). The fused Cry used to
    // hand the whole list to every ingredient, and each ingredient read its first slot, so every
    // choice after the first was offered, accepted and then ignored.
    use super::*;

    #[test]
    fn r102_a_crafted_bigot_twisted_sorcerer_destroys_bigot_s_target_and_deals_the_sorcerer_s_4_to_the_sorcerer_s_own_target_r90()
     {
        register_all();
        let mut g = scenario(json!({
            "seed": "craft-453", // the first Discover offers Bigot, the second Twisted Sorcerer (pools of every set, R380)
            "p1": { "hand": [CRAFT_A_CARD, RENO], "mana": 4 },
            "p2": { "hand": [RENO], "field": [PREM_PANTHER] },
        }));
        g.play(CRAFT_A_CARD, json!({}));
        g.answer(json!(BIGOT));
        g.answer(json!(TWISTED_SORCERER));
        let crafted = must(
            g.hand(P1).iter().find(|card| card.def_id.starts_with("t-")).cloned(),
            "the crafted card",
        );
        let panther = must(g.unit(P2, 1), "p2's Prem Panther (not a Human)");

        // Bigot's declaration first, the Sorcerer's second: both targets travel in the one play.
        let choice = vec![
            Selection::Instance { instance_id: panther.id.clone() },
            Selection::Hero { player: P2 },
        ];
        assert!(plays_of(g.state(), &crafted)
            .iter()
            .any(|play| matches!(play, ActionBody::Play { targets: Some(targets), .. } if *targets == choice)));

        g.play(&crafted.id, json!({ "zone": 2, "targets": choice }));

        let sorcerer_hit = g.last_events().iter().find_map(|event| match event {
            GameEvent::Damage { target_id, .. } => Some(target_id.clone()),
            _ => None,
        });
        assert_eq!(
            json!({
                "panther": g.card(&panther.id).zone.z().as_str(),
                "sorcererHit": sorcerer_hit,
                "p2Health": g.state().players.p2.hero.health,
            }),
            json!({ "panther": "graveyard", "sorcererHit": "hero-p2", "p2Health": 26 })
        );
    }

    #[test]
    fn r102_a_crafted_archivist_silly_silas_draws_by_archivist_s_mode_and_rotates_by_silas_s_direction_r81_r90() {
        register_all();
        let mut g = scenario(json!({
            "seed": "craft-446", // the first Discover offers Archivist, the second Silly Silas (pools of every set, R380)
            "p1": { "hand": [CRAFT_A_CARD, RENO], "mana": 4, "library": [MR_VANILLA, MIDRANGE_MENACE, MR_VANILLA] },
            "p2": { "hand": [RENO], "field": [MR_VANILLA] },
        }));
        g.play(CRAFT_A_CARD, json!({}));
        g.answer(json!(ARCHIVIST));
        g.answer(json!(SILLY_SILAS));
        let crafted = must(
            g.hand(P1).iter().find(|card| card.def_id.starts_with("t-")).cloned(),
            "the crafted card",
        );
        let theirs = must(g.unit(P2, 1), "p2's Mr. Vanilla");

        // Archivist's declaration first, Silas's second: "highest", then "right".
        let wanted = vec!["highest".to_string(), "right".to_string()];
        assert!(plays_of(g.state(), &crafted)
            .iter()
            .any(|play| matches!(play, ActionBody::Play { modes: Some(modes), .. } if *modes == wanted)));

        g.play(&crafted.id, json!({ "zone": 2, "modes": ["highest", "right"] }));

        // Rotating right moves the crafted card from lane 2 to lane 3, and p2's lane-1 card, the last
        // step of the ring, onto p1's lane 1 (§3.1).
        let hand: Vec<String> = g.hand(P1).iter().map(|card| card.def_id.clone()).collect();
        assert_eq!(
            json!({
                "rotated": g.last_events().iter().any(|event| matches!(event, GameEvent::Rotated { .. })),
                "craftedLane3": g.unit(P1, 3).map(|card| card.id.clone()),
                "theirsNowControlledBy": g.card(&theirs.id).controller,
                "hand": hand,
            }),
            json!({ "rotated": true, "craftedLane3": crafted.id, "theirsNowControlledBy": "p1", "hand": [RENO, MIDRANGE_MENACE] })
        );
    }
}

mod s8_conventions_r90_a_mode_that_deals_damage_or_heals_needs_its_target {
    use super::*;

    #[test]
    fn r90_efficiency_dividend_s_damage_and_heal_modes_are_never_offered_or_accepted_without_a_target_while_one_exists_and_its_mana_mode_names_none_8_conventions()
     {
        register_all();
        // §8: "target" means the player picks from all legal units and heroes, and only a target SET that
        // is empty lets the Cry fizzle; R90: "a declaration the board cannot satisfy does not refuse the
        // play". A hero is always a legal target, so "deal X damage to a target" and "heal a target 2X"
        // can always be satisfied. The card used to declare its target `min: 0` for the sake of the mana
        // mode, which let a damage or heal play name nobody and pay its X for nothing.
        let g = scenario(json!({
            "p1": { "hand": [EFFICIENCY_DIVIDEND, RENO], "mana": 4 },
            "p2": { "hand": [RENO], "field": [MR_VANILLA] },
        }));
        let dividend = must(g.hand(P1).first().cloned(), "Efficiency Dividend");

        let untargeted: Vec<ActionBody> = plays_of(g.state(), &dividend)
            .into_iter()
            .filter(|play| modes_of(play).first().map(String::as_str) != Some("mana") && targets_of(play).is_empty())
            .collect();
        assert_eq!(untargeted, Vec::<ActionBody>::new());

        let refused = act(
            g.state(),
            body(json!({ "type": "play", "instanceId": dividend.id, "x": 3, "modes": ["damage"], "targets": [] })),
            P1,
        );
        assert!(refused.error.is_some());

        // The mana mode names no target at all: it is offered without one, and refused with one.
        let mana: Vec<ActionBody> = plays_of(g.state(), &dividend)
            .into_iter()
            .filter(|play| modes_of(play).first().map(String::as_str) == Some("mana"))
            .collect();
        assert!(!mana.is_empty());
        assert!(mana.iter().all(|play| targets_of(play).is_empty()));
        let aimed = act(
            g.state(),
            body(json!({
                "type": "play",
                "instanceId": dividend.id,
                "x": 2,
                "modes": ["mana"],
                "targets": [{ "pick": "hero", "player": "p2" }],
            })),
            P1,
        );
        assert!(aimed.error.is_some());
    }
}

mod r211_concede_is_on_offer_while_a_prompt_is_open {
    use super::*;

    #[test]
    fn r211_legalactions_offers_concede_to_both_players_while_a_prompt_is_open_as_reduce_accepts_it_2_5_10_2() {
        register_all();
        // KY's Private Tutor opens a type prompt for p1. reduce accepts a concede from either seat while
        // it is open (BUILD M1-T3), so legalActions — which the client's Concede button reads (§10.2) —
        // must list it for both, alongside the holder's answers.
        let mut g = scenario(json!({
            "p1": { "hand": [KYS_TUTOR], "library": [RENO, MR_VANILLA, "core-005"] },
            "p2": { "field": [{ "def": MR_VANILLA, "lane": 1 }], "library": [RENO] },
        }));
        g.play(KYS_TUTOR, json!({}));
        assert_eq!(g.state().pending.as_ref().map(|pending| pending.player_id), Some(P1));

        for player in [P1, P2] {
            assert_eq!(
                reduce(g.state(), &Action::new(ActionBody::Concede, player, format!("c-{player}"))).error,
                None
            );
            let types: Vec<ActionType> = legal_actions(g.state(), player)
                .iter()
                .map(ActionBody::action_type)
                .collect();
            assert!(types.contains(&ActionType::Concede));
        }
        // The holder is still offered its answers; the other seat nothing else.
        assert!(legal_actions(g.state(), P1)
            .iter()
            .any(|action| action.action_type() == ActionType::Answer));
        assert_eq!(legal_actions(g.state(), P2), vec![ActionBody::Concede]);
    }
}

// ---------------------------------------------------------------------------
// Round 9: every pick a play may make is one legalActions offers (R81, R90, R101, R102)
// ---------------------------------------------------------------------------

const LAVA_GOLEM: &str = "core-055"; // Tribute 3; may tribute enemy units (§8 #55, R101)
const KPOP_FANATIC: &str = "core-050"; // Unit: Cry: choose an enemy permanent (unit or backrow)
const RUSH_TOKEN_FARM: &str = "core-058"; // a public Field Spell that acts only at its start of turn

fn units_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
    [1, 2, 3, 4, 5]
        .into_iter()
        .filter_map(|lane| s.unit(player, lane))
        .collect()
}

fn plays_for(state: &GameState, player: PlayerId, card: &CardInstance) -> Vec<ActionBody> {
    legal_actions(state, player)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id))
        .collect()
}

/// A sorted copy joined by commas (TS `[...ids].sort().join(",")`).
fn sorted_key(ids: &[String]) -> String {
    let mut ids = ids.to_vec();
    ids.sort();
    ids.join(",")
}

mod r81_r90_r101_every_tribute_a_play_may_pay_is_one_legalactions_offers {
    use super::*;

    #[test]
    fn s8_55_a_lava_golem_play_that_tributes_three_enemy_units_is_offered_not_only_sets_holding_the_player_s_own_first_units_r81_r90_r101()
     {
        register_all();
        // p1 has four units and one free unit zone, p2 five units: nine units Lava Golem may tribute.
        let s = scenario(json!({
            "p1": { "hand": [LAVA_GOLEM], "field": [MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA] },
            "p2": { "field": [MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA], "hand": [MR_VANILLA] },
        }));
        let golem = must(
            s.state().players.p1.hand.iter().find(|card| card.def_id == LAVA_GOLEM).cloned(),
            "the Lava Golem in hand",
        );
        let enemies: Vec<String> = units_of(&s, P2).into_iter().map(|unit| unit.id).collect();
        let all_enemy = vec![enemies[2].clone(), enemies[3].clone(), enemies[4].clone()];

        // reduce accepts the play: #55 "may tribute enemy units" (R101), three units pay Tribute 3.
        let accepted = reduce(
            s.state(),
            &json_as::<Action>(json!({
                "type": "play",
                "playerId": "p1",
                "nonce": "r9-legality-golem",
                "instanceId": golem.id,
                "zone": { "row": "units", "lane": 5 },
                "tributes": all_enemy,
            })),
        );
        assert_eq!(accepted.error, None, "reduce accepts Lava Golem tributing three enemy units");

        // …so legalActions, which the client narrows and the AI policy draws from (§10.2, CLAUDE.md
        // rule 7), must offer it. The Tribute sets are enumerated own units first and cut at
        // MAX_CHOICE_COMBINATIONS, so with nine candidates every offered set holds one of p1's first
        // three units and no set of enemy units alone is ever offered.
        let offered = plays_for(s.state(), P1, &golem);
        assert!(!offered.is_empty());
        let enemy_only: Vec<&ActionBody> = offered
            .iter()
            .filter(|play| tributes_of(play).iter().all(|id| enemies.contains(id)))
            .collect();
        assert!(
            !enemy_only.is_empty(),
            "offered Lava Golem plays whose Tribute is paid with enemy units only"
        );
        let exact = offered
            .iter()
            .any(|play| sorted_key(&tributes_of(play)) == sorted_key(&all_enemy));
        assert!(exact, "the accepted all-enemy Tribute set is among the offered plays");
    }
}

/// The first pick of an offered play, as the TS test keys it: `none`, `hero:<player>`, the instance
/// id, or the pick's own name.
fn first_pick_key(action: &ActionBody) -> String {
    match targets_of(action).first() {
        None => "none".to_string(),
        Some(Selection::Hero { player }) => format!("hero:{player}"),
        Some(Selection::Instance { instance_id }) => instance_id.clone(),
        Some(Selection::Zone { .. }) => "zone".to_string(),
        Some(Selection::Mode { .. }) => "mode".to_string(),
        Some(Selection::None) => "none".to_string(),
    }
}

mod r81_r90_r102_every_pick_a_crafted_card_s_declaration_may_make_is_one_legalactions_offers {
    use super::*;

    #[test]
    fn s8_99_a_crafted_twisted_sorcerer_k_pop_fanatic_is_offered_with_the_sorcerer_s_4_damage_aimed_at_the_enemy_hero_r81_r90_r102()
     {
        register_all();
        // p1: four units and a free lane; p2: five units and three public Field Spells. The Sorcerer's
        // declaration reaches eleven picks (p1's four units and hero, p2's five units and hero), Kpop's
        // eight (p2's five units and three backrow cards).
        let mut s = scenario(json!({
            "p1": { "hand": [TWISTED_SORCERER, KPOP_FANATIC], "field": [MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA] },
            "p2": {
                "field": [MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA, MR_VANILLA],
                "backrow": [RUSH_TOKEN_FARM, RUSH_TOKEN_FARM, RUSH_TOKEN_FARM],
                "hand": [MR_VANILLA],
            },
        }));
        let sorcerer = must(
            s.state().players.p1.hand.iter().find(|card| card.def_id == TWISTED_SORCERER).cloned(),
            "the Sorcerer",
        );
        let kpop = must(
            s.state().players.p1.hand.iter().find(|card| card.def_id == KPOP_FANATIC).cloned(),
            "the K-Pop Fanatic",
        );
        // §8 #99: "Discover a Unit, then Discover another; Fuse them; the result costs 0 and goes to your
        // hand" — the fusion #99's last step makes, called where the rule lives (R77, R102).
        let crafted = {
            // TS `{ state: s.state, events: [], rng: createRng(s.state.seed, s.state.rngCursor) }`.
            let state = s.state_mut();
            let mut events: Vec<GameEvent> = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            subsystems::fuse(&mut sink, json_as(json!({ "ingredients": [sorcerer, kpop], "toHand": "p1" })))
        };
        let crafted = must(crafted, "the crafted card");
        let hand_ids: Vec<String> = s.state().players.p1.hand.iter().map(|card| card.id.clone()).collect();
        assert!(hand_ids.contains(&crafted.id));

        let enemy_units: Vec<String> = units_of(&s, P2).into_iter().map(|unit| unit.id).collect();
        let play = json_as::<Action>(json!({
            "type": "play",
            "playerId": "p1",
            "nonce": "r9-legality-crafted",
            "instanceId": crafted.id,
            "zone": { "row": "units", "lane": 5 },
            "targets": [{ "pick": "hero", "player": "p2" }, { "pick": "instance", "instanceId": enemy_units[0] }],
        }));
        // reduce accepts it: the Sorcerer's part names the enemy hero, Kpop's an enemy unit (R90, R102).
        let accepted = reduce(s.state(), &play);
        assert_eq!(accepted.error, None, "reduce accepts the crafted card's play at the enemy hero");
        assert_eq!(
            accepted.state.players.p2.hero.health, 26,
            "the Sorcerer's part dealt its 4 to the enemy hero"
        );

        // legalActions must offer the Sorcerer's part every pick it may make. The two declarations are
        // crossed first-slowest and cut at MAX_CHOICE_COMBINATIONS, so only the Sorcerer's first eight
        // picks (p1's side, then p2's first three units) ever reach an offered play.
        let offered = plays_for(s.state(), P1, &crafted);
        assert!(!offered.is_empty());
        let first_picks: IndexSet<String> = offered.iter().map(first_pick_key).collect();
        assert!(
            first_picks.contains("hero:p2"),
            "an offered play aims the Sorcerer's part at the enemy hero"
        );
        for id in &enemy_units {
            assert!(first_picks.contains(id), "an offered play aims the Sorcerer's part at {id}");
        }
    }
}
