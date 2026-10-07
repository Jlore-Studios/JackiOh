//! The board-wide and adjacency verbs of the effects library (BUILD M3-T1 "every effect has its own
//! test"; SPEC §6.3, §3.1, §3.2, §4.4, §4.5, R11, R12, R13, R16, R31, R46, R55, R59, R68, R78,
//! §10.7). Eight verbs, all of them a thin walk over `cardsInScope` / `adjacentTo`:
//!   destroyAll, destroyAdjacentTo   (effects/destroy.ts)  — #2, #16, #17, #43, #88
//!   damageAll                       (effects/damage.ts)   — #13
//!   bounceAll, exileAll, exileAdjacentTo, discardHand, exileHand
//!                                   (effects/move.ts)     — #17, #34, #76, #78, #100
//!
//! What these tests are really pinning down is the difference between a sweep and a loop of
//! single-target verbs: `destroyAll` only marks, so §4.5 collects the whole board under ONE state
//! check (R59), and `damageAll` snapshots its targets, so one hit never changes who else is hit.
//! The fixture defs live here rather than in a shared fixture, per CLAUDE.md and BUILD §0.
//!
//! Port of `packages/engine/test/effects-boardwide.test.ts`.

use std::collections::BTreeSet;

use jackioh_engine::effects::{
    bounce_all, damage_all, destroy, destroy_adjacent_to, destroy_all, discard_hand, discard_random, exile_adjacent_to,
    exile_all, exile_hand,
};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::token_def;
use crate::rules::fixtures::combat::{armoured, indestructible, plain, shielded};
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, slot};

// ---------------------------------------------------------------------------
// Fixture cards: one tagged body per scope filter the eight call sites use.
// ---------------------------------------------------------------------------

/// TS `unitDefOf(name, overrides)`; `index` is TS's `nextIndex` (1300, bumped once per call in
/// declaration order: human 1301, felinor 1302, beast 1303, field-spell 1304).
fn unit_def_of(name: &str, index: u32, overrides: Value) -> CardDef {
    let mut def = json!({
        "id": format!("bw-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (boardwide)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": format!("{name} radiant") },
    });
    if let (Some(def), Some(overrides)) = (def.as_object_mut(), overrides.as_object()) {
        for (key, value) in overrides {
            def.insert(key.clone(), value.clone());
        }
    }
    json_as(def)
}

/// #2 Bigot's radiant destroys every *non-Human* enemy unit, so the scope needs both halves.
const HUMAN: &str = "bw-human";
/// #43 Big Felinor spares Felinors, its own token included (t-felinor).
const FELINOR: &str = "bw-felinor";
/// An untagged body: what `notTags` keeps and `tags` rejects.
const BEAST: &str = "bw-beast";
/// #88 Twisting Nether and #100 Ceaseless Void reach the backrow, which has no unit stats.
const FIELD_SPELL: &str = "bw-field-spell";

fn defs() -> Vec<CardDef> {
    vec![
        unit_def_of("human", 1301, json!({ "tags": ["Human"] })),
        unit_def_of("felinor", 1302, json!({ "tags": ["Felinor"] })),
        unit_def_of("beast", 1303, json!({})),
        unit_def_of(
            "field-spell",
            1304,
            json!({
                "type": "Field Spell",
                "base": { "keywords": [], "text": "field spell" },
                "radiant": { "keywords": [], "text": "field spell" },
            }),
        ),
    ]
}

fn rush_token() -> String {
    token_def("rush").id
}

/// A fresh game whose catalog also carries this file's fixtures; p1 is active and resolving.
fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    state.turn = 3;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    state
}

#[derive(Default)]
struct RunOptions<'a> {
    controller: Option<PlayerId>,
    self_: Option<&'a CardInstance>,
}

/// A sink plus `apply`, so one test can run a sweep and then the state check on the same events.
/// It holds the state (TS's runner shared it), the events every apply appends to and the rng.
struct Runner {
    state: GameState,
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Runner {
    fn new(state: GameState) -> Runner {
        let rng = Rng::new(&state.seed, state.rng_cursor);
        Runner { state, events: Vec::new(), rng }
    }

    fn context_apply(&mut self, effects: &[Effect], target: Option<&CardInstance>, options: RunOptions<'_>) {
        let targets: Vec<Selection> =
            target.map(|target| vec![Selection::Instance { instance_id: target.id.clone() }]).unwrap_or_default();
        // TS handed over the live object: read it back as it stands now.
        let self_ = options.self_.map(|card| find_instance(&self.state, &card.id).cloned().unwrap_or_else(|| card.clone()));
        {
            let sink = EngineSink::new(&mut self.state, &mut self.events, &mut self.rng);
            let mut ctx = make_context(
                sink,
                self_,
                HookOptions {
                    controller: Some(options.controller.unwrap_or(PlayerId::P1)),
                    targets: Some(targets),
                    ..Default::default()
                },
            );
            for effect in effects {
                (effect.apply)(&mut ctx);
            }
        }
        self.state.rng_cursor = self.rng.cursor();
    }

    fn apply(&mut self, effect: Effect, target: Option<&CardInstance>, options: RunOptions<'_>) {
        self.context_apply(&[effect], target, options);
    }

    // #16 Hit Job and #34 Collateral Damage pair a single-target verb with an adjacency verb in one
    // effect list, which must resolve against one context and one state check (R59).
    fn apply_all(&mut self, effects: Vec<Effect>, target: Option<&CardInstance>, options: RunOptions<'_>) {
        self.context_apply(&effects, target, options);
    }

    fn check(&mut self) {
        let mut sink = EngineSink::new(&mut self.state, &mut self.events, &mut self.rng);
        state_check(&mut sink);
    }

    fn card(&self, card: &CardInstance) -> CardInstance {
        find_instance(&self.state, &card.id).expect("the card is in the state").clone()
    }

    fn marked(&self, card: &CardInstance) -> Option<bool> {
        self.card(card).marked_destroyed
    }

    fn id_at(&self, player: PlayerId, row: Row, lane: i32) -> Option<String> {
        card_at(&self.state, slot(player, row, lane)).map(|card| card.id.clone())
    }

    fn graveyard(&self, player: PlayerId) -> Vec<String> {
        ids_of(&self.state.players[player].graveyard)
    }

    fn hand(&self, player: PlayerId) -> Vec<String> {
        ids_of(&self.state.players[player].hand)
    }

    fn exile(&self, player: PlayerId) -> Vec<String> {
        ids_of(&self.state.players[player].exile)
    }

    /// R11: a card that ceased to exist is in no pile of the state (TS read the detached instance's
    /// `zone.z === "gone"`).
    fn gone(&self, card: &CardInstance) -> bool {
        find_instance(&self.state, &card.id).is_none()
    }

    fn damage_targets(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, .. } => Some(target_id.clone()),
                _ => None,
            })
            .collect()
    }
}

fn defaults() -> RunOptions<'static> {
    RunOptions::default()
}

fn with_self(card: &CardInstance) -> RunOptions<'_> {
    RunOptions { controller: None, self_: Some(card) }
}

/// The one card a single-card fixture call made, without an optional chain in the assertion.
fn only(cards: Vec<CardInstance>) -> CardInstance {
    if cards.len() != 1 {
        panic!("expected exactly one fixture card");
    }
    cards.into_iter().next().expect("expected exactly one fixture card")
}

/// A card owned by one player but standing on the other's field, for R12's "owner's graveyard".
fn stolen_onto(state: &mut GameState, def_id: &str, owner: PlayerId, at: ZoneSlot) -> CardInstance {
    let card = new_instance(state, def_id, owner, Zone::Hand { player: owner });
    if !place_on_field(state, card.clone(), at, Default::default()) {
        panic!("could not place the stolen fixture");
    }
    card
}

fn ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

fn instance_ids_of(events: &[GameEvent], kind: GameEventType) -> Vec<String> {
    events
        .iter()
        .filter(|event| event.event_type() == kind)
        .filter_map(|event| serde_json::to_value(event).ok()?.get("instanceId")?.as_str().map(str::to_string))
        .collect()
}

/// TS `const chosen = { of: "chosen" } as const`.
fn chosen() -> Value {
    json!({ "of": "chosen" })
}

// ---------------------------------------------------------------------------
// destroyAll
// ---------------------------------------------------------------------------

mod destroy_all_s6_3_s4_5_r46_r59_m3_t1 {
    use super::*;

    #[test]
    fn marks_every_matching_enemy_unit_leaves_allies_and_other_tags_standing_and_one_state_check_buries_them_in_their_owners_graveyards_r12_r59(
    ) {
        // §6.3.
        let mut state = game("destroyAll-scope");
        let ally = put(&mut state, BEAST, slot(PlayerId::P1, Row::Units, 1));
        let ally_human = put(&mut state, HUMAN, slot(PlayerId::P1, Row::Units, 2));
        let enemy_one = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 1));
        let enemy_human = put(&mut state, HUMAN, slot(PlayerId::P2, Row::Units, 2));
        let enemy_two = put(&mut state, FELINOR, slot(PlayerId::P2, Row::Units, 3));
        // R12: owned by p1, standing on p2's side, so the sweep's scope is by side and the graveyard
        // is by owner. These are two different questions and this card answers both at once.
        let stolen = stolen_onto(&mut state, BEAST, PlayerId::P1, slot(PlayerId::P2, Row::Units, 4));
        let mut run = Runner::new(state);

        run.apply(
            destroy_all(json_as(json!({ "side": "enemy", "rows": ["units"], "notTags": ["Human"] }))),
            None,
            defaults(),
        );

        // Only marks: §4.5 step 1 has not run, so nothing has moved and nothing has been announced.
        assert_eq!(
            [&enemy_one, &enemy_two, &stolen].map(|card| run.marked(card)),
            [Some(true), Some(true), Some(true)]
        );
        assert_eq!(run.marked(&enemy_human), None);
        assert_eq!(run.marked(&ally), None);
        assert_eq!(run.marked(&ally_human), None);
        assert_eq!(run.id_at(PlayerId::P2, Row::Units, 1), Some(enemy_one.id.clone()));
        assert_eq!(run.id_at(PlayerId::P2, Row::Units, 3), Some(enemy_two.id.clone()));
        assert_eq!(run.state.players[PlayerId::P1].graveyard.len(), 0);
        assert_eq!(run.state.players[PlayerId::P2].graveyard.len(), 0);
        assert_eq!(run.events, Vec::<GameEvent>::new());

        run.check();

        assert_eq!(run.id_at(PlayerId::P2, Row::Units, 1), None);
        assert_eq!(run.id_at(PlayerId::P2, Row::Units, 3), None);
        assert_eq!(run.id_at(PlayerId::P2, Row::Units, 4), None);
        assert_eq!(run.id_at(PlayerId::P2, Row::Units, 2), Some(enemy_human.id.clone()));
        assert_eq!(run.id_at(PlayerId::P1, Row::Units, 1), Some(ally.id.clone()));
        assert_eq!(run.id_at(PlayerId::P1, Row::Units, 2), Some(ally_human.id.clone()));
        // R12: the two p2-owned bodies to p2's graveyard, the stolen p1-owned one to p1's.
        assert_eq!(sorted(run.graveyard(PlayerId::P2)), sorted(vec![enemy_one.id.clone(), enemy_two.id.clone()]));
        assert_eq!(run.graveyard(PlayerId::P1), vec![stolen.id.clone()]);
        // R59: one state check collected all three, so there are exactly three deaths from one pass.
        assert_eq!(events_of_type(&run.events, GameEventType::Destroyed).len(), 3);
    }

    #[test]
    fn rows_units_backrow_marks_a_backrow_card_too_c88_twisting_nether() {
        // §6.3: rows ["units", "backrow"] (#88 Twisting Nether).
        let mut state = game("destroyAll-backrow");
        let ally_unit = put(&mut state, BEAST, slot(PlayerId::P1, Row::Units, 1));
        let ally_backrow = put(&mut state, FIELD_SPELL, slot(PlayerId::P1, Row::Backrow, 1));
        let enemy_unit = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 2));
        let enemy_backrow = put(&mut state, FIELD_SPELL, slot(PlayerId::P2, Row::Backrow, 3));
        let mut run = Runner::new(state);

        run.apply(destroy_all(json_as(json!({ "side": "any", "rows": ["units", "backrow"] }))), None, defaults());

        assert_eq!(
            [&ally_unit, &ally_backrow, &enemy_unit, &enemy_backrow].map(|card| run.marked(card)),
            [Some(true), Some(true), Some(true), Some(true)]
        );

        run.check();

        assert_eq!(run.id_at(PlayerId::P1, Row::Backrow, 1), None);
        assert_eq!(run.id_at(PlayerId::P2, Row::Backrow, 3), None);
        assert_eq!(sorted(run.graveyard(PlayerId::P1)), sorted(vec![ally_unit.id.clone(), ally_backrow.id.clone()]));
        assert_eq!(sorted(run.graveyard(PlayerId::P2)), sorted(vec![enemy_unit.id.clone(), enemy_backrow.id.clone()]));
    }

    #[test]
    fn defaults_to_the_unit_row_so_a_backrow_card_is_untouched_c2_c17_c43() {
        // §3.2 (#2, #17, #43).
        let mut state = game("destroyAll-default-rows");
        let enemy_unit = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 1));
        let enemy_backrow = put(&mut state, FIELD_SPELL, slot(PlayerId::P2, Row::Backrow, 1));
        let mut run = Runner::new(state);

        run.apply(destroy_all(json_as(json!({ "side": "enemy" }))), None, defaults());

        assert_eq!(run.marked(&enemy_unit), Some(true));
        assert_eq!(run.marked(&enemy_backrow), None);

        run.check();

        assert_eq!(run.id_at(PlayerId::P2, Row::Backrow, 1), Some(enemy_backrow.id.clone()));
    }

    #[test]
    fn r46_does_not_skip_an_indestructible_unit_the_mark_lands_and_s4_5_is_what_spares_it_flattens_it_to_attack_position_and_drops_its_taunt(
    ) {
        let mut state = game("destroyAll-indestructible");
        let warded = put(&mut state, &indestructible().id, slot(PlayerId::P2, Row::Units, 1));
        find_instance_mut(&mut state, &warded.id).expect("the warded unit").position = Some(Position::Def);
        let mortal = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 2));
        let mut run = Runner::new(state);

        run.apply(destroy_all(json_as(json!({ "side": "enemy" }))), None, defaults());

        // The whole point: the scope does NOT pre-filter Indestructible. Were the mark never set,
        // R46's Attack-Position switch and Taunt suppression below would silently never happen.
        assert_eq!(run.marked(&warded), Some(true));
        assert_eq!(run.marked(&mortal), Some(true));

        run.check();

        assert_eq!(run.id_at(PlayerId::P2, Row::Units, 1), Some(warded.id.clone()));
        assert_eq!(run.marked(&warded), Some(false));
        assert_eq!(run.card(&warded).position, Some(Position::Atk));
        assert_eq!(run.card(&warded).taunt_suppressed_turn, Some(run.state.turn));
        assert_eq!(run.graveyard(PlayerId::P2), vec![mortal.id.clone()]);
    }

    #[test]
    fn marks_only_the_top_of_a_stack_pile_never_the_dormant_card_underneath_r13() {
        // §3.2.
        let mut state = game("destroyAll-stack");
        let dormant = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 1));
        let top = new_instance(&mut state, BEAST, PlayerId::P2, Zone::Hand { player: PlayerId::P2 });
        assert!(place_on_field(&mut state, top.clone(), slot(PlayerId::P2, Row::Units, 1), json_as(json!({ "stack": true }))));
        let mut run = Runner::new(state);

        run.apply(destroy_all(json_as(json!({ "side": "enemy" }))), None, defaults());

        assert_eq!(run.marked(&top), Some(true));
        assert_eq!(run.marked(&dormant), None);

        run.check();

        // The top card left; the card beneath resumes acting rather than dying with it.
        assert_eq!(run.graveyard(PlayerId::P2), vec![top.id.clone()]);
        assert_eq!(run.id_at(PlayerId::P2, Row::Units, 1), Some(dormant.id.clone()));
    }
}

// ---------------------------------------------------------------------------
// destroyAdjacentTo
// ---------------------------------------------------------------------------

mod destroy_adjacent_to_s3_1_m3_t1 {
    use super::*;

    #[test]
    fn marks_lanes_n_1_and_n_1_on_the_target_s_side_and_row_never_lane_n_never_across_sides_never_across_rows_c16_hit_job() {
        // §3.1: lanes N-1 and N+1.
        let mut state = game("destroyAdjacent-scope");
        let left = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 2));
        let target = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 3));
        let right = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 4));
        let across_side = put(&mut state, BEAST, slot(PlayerId::P1, Row::Units, 2));
        let same_lane_other_side = put(&mut state, BEAST, slot(PlayerId::P1, Row::Units, 3));
        let across_row = put(&mut state, FIELD_SPELL, slot(PlayerId::P2, Row::Backrow, 2));
        let mut run = Runner::new(state);

        run.apply(destroy_adjacent_to(json_as(json!({ "target": chosen() }))), Some(&target), defaults());

        assert_eq!(run.marked(&left), Some(true));
        assert_eq!(run.marked(&right), Some(true));
        // Lane N itself: #16 marks it with its own `destroy`, not with this verb.
        assert_eq!(run.marked(&target), None);
        assert_eq!(run.marked(&across_side), None);
        assert_eq!(run.marked(&same_lane_other_side), None);
        assert_eq!(run.marked(&across_row), None);
    }

    #[test]
    fn an_edge_lane_has_one_neighbour_and_c16_s_pairing_kills_the_target_with_it_in_one_state_check_r59() {
        // §3.1.
        let mut state = game("destroyAdjacent-edge");
        let target = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 1));
        let right = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 2));
        let far = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 3));
        let mut run = Runner::new(state);

        // Exactly what 016-hit-job.ts returns from its radiant Cry.
        run.apply_all(
            vec![
                destroy(json_as(json!({ "target": chosen() }))),
                destroy_adjacent_to(json_as(json!({ "target": chosen() }))),
            ],
            Some(&target),
            defaults(),
        );

        assert_eq!([run.marked(&target), run.marked(&right)], [Some(true), Some(true)]);
        assert_eq!(run.marked(&far), None);

        run.check();

        assert_eq!(sorted(run.graveyard(PlayerId::P2)), sorted(vec![target.id.clone(), right.id.clone()]));
        assert_eq!(events_of_type(&run.events, GameEventType::Destroyed).len(), 2);
        assert_eq!(run.id_at(PlayerId::P2, Row::Units, 3), Some(far.id.clone()));
    }

    #[test]
    fn fizzles_silently_when_the_target_is_not_on_the_field_and_the_card_still_resolves() {
        // §3.1.
        let mut state = game("destroyAdjacent-fizzle");
        let in_hand_card = only(in_hand(&mut state, BEAST, PlayerId::P1, 1));
        let neighbour = put(&mut state, BEAST, slot(PlayerId::P1, Row::Units, 1));
        let mut run = Runner::new(state);

        run.apply(destroy_adjacent_to(json_as(json!({ "target": chosen() }))), Some(&in_hand_card), defaults());

        assert_eq!(run.marked(&neighbour), None);
        assert_eq!(run.events, Vec::<GameEvent>::new());
    }
}

// ---------------------------------------------------------------------------
// damageAll
// ---------------------------------------------------------------------------

mod damage_all_s6_3_s4_4_r59_m3_t1 {
    use super::*;

    #[test]
    fn deals_one_instance_to_each_enemy_unit_and_with_heroes_to_the_enemy_hero_leaving_allies_untouched_c13_jlockeed_shredder_10(
    ) {
        // §4.4.
        let mut state = game("damageAll-enemies");
        let self_card = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 1));
        let ally = put(&mut state, BEAST, slot(PlayerId::P1, Row::Units, 2));
        let first = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 1));
        let second = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 2));
        let third = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 3));
        let mut run = Runner::new(state);

        run.apply(
            damage_all(json_as(json!({ "side": "enemy", "amount": 2, "heroes": true }))),
            None,
            with_self(&self_card),
        );

        assert_eq!([&first, &second, &third].map(|card| run.card(card).damage), [2, 2, 2]);
        assert_eq!(run.card(&ally).damage, 0);
        assert_eq!(run.card(&self_card).damage, 0);
        assert_eq!(run.state.players[PlayerId::P2].hero.health, HERO_HEALTH - 2);
        assert_eq!(run.state.players[PlayerId::P1].hero.health, HERO_HEALTH);
        // Units in lane order first, then the hero of the scoped side.
        assert_eq!(
            run.damage_targets(),
            vec![first.id.clone(), second.id.clone(), third.id.clone(), "hero-p2".to_string()]
        );
        assert!(run.events.iter().all(|event| match event {
            GameEvent::Damage { source_id, .. } => source_id.as_deref() == Some(self_card.id.as_str()),
            _ => true,
        }));
    }

    #[test]
    fn snapshots_its_targets_before_the_first_hit_one_damage_event_per_unit_present_when_the_sweep_began_and_no_unit_hit_twice_r59(
    ) {
        // §4.4.
        let mut state = game("damageAll-snapshot");
        let self_card = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 1));
        let present: Vec<CardInstance> =
            [1, 2, 3].iter().map(|lane| put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, *lane))).collect();
        let mut run = Runner::new(state);

        // 2 damage on 2-health bodies: every one of them is at 0 health by the second hit, and §4.5
        // never runs between the hits of one effect (R59), so all three are still standing here.
        run.apply(damage_all(json_as(json!({ "side": "enemy", "amount": 2 }))), None, with_self(&self_card));

        let hits = run.damage_targets();
        assert_eq!(hits.len(), present.len());
        assert_eq!(hits, present.iter().map(|card| card.id.clone()).collect::<Vec<_>>());
        assert_eq!(hits.iter().collect::<BTreeSet<_>>().len(), present.len());
        assert_eq!(present.iter().map(|card| run.card(card).damage).collect::<Vec<_>>(), vec![2, 2, 2]);

        // The list belongs to the apply that read it: a unit that joins the board afterwards took
        // nothing from the first sweep and is hit by the next one. No engine verb can move a card off
        // the field from inside `dealDamage`, so the snapshot's other half — a unit that dies mid-sweep
        // not changing who else is hit — is asserted above as "each target hit exactly once, in
        // `cardsInScope` order", which a per-hit re-read of the row could not promise.
        let latecomer = put(&mut run.state, BEAST, slot(PlayerId::P2, Row::Units, 4));
        assert_eq!(events_of_type(&run.events, GameEventType::Damage).len(), present.len());

        run.apply(damage_all(json_as(json!({ "side": "enemy", "amount": 1 }))), None, with_self(&self_card));

        assert_eq!(run.card(&latecomer).damage, 1);
        assert_eq!(events_of_type(&run.events, GameEventType::Damage).len(), present.len() + present.len() + 1);
    }

    #[test]
    fn puts_every_hit_through_the_whole_pipeline_divine_shield_eats_one_armor_soaks_one_indestructible_takes_none_and_the_sweep_carries_on(
    ) {
        // §4.4.
        let mut state = game("damageAll-pipeline");
        let self_card = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 1));
        let shield = put(&mut state, &shielded().id, slot(PlayerId::P2, Row::Units, 1));
        let armor = put(&mut state, &armoured().id, slot(PlayerId::P2, Row::Units, 2));
        let warded = put(&mut state, &indestructible().id, slot(PlayerId::P2, Row::Units, 3));
        let soft = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 4));
        let mut run = Runner::new(state);

        run.apply(damage_all(json_as(json!({ "side": "enemy", "amount": 3 }))), None, with_self(&self_card));

        assert_eq!(run.card(&shield).damage, 0);
        assert_eq!(run.card(&shield).divine_shield_spent, Some(true));
        assert_eq!(run.card(&armor).damage, 0);
        assert_eq!(run.card(&warded).damage, 0);
        assert_eq!(run.card(&soft).damage, 3);
        assert_eq!(instance_ids_of(&run.events, GameEventType::DivineShieldLost), vec![shield.id.clone()]);
        assert_eq!(run.damage_targets(), vec![soft.id.clone()]);
    }

    #[test]
    fn ignore_armor_reaches_the_same_armor_7_body_the_plain_sweep_could_not_true_strike_s4_4_step_2() {
        // §4.4.
        let mut state = game("damageAll-true-strike");
        let self_card = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 1));
        let armor = put(&mut state, &armoured().id, slot(PlayerId::P2, Row::Units, 1));
        let mut run = Runner::new(state);

        run.apply(
            damage_all(json_as(json!({ "side": "enemy", "amount": 3, "ignoreArmor": true }))),
            None,
            with_self(&self_card),
        );

        assert_eq!(run.card(&armor).damage, 3);
        assert_eq!(run.damage_targets(), vec![armor.id.clone()]);
    }

    #[test]
    fn r68_heroes_follows_the_scope_s_sides_side_any_hits_both_heroes_the_active_player_s_first() {
        let mut state = game("damageAll-both-heroes");
        let self_card = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 1));
        let mut run = Runner::new(state);

        run.apply(
            damage_all(json_as(json!({ "side": "any", "amount": 1, "heroes": true }))),
            None,
            with_self(&self_card),
        );

        assert_eq!(run.state.players[PlayerId::P1].hero.health, HERO_HEALTH - 1);
        assert_eq!(run.state.players[PlayerId::P2].hero.health, HERO_HEALTH - 1);
        assert_eq!(
            run.damage_targets(),
            vec![self_card.id.clone(), "hero-p1".to_string(), "hero-p2".to_string()]
        );
    }

    #[test]
    fn a_sweep_over_an_empty_board_still_hits_the_scoped_hero() {
        // §6.3.
        let state = game("damageAll-empty-board");
        let mut run = Runner::new(state);

        run.apply(damage_all(json_as(json!({ "side": "enemy", "amount": 4, "heroes": true }))), None, defaults());

        assert_eq!(run.state.players[PlayerId::P2].hero.health, HERO_HEALTH - 4);
        assert_eq!(run.damage_targets(), vec!["hero-p2".to_string()]);
    }
}

// ---------------------------------------------------------------------------
// bounceAll
// ---------------------------------------------------------------------------

mod bounce_all_s6_3_s3_2_r11_r78_r747_m3_t1 {
    use super::*;

    #[test]
    fn r747_returns_real_cards_to_their_controllers_hands_and_r11_makes_a_unit_token_cease_to_exist_c17_flood() {
        let mut state = game("bounceAll-owners");
        let mine = put(&mut state, BEAST, slot(PlayerId::P1, Row::Units, 1));
        {
            let card = find_instance_mut(&mut state, &mine.id).expect("mine");
            card.damage = 1;
            card.buffs = AttackHealth { attack: 3, health: 3 };
        }
        let token = put(&mut state, &rush_token(), slot(PlayerId::P1, Row::Units, 2));
        let theirs = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 1));
        // R747: p1 owns it, p2 is standing it up; a bounce sends it to its controller p2's hand,
        // not its owner's — bouncing an enemy permanent never fills your own hand.
        let stolen = stolen_onto(&mut state, BEAST, PlayerId::P1, slot(PlayerId::P2, Row::Units, 2));
        let mut run = Runner::new(state);

        run.apply(bounce_all(json_as(json!({ "side": "any" }))), None, defaults());

        assert_eq!(run.hand(PlayerId::P1), vec![mine.id.clone()]);
        assert_eq!(run.hand(PlayerId::P2), vec![theirs.id.clone(), stolen.id.clone()]);
        // R747: it lands in p2's hand as p2's own card.
        assert_eq!(run.card(&stolen).owner, PlayerId::P2);
        assert_eq!(run.card(&stolen).controller, PlayerId::P2);
        assert_eq!(run.card(&stolen).zone, Zone::Hand { player: PlayerId::P2 });
        // R11: the token reached no hand at all and is not in either player's pile.
        assert!(run.gone(&token));
        assert!(!run.hand(PlayerId::P1).contains(&token.id));
        assert_eq!(run.state.players[PlayerId::P1].graveyard.len(), 0);
        // R78: the instance reset on the way off the field.
        assert_eq!(run.card(&mine).damage, 0);
        assert_eq!(run.card(&mine).buffs, AttackHealth { attack: 0, health: 0 });
        assert_eq!(
            instance_ids_of(&run.events, GameEventType::Bounced),
            vec![mine.id.clone(), token.id.clone(), theirs.id.clone(), stolen.id.clone()]
        );
        for lane in [1, 2] {
            assert_eq!(run.id_at(PlayerId::P1, Row::Units, lane), None);
            assert_eq!(run.id_at(PlayerId::P2, Row::Units, lane), None);
        }
    }

    #[test]
    fn the_hand_cap_applies_inside_the_sweep_so_a_real_card_is_burned_to_the_graveyard() {
        // §2.4.
        let mut state = game("bounceAll-hand-cap");
        in_hand(&mut state, BEAST, PlayerId::P1, HAND_CAP as _);
        let mine = put(&mut state, BEAST, slot(PlayerId::P1, Row::Units, 1));
        let mut run = Runner::new(state);

        run.apply(bounce_all(json_as(json!({ "side": "self" }))), None, defaults());

        assert_eq!(run.state.players[PlayerId::P1].hand.len(), HAND_CAP as usize);
        assert!(!run.hand(PlayerId::P1).contains(&mine.id));
        assert_eq!(run.graveyard(PlayerId::P1), vec![mine.id.clone()]);
        assert_eq!(instance_ids_of(&run.events, GameEventType::Burned), vec![mine.id.clone()]);
    }

    #[test]
    fn defaults_to_the_unit_row_bounce_all_units_leaves_the_backrow_in_place() {
        // §3.2.
        let mut state = game("bounceAll-default-rows");
        let unit = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 1));
        let backrow = put(&mut state, FIELD_SPELL, slot(PlayerId::P2, Row::Backrow, 1));
        let mut run = Runner::new(state);

        run.apply(bounce_all(json_as(json!({ "side": "enemy" }))), None, defaults());

        assert_eq!(run.hand(PlayerId::P2), vec![unit.id.clone()]);
        assert_eq!(run.id_at(PlayerId::P2, Row::Backrow, 1), Some(backrow.id.clone()));
    }
}

// ---------------------------------------------------------------------------
// exileAll
// ---------------------------------------------------------------------------

mod exile_all_s6_3_r11_r55_m3_t1 {
    use super::*;

    #[test]
    fn exiles_every_permanent_on_both_sides_but_the_running_card_and_bumps_counters_exiled_once_per_card_that_reached_the_pile_c100_ceaseless_void(
    ) {
        // §6.3.
        let mut state = game("exileAll-void");
        let self_card = put(&mut state, BEAST, slot(PlayerId::P1, Row::Units, 1));
        let my_other = put(&mut state, BEAST, slot(PlayerId::P1, Row::Units, 2));
        let my_token = put(&mut state, &rush_token(), slot(PlayerId::P1, Row::Units, 3));
        let my_backrow = put(&mut state, FIELD_SPELL, slot(PlayerId::P1, Row::Backrow, 1));
        let their_unit = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 1));
        let their_backrow = put(&mut state, FIELD_SPELL, slot(PlayerId::P2, Row::Backrow, 2));
        let mut run = Runner::new(state);

        run.apply(
            exile_all(json_as(json!({ "side": "any", "rows": ["units", "backrow"], "excludeSelf": true }))),
            None,
            with_self(&self_card),
        );

        // excludeSelf: the Void is still standing after its own Cry.
        assert_eq!(run.id_at(PlayerId::P1, Row::Units, 1), Some(self_card.id.clone()));
        assert_eq!(run.exile(PlayerId::P1), vec![my_other.id.clone(), my_backrow.id.clone()]);
        assert_eq!(run.exile(PlayerId::P2), vec![their_unit.id.clone(), their_backrow.id.clone()]);
        // R11: the token ceased to exist instead of reaching a pile, so it is not counted (R55)...
        assert!(run.gone(&my_token));
        assert_eq!(run.state.counters.exiled, 4);
        // ... while the event still reports it leaving.
        assert_eq!(
            instance_ids_of(&run.events, GameEventType::Exiled),
            vec![
                my_other.id.clone(),
                my_token.id.clone(),
                my_backrow.id.clone(),
                their_unit.id.clone(),
                their_backrow.id.clone(),
            ]
        );
        assert_eq!(run.id_at(PlayerId::P2, Row::Backrow, 2), None);
    }

    #[test]
    fn without_exclude_self_the_sweep_takes_the_running_card_too() {
        // §6.3.
        let mut state = game("exileAll-includes-self");
        let self_card = put(&mut state, BEAST, slot(PlayerId::P1, Row::Units, 1));
        let mut run = Runner::new(state);

        run.apply(exile_all(json_as(json!({ "side": "self" }))), None, with_self(&self_card));

        assert_eq!(run.id_at(PlayerId::P1, Row::Units, 1), None);
        assert_eq!(run.exile(PlayerId::P1), vec![self_card.id.clone()]);
        assert_eq!(run.state.counters.exiled, 1);
    }
}

// ---------------------------------------------------------------------------
// exileAdjacentTo
// ---------------------------------------------------------------------------

mod exile_adjacent_to_s3_1_m3_t1 {
    use super::*;

    #[test]
    fn exiles_the_neighbours_in_the_target_s_row_and_leaves_the_other_row_and_the_target_alone_c34_collateral_damage() {
        // §3.1.
        let mut state = game("exileAdjacent-row");
        let left = put(&mut state, FIELD_SPELL, slot(PlayerId::P2, Row::Backrow, 2));
        let target = put(&mut state, FIELD_SPELL, slot(PlayerId::P2, Row::Backrow, 3));
        let right = put(&mut state, FIELD_SPELL, slot(PlayerId::P2, Row::Backrow, 4));
        let unit_beside = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 2));
        let unit_behind = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 3));
        let mut run = Runner::new(state);

        run.apply(exile_adjacent_to(json_as(json!({ "target": chosen() }))), Some(&target), defaults());

        assert_eq!(run.exile(PlayerId::P2), vec![left.id.clone(), right.id.clone()]);
        assert_eq!(run.state.counters.exiled, 2);
        // Adjacency never crosses rows, so "in its row" needs no argument of its own.
        assert_eq!(run.id_at(PlayerId::P2, Row::Backrow, 3), Some(target.id.clone()));
        assert_eq!(run.id_at(PlayerId::P2, Row::Units, 2), Some(unit_beside.id.clone()));
        assert_eq!(run.id_at(PlayerId::P2, Row::Units, 3), Some(unit_behind.id.clone()));
    }

    #[test]
    fn an_empty_neighbouring_lane_contributes_nothing_and_the_effect_still_resolves() {
        // §3.1.
        let mut state = game("exileAdjacent-gap");
        let target = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 3));
        let right = put(&mut state, BEAST, slot(PlayerId::P2, Row::Units, 4));
        let mut run = Runner::new(state);

        run.apply(exile_adjacent_to(json_as(json!({ "target": chosen() }))), Some(&target), defaults());

        assert_eq!(run.exile(PlayerId::P2), vec![right.id.clone()]);
        assert_eq!(run.id_at(PlayerId::P2, Row::Units, 3), Some(target.id.clone()));
    }
}

// ---------------------------------------------------------------------------
// discardHand
// ---------------------------------------------------------------------------

mod discard_hand_s6_3_r16_r31_s10_7_m3_t1 {
    use super::*;

    #[test]
    fn r31_empties_the_hand_into_the_graveyard_in_hand_order_c76_field_of_dreams() {
        let mut state = game("discardHand-order");
        let cards = in_hand(&mut state, BEAST, PlayerId::P1, 3);
        let mut run = Runner::new(state);

        run.apply(discard_hand(json_as(json!({ "player": "self" }))), None, defaults());

        let ids: Vec<String> = cards.iter().map(|card| card.id.clone()).collect();
        assert_eq!(run.state.players[PlayerId::P1].hand.len(), 0);
        assert_eq!(run.graveyard(PlayerId::P1), ids);
        assert_eq!(instance_ids_of(&run.events, GameEventType::Discarded), ids);
        assert_eq!(instance_ids_of(&run.events, GameEventType::EnteredGraveyard), ids);
    }

    #[test]
    fn draws_nothing_from_the_rng_so_rng_cursor_and_every_downstream_replay_hash_are_unchanged() {
        // §10.7.
        let mut state = game("discardHand-rng");
        in_hand(&mut state, BEAST, PlayerId::P1, 3);
        let mut run = Runner::new(state);
        let before = run.rng.cursor();

        run.apply(discard_hand(json_as(json!({ "player": "self" }))), None, defaults());

        assert_eq!(run.state.players[PlayerId::P1].hand.len(), 0);
        assert_eq!(run.rng.cursor(), before);
        assert_eq!(run.state.rng_cursor, before);

        // Not a vacuous assertion: the random form of the same sweep does move the cursor, which is
        // exactly why a whole-hand discard must not be written as `discardRandom({ count: n })`.
        in_hand(&mut run.state, BEAST, PlayerId::P1, 3);
        run.apply(discard_random(json_as(json!({ "count": 3, "player": "self" }))), None, defaults());
        assert!(run.rng.cursor() > before);
    }

    #[test]
    fn r11_a_unit_token_card_in_hand_ceases_to_exist_rather_than_reaching_the_graveyard() {
        let mut state = game("discardHand-token");
        let real = only(in_hand(&mut state, BEAST, PlayerId::P1, 1));
        let token = only(in_hand(&mut state, &rush_token(), PlayerId::P1, 1));
        let mut run = Runner::new(state);

        run.apply(discard_hand(json_as(json!({ "player": "self" }))), None, defaults());

        assert_eq!(run.state.players[PlayerId::P1].hand.len(), 0);
        assert_eq!(run.graveyard(PlayerId::P1), vec![real.id.clone()]);
        assert!(run.gone(&token));
        assert_eq!(events_of_type(&run.events, GameEventType::Discarded).len(), 2);
        assert_eq!(instance_ids_of(&run.events, GameEventType::EnteredGraveyard), vec![real.id.clone()]);
    }

    #[test]
    fn player_enemy_empties_the_opponent_s_hand_and_leaves_the_controller_s_alone() {
        // §6.3.
        let mut state = game("discardHand-enemy");
        let mine = in_hand(&mut state, BEAST, PlayerId::P1, 2);
        let theirs = in_hand(&mut state, BEAST, PlayerId::P2, 2);
        let mut run = Runner::new(state);

        run.apply(discard_hand(json_as(json!({ "player": "enemy" }))), None, defaults());

        assert_eq!(run.hand(PlayerId::P1), mine.iter().map(|card| card.id.clone()).collect::<Vec<_>>());
        assert_eq!(run.state.players[PlayerId::P2].hand.len(), 0);
        assert_eq!(run.graveyard(PlayerId::P2), theirs.iter().map(|card| card.id.clone()).collect::<Vec<_>>());
    }
}

// ---------------------------------------------------------------------------
// exileHand
// ---------------------------------------------------------------------------

mod exile_hand_s6_3_r11_r55_s10_7_m3_t1 {
    use super::*;

    #[test]
    fn empties_the_hand_into_the_exile_pile_and_bumps_counters_exiled_once_per_card_that_got_there_c78_fullsend() {
        // §6.3.
        let mut state = game("exileHand-sweep");
        let cards = in_hand(&mut state, BEAST, PlayerId::P1, 3);
        let token = only(in_hand(&mut state, &rush_token(), PlayerId::P1, 1));
        let mut run = Runner::new(state);
        let before = run.rng.cursor();

        run.apply(exile_hand(json_as(json!({ "player": "self" }))), None, defaults());

        assert_eq!(run.state.players[PlayerId::P1].hand.len(), 0);
        assert_eq!(run.exile(PlayerId::P1), cards.iter().map(|card| card.id.clone()).collect::<Vec<_>>());
        // R11: the unit-token card ceased to exist, so it is not in the pile and not counted (R55).
        assert!(run.gone(&token));
        assert_eq!(run.state.counters.exiled, 3);
        assert_eq!(events_of_type(&run.events, GameEventType::Exiled).len(), 4);
        assert_eq!(run.state.players[PlayerId::P1].graveyard.len(), 0);
        // The same determinism as `discardHand`: no choice, so no rng draw (§10.7).
        assert_eq!(run.rng.cursor(), before);
    }

    #[test]
    fn does_nothing_to_an_empty_hand_and_the_card_still_resolves() {
        // §6.3.
        let state = game("exileHand-empty");
        let mut run = Runner::new(state);

        run.apply(exile_hand(json_as(json!({ "player": "self" }))), None, defaults());

        assert_eq!(run.state.counters.exiled, 0);
        assert_eq!(run.events, Vec::<GameEvent>::new());
    }
}
