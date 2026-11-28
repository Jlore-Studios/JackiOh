//! Felinors vs Humans, E2 (#5237): lasting effects, watchers, grants and auras, each proved through a
//! fixture script that calls the verb or carries the flag (docs/felinors-vs-humans-set.md FV2, FV3):
//! the summon watcher (R5237000), grants for turns (R5237001), the Float of last turn (R5237002), the
//! raised mana cap (R5237003), staying sick (R5237004), the hand place of a play (R5237005), an end of
//! turn grant (R5237006), tagged Units' stats (R5237007), a keyword lent to every card (R5237008), harm
//! that passes a tag by (R5237009), immunity to a tag (R5237010), the bit-length aura (R5237011,
//! R5237012) and the recursion log a Plagiarized card keeps (R5237013).
//!
//! The file registers its own catalog on each test's thread (`install`): script-less (0) Spells for the
//! scenario's filler deck and every library, then the test's own cards. Ids are `fvhl-…`.

use jackioh_engine::effects;
use jackioh_engine::prelude::*;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

/// The script-less (0) Spell every library holds, so no turn ends by itself and no draw fatigues.
const FILLER: &str = "fvhl-filler-0";
/// A vanilla Unit that plays for (0).
const PAWN: &str = "stand-in (0) 1/1";
/// A sturdier vanilla Unit for the field.
const WALL: &str = "stand-in (0) 1/5";
/// The bit-length aura's test Unit: (4), 5/8, so each number has a shorter bit length.
const BIG: &str = "stand-in (4) 5/8";
/// The same numbers on the field, played for (1).
const BIG_ON_FIELD: &str = "stand-in (1) 5/8";

/// What the summon watcher's step gives each Unit it answers.
const WATCH_BUFF: i32 = 1;
/// The Attack a grant for turns gives.
const TIMED_ATTACK: i32 = 2;
/// The mana cap raise.
const CAP_RAISE: i32 = 2;
/// What the end-of-turn grant deals the enemy hero each time it runs.
const GRANT_DAMAGE: i32 = 2;
/// The tagged Units' stats.
const TAG_STATS: i32 = 2;
/// The tag-immunity tests' sweep.
const BLAST_DAMAGE: i32 = 2;

/// The step the summon watcher re-enters.
const SUMMONED_STEP: &str = "summoned";
/// The key a granted end-of-turn ability is registered under on its granting card's script.
const GRANT_KEY: &str = "endTurn";

/// A Core Common fixture definition, its Radiant face the same as its base.
fn def(id: &str, index: u32, type_: &str, stats: Option<(i32, i32)>, tags: Vec<Tag>) -> CardDef {
    let face = match stats {
        Some((attack, health)) => {
            json!({ "attack": attack, "health": health, "keywords": [], "text": id })
        }
        None => json!({ "keywords": [], "text": id }),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (FvH lasting fixture)"),
        "set": "Core",
        "type": type_,
        "tags": tags,
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face,
        "radiant": face,
    }))
}

fn spell(id: &str, index: u32) -> CardDef {
    def(id, index, "Spell", None, Vec::new())
}

/// A script-less (0) Felinors vs Humans Spell for a library: the Float of last turn (R5237002) and a
/// play's hand place (R5237005) are kept only in a game holding a card of the set (FV3 E2, `fvh_kept`).
const FVH_KEEPER: &str = "fvhl-fvh-keeper";

fn fvh_keeper() -> CardDef {
    let face = json!({ "keywords": [], "text": FVH_KEEPER });
    json_as(json!({
        "id": FVH_KEEPER,
        "index": "523990",
        "name": "fvhl-fvh-keeper (FvH lasting fixture)",
        "set": "Felinors vs Humans",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face,
        "radiant": face,
    }))
}

/// p2's library with the Felinors vs Humans keeper at its bottom, so `fvh_kept` holds and no hand moves.
fn kept_library() -> Value {
    json!([FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FVH_KEEPER])
}

/// This file's catalog on the calling test's thread: `DECK_SIZE` script-less (0) Spells (the filler
/// deck and the libraries), then `cards`, each with its script on both faces.
fn install(cards: Vec<(CardDef, Script)>) {
    let mut catalog: CardDefs = IndexMap::new();
    for n in 0..DECK_SIZE {
        let filler = spell(&format!("fvhl-filler-{n}"), 523_900 + n as u32);
        catalog.insert(filler.id.clone(), filler);
    }
    let keeper = fvh_keeper();
    catalog.insert(keeper.id.clone(), keeper);
    let mut scripts: IndexMap<String, CardScripts> = IndexMap::new();
    for (card, script) in cards {
        scripts.insert(
            card.id.clone(),
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        );
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    register_scripts(scripts);
}

/// p1's turn 9, its main phase; each side as given, with fillers to draw, and no turn that ends by itself.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([FILLER, FILLER, FILLER, FILLER, FILLER, FILLER]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn lane_id(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit_at(player, lane).id
}

fn has_taunt(s: &Scenario, id: &str) -> bool {
    unit_has(s.state(), s.card(id), KeywordKind::Taunt)
}

fn hero_health(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].hero.health
}

/// A Spell whose Cry is `effect`, made fresh at each play.
fn cast(effect: impl Fn() -> Effect + Send + Sync + 'static) -> Script {
    Script {
        cry: Some(hook(move |_ctx| vec![effect()])),
        ..Script::default()
    }
}

/// A Spell aimed at one Unit of either side, its Cry `effect`.
fn aimed(effect: impl Fn() -> Effect + Send + Sync + 'static) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(move |_ctx| vec![effect()])),
        ..Script::default()
    }
}

mod r5237000_watch_summons {
    use super::*;

    const WATCHER: &str = "fvhl-watcher";

    /// Its Cry sets the watcher; its step gives the Unit it answers +1/+1.
    fn watcher() -> (CardDef, Script) {
        let script = Script {
            cry: Some(hook(|_ctx| {
                vec![effects::delay::watch_summons(WatchSummonsArgs {
                    step: SUMMONED_STEP.to_string(),
                    label: "fvh summon watch".to_string(),
                    hook: None,
                    data: None,
                    units_only: None,
                })]
            })),
            resume: IndexMap::from([(
                SUMMONED_STEP,
                hook(|ctx| {
                    let Some(id) = ctx.data.get("instanceId").and_then(Value::as_str) else {
                        return Vec::new();
                    };
                    vec![effects::buff(json_as(json!({
                        "target": { "of": "instance", "instanceId": id },
                        "attack": WATCH_BUFF,
                        "health": WATCH_BUFF,
                    })))]
                }),
            )]),
            ..Script::default()
        };
        (spell(WATCHER, 523_000), script)
    }

    #[test]
    fn r5237000_every_unit_its_owner_summons_later_takes_the_step_for_the_rest_of_the_game() {
        install(vec![watcher()]);
        let mut s = board(
            "r5237000-later",
            json!({ "hand": [WATCHER, PAWN, PAWN, PAWN] }),
            json!({ "hand": [FILLER] }),
        );
        s.play(WATCHER, json!({}));
        s.play(PAWN, json!({}));
        let first = lane_id(&s, P1, 1);
        s.expect_stats(
            first.as_str(),
            json!({ "attack": 1 + WATCH_BUFF, "health": 1 + WATCH_BUFF }),
        );
        s.play(PAWN, json!({}));
        let second = lane_id(&s, P1, 2);
        s.expect_stats(
            second.as_str(),
            json!({ "attack": 1 + WATCH_BUFF, "health": 1 + WATCH_BUFF }),
        );
        // A turn of each later, the watcher still answers.
        s.end_turn().end_turn();
        s.play(PAWN, json!({}));
        let third = lane_id(&s, P1, 3);
        s.expect_stats(
            third.as_str(),
            json!({ "attack": 1 + WATCH_BUFF, "health": 1 + WATCH_BUFF }),
        );
        // Each answer is once: the earlier Units took nothing more.
        s.expect_stats(
            first.as_str(),
            json!({ "attack": 1 + WATCH_BUFF, "health": 1 + WATCH_BUFF }),
        );
    }

    #[test]
    fn r5237000_units_on_the_field_before_it_and_the_opponents_summons_take_nothing() {
        install(vec![watcher()]);
        let mut s = board(
            "r5237000-not",
            json!({ "hand": [WATCHER], "field": [PAWN] }),
            json!({ "hand": [PAWN] }),
        );
        let before = lane_id(&s, P1, 1);
        s.play(WATCHER, json!({}));
        s.expect_stats(before.as_str(), json!({ "attack": 1, "health": 1 }));
        s.end_turn();
        s.play(PAWN, json!({}));
        let theirs = lane_id(&s, P2, 1);
        s.expect_stats(theirs.as_str(), json!({ "attack": 1, "health": 1 }));
        s.expect_stats(before.as_str(), json!({ "attack": 1, "health": 1 }));
    }
}

mod r5237001_grant_for_turns {
    use super::*;

    const ONE_TURN: &str = "fvhl-for-a-turn";
    const TWO_TURNS: &str = "fvhl-for-two-turns";

    fn timed(id: &'static str, index: u32, turns: i32) -> (CardDef, Script) {
        let script = aimed(move || {
            effects::turn_buff::grant_for_turns(GrantForTurnsArgs {
                target: TargetSpec::Chosen { index: None },
                keywords: vec![Keyword::Taunt],
                attack: Some(TIMED_ATTACK),
                turns,
            })
        });
        (spell(id, index), script)
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![timed(ONE_TURN, 523_010, 1), timed(TWO_TURNS, 523_011, 2)]
    }

    fn holds(s: &Scenario, id: &str) -> bool {
        let attack = s.stats(id).attack;
        let taunt = has_taunt(s, id);
        assert_eq!(
            attack == 1 + TIMED_ATTACK,
            taunt,
            "the Attack and the keyword go together"
        );
        taunt
    }

    #[test]
    fn r5237001_one_turn_holds_through_the_opponents_turn_and_ends_as_the_caster_starts_their_next() {
        install(cards());
        let mut s = board(
            "r5237001-one",
            json!({ "hand": [ONE_TURN, FILLER], "field": [WALL] }),
            json!({ "hand": [FILLER] }),
        );
        let wall = lane_id(&s, P1, 1);
        assert!(!holds(&s, &wall));
        s.play(ONE_TURN, pick::targets(pick::id(&wall)));
        assert!(holds(&s, &wall), "it holds as cast");
        s.end_turn();
        assert!(holds(&s, &wall), "and through the opponent's turn");
        s.end_turn();
        assert!(
            !holds(&s, &wall),
            "gone once the caster has started one more turn"
        );
    }

    #[test]
    fn r5237001_two_turns_hold_through_the_casters_next_turn() {
        install(cards());
        let mut s = board(
            "r5237001-two",
            json!({ "hand": [TWO_TURNS, FILLER], "field": [WALL] }),
            json!({ "hand": [FILLER] }),
        );
        let wall = lane_id(&s, P1, 1);
        s.play(TWO_TURNS, pick::targets(pick::id(&wall)));
        s.end_turn().end_turn();
        assert!(holds(&s, &wall), "the caster has started one more turn of two");
        s.end_turn();
        assert!(holds(&s, &wall));
        s.end_turn();
        assert!(
            !holds(&s, &wall),
            "gone at the start of the caster's second turn from now"
        );
    }

    #[test]
    fn r5237001_the_count_is_the_casters_turns_not_the_holders() {
        install(cards());
        let mut s = board(
            "r5237001-enemy",
            json!({ "hand": [ONE_TURN, FILLER] }),
            json!({ "hand": [FILLER], "field": [WALL] }),
        );
        let theirs = lane_id(&s, P2, 1);
        s.play(ONE_TURN, pick::targets(pick::id(&theirs)));
        s.end_turn();
        assert!(holds(&s, &theirs), "the holder starting a turn does not end it");
        s.end_turn();
        assert!(!holds(&s, &theirs), "the caster's next start does");
    }
}

mod r5237002_float_last_turn {
    use super::*;

    fn unspent_at_end(s: &Scenario, player: PlayerId) -> i32 {
        must(
            s.last_events().iter().find_map(|event| match event {
                GameEvent::TurnEnded {
                    player: ended,
                    unspent_mana,
                    ..
                } if *ended == player => Some(*unspent_mana),
                _ => None,
            }),
            "the turnEnded event",
        )
    }

    #[test]
    fn r5237002_is_the_mana_its_player_left_unspent_at_the_end_of_their_last_turn() {
        install(vec![]);
        let mut s = board(
            "r5237002-float",
            json!({ "mana": 3, "hand": [FILLER] }),
            json!({ "hand": [FILLER], "library": kept_library() }),
        );
        s.end_turn();
        assert_eq!(float_last_turn(s.state(), P1), 3);
        assert_eq!(unspent_at_end(&s, P1), 3);
        s.end_turn();
        let theirs = unspent_at_end(&s, P2);
        assert_eq!(float_last_turn(s.state(), P2), theirs);
        // Back on p1's next turn, their last turn is still the one that left 3.
        assert_eq!(
            float_last_turn(s.state(), P1),
            3,
            "never reset by the turn that follows"
        );
    }

    #[test]
    fn r5237002_is_0_before_its_player_has_ended_a_turn() {
        install(vec![]);
        let mut s = board(
            "r5237002-none",
            json!({ "hand": [FILLER] }),
            json!({ "hand": [FILLER], "library": kept_library() }),
        );
        assert_eq!(float_last_turn(s.state(), P1), 0);
        assert_eq!(float_last_turn(s.state(), P2), 0);
        s.end_turn();
        assert_eq!(float_last_turn(s.state(), P2), 0, "p2 has ended no turn yet");
    }

    #[test]
    fn r5237002_a_turn_that_spent_everything_floats_0() {
        install(vec![]);
        let mut s = board(
            "r5237002-spent",
            json!({ "mana": 0, "hand": [FILLER] }),
            json!({ "hand": [FILLER], "library": kept_library() }),
        );
        s.end_turn().end_turn();
        assert_eq!(float_last_turn(s.state(), P1), 0);
    }
}

mod r5237003_raise_mana_cap {
    use super::*;

    const RAISE: &str = "fvhl-raise";
    const RAISE_THEIRS: &str = "fvhl-raise-theirs";

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (
                spell(RAISE, 523_020),
                cast(|| {
                    effects::mana::raise_mana_cap(RaiseManaCapArgs {
                        amount: CAP_RAISE,
                        player: None,
                    })
                }),
            ),
            (
                spell(RAISE_THEIRS, 523_021),
                cast(|| {
                    effects::mana::raise_mana_cap(RaiseManaCapArgs {
                        amount: CAP_RAISE,
                        player: Some(PlayerSpec::Enemy),
                    })
                }),
            ),
        ]
    }

    #[test]
    fn r5237003_raises_its_players_cap_for_the_rest_of_the_game() {
        install(cards());
        let mut s = board(
            "r5237003-raise",
            json!({ "hand": [RAISE, FILLER] }),
            json!({ "hand": [FILLER] }),
        );
        s.play(RAISE, json!({}));
        assert_eq!(s.state().players[P1].mana_cap_bonus, Some(CAP_RAISE));
        s.end_turn().end_turn();
        assert_eq!(s.state().players[P1].mana.max, MAX_MANA + CAP_RAISE);
        s.end_turn().end_turn();
        assert_eq!(
            s.state().players[P1].mana.max,
            MAX_MANA + CAP_RAISE,
            "still raised, and the raised cap holds it there"
        );
    }

    #[test]
    fn r5237003_without_it_the_cap_holds_at_max_mana() {
        install(cards());
        let mut s = board(
            "r5237003-none",
            json!({ "hand": [FILLER] }),
            json!({ "hand": [FILLER] }),
        );
        s.end_turn().end_turn();
        assert_eq!(s.state().players[P1].mana.max, MAX_MANA);
        assert_eq!(s.state().players[P1].mana_cap_bonus, None);
    }

    #[test]
    fn r5237003_the_enemy_option_raises_the_opponents_cap_alone() {
        install(cards());
        let mut s = board(
            "r5237003-theirs",
            json!({ "hand": [RAISE_THEIRS, FILLER] }),
            json!({ "hand": [FILLER] }),
        );
        s.play(RAISE_THEIRS, json!({}));
        assert_eq!(s.state().players[P2].mana_cap_bonus, Some(CAP_RAISE));
        assert_eq!(s.state().players[P1].mana_cap_bonus, None);
        s.end_turn().end_turn().end_turn();
        assert_eq!(s.state().players[P2].mana.max, MAX_MANA + CAP_RAISE);
        assert_eq!(
            s.state().players[P1].mana.max,
            MAX_MANA,
            "the caster's cap is untouched"
        );
    }
}

mod r5237004_stays_sick {
    use super::*;

    /// A 2/5 Unit that stays summoning-sick while it is undamaged.
    const SULKER: &str = "fvhl-sulker";

    fn sulker() -> (CardDef, Script) {
        let script = Script {
            stays_sick: Some(read_hook(|a| a.self_.damage == 0)),
            ..Script::default()
        };
        (def(SULKER, 523_030, "Unit", Some((2, 5)), Vec::new()), script)
    }

    #[test]
    fn r5237004_a_unit_whose_hook_holds_is_sick_and_cannot_attack() {
        install(vec![sulker()]);
        let mut s = board(
            "r5237004-sick",
            json!({ "field": [SULKER], "hand": [FILLER] }),
            json!({ "field": [WALL] }),
        );
        let sulker = lane_id(&s, P1, 1);
        let wall = lane_id(&s, P2, 1);
        assert!(is_sick(s.state(), s.card(sulker.as_str())));
        assert!(s.attacks_of(sulker.as_str()).is_empty(), "no attack is offered");
        s.expect_refused(|s| s.attack(sulker.as_str(), "hero"));
        s.expect_refused(|s| s.attack(sulker.as_str(), wall.as_str()));
        // A turn later it is still sick: the hook, not the turn it arrived on, holds it.
        s.end_turn().end_turn();
        assert!(s.attacks_of(sulker.as_str()).is_empty());
    }

    #[test]
    fn r5237004_once_its_hook_no_longer_holds_it_attacks() {
        install(vec![sulker()]);
        let mut s = board(
            "r5237004-well",
            json!({ "field": [{ "def": SULKER, "damage": 1 }], "hand": [FILLER] }),
            json!({ "field": [WALL] }),
        );
        let sulker = lane_id(&s, P1, 1);
        assert!(!is_sick(s.state(), s.card(sulker.as_str())));
        assert!(!s.attacks_of(sulker.as_str()).is_empty());
        let health = hero_health(&s, P2);
        s.attack(sulker.as_str(), "hero");
        assert_eq!(hero_health(&s, P2), health - 2);
    }
}

mod r5237005_hand_place {
    use super::*;

    fn played(s: &Scenario, id: &str) -> GameEvent {
        must(
            s.last_events()
                .iter()
                .find(|event| matches!(event, GameEvent::CardPlayed { instance_id, .. } if instance_id == id))
                .cloned(),
            "the cardPlayed event",
        )
    }

    /// The place the play's event records: (index, hand size before the play).
    fn place_of(event: &GameEvent) -> (i32, i32) {
        let GameEvent::CardPlayed { from_hand, .. } = event else {
            panic!("not a cardPlayed event: {event:?}");
        };
        let place = must(from_hand.as_ref(), "the place it left the hand from");
        (place.index, place.of)
    }

    fn three_in_hand(seed: &str) -> Scenario {
        install(vec![]);
        board(
            seed,
            json!({ "hand": [FILLER, FILLER, FILLER] }),
            json!({ "hand": [FILLER], "library": kept_library() }),
        )
    }

    #[test]
    fn r5237005_the_leftmost_and_rightmost_plays_record_their_place() {
        let mut s = three_in_hand("r5237005-ends");
        let hand = s.hand(P1);
        s.play(hand[0].clone(), json!({}));
        let left = played(&s, &hand[0].id);
        assert_eq!(place_of(&left), (0, 3));
        assert!(played_leftmost(&left));
        assert!(!played_rightmost(&left));

        let mut s = three_in_hand("r5237005-right");
        let hand = s.hand(P1);
        s.play(hand[2].clone(), json!({}));
        let right = played(&s, &hand[2].id);
        assert_eq!(place_of(&right), (2, 3));
        assert!(played_rightmost(&right));
        assert!(!played_leftmost(&right));
    }

    #[test]
    fn r5237005_a_one_card_hand_is_both() {
        install(vec![]);
        let mut s = board(
            "r5237005-one",
            json!({ "hand": [FILLER] }),
            json!({ "hand": [FILLER], "library": kept_library() }),
        );
        let only = s.hand(P1)[0].clone();
        s.play(only.clone(), json!({}));
        let event = played(&s, &only.id);
        assert_eq!(place_of(&event), (0, 1));
        assert!(played_leftmost(&event));
        assert!(played_rightmost(&event));
    }

    #[test]
    fn r5237005_a_middle_card_is_neither() {
        let mut s = three_in_hand("r5237005-middle");
        let hand = s.hand(P1);
        s.play(hand[1].clone(), json!({}));
        let middle = played(&s, &hand[1].id);
        assert_eq!(place_of(&middle), (1, 3));
        assert!(!played_leftmost(&middle));
        assert!(!played_rightmost(&middle));
    }

    #[test]
    fn r5237005_an_event_that_is_no_play_is_neither() {
        let mut s = three_in_hand("r5237005-other");
        s.end_turn();
        let ended = must(
            s.last_events()
                .iter()
                .find(|event| matches!(event, GameEvent::TurnEnded { .. }))
                .cloned(),
            "the turnEnded event",
        );
        assert!(!played_leftmost(&ended));
        assert!(!played_rightmost(&ended));
    }
}

mod r5237006_end_of_turn_grant {
    use super::*;

    const AT_END: &str = "fvhl-grant-end";
    const AT_DEATH: &str = "fvhl-grant-death";

    /// "Give a Unit: End of turn: deal 2 damage to the enemy hero", granted `on: EndOfTurn` when
    /// `at_end`, else with no `on`.
    fn granter(id: &'static str, index: u32, at_end: bool) -> (CardDef, Script) {
        let script = Script {
            targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
            cry: Some(hook(move |_ctx| {
                let mut args: GrantAbilityArgs = json_as(json!({
                    "target": { "of": "chosen" },
                    "grant": format!("{id}#{GRANT_KEY}"),
                }));
                args.on = at_end.then_some(GrantOn::EndOfTurn);
                vec![effects::grant_ability(args)]
            })),
            grants: IndexMap::from([(
                GRANT_KEY,
                hook(|_ctx| {
                    vec![effects::damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": GRANT_DAMAGE }),
                    ))]
                }),
            )]),
            ..Script::default()
        };
        (spell(id, index), script)
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![granter(AT_END, 523_040, true), granter(AT_DEATH, 523_041, false)]
    }

    #[test]
    fn r5237006_runs_at_each_end_of_its_holders_controllers_turn() {
        install(cards());
        let mut s = board(
            "r5237006-end",
            json!({ "hand": [AT_END, FILLER], "field": [WALL] }),
            json!({ "hand": [FILLER] }),
        );
        let wall = lane_id(&s, P1, 1);
        s.play(AT_END, pick::targets(pick::id(&wall)));
        assert!(jackioh_engine::grants::end_of_turn_hook_of(s.state(), s.card(wall.as_str())).is_some());
        let health = hero_health(&s, P2);
        s.end_turn();
        assert_eq!(
            hero_health(&s, P2),
            health - GRANT_DAMAGE,
            "its controller's end of turn"
        );
        s.end_turn().end_turn();
        assert_eq!(hero_health(&s, P2), health - 2 * GRANT_DAMAGE, "and the next one");
    }

    #[test]
    fn r5237006_never_at_the_other_sides_end_of_turn() {
        install(cards());
        let mut s = board(
            "r5237006-theirs",
            json!({ "hand": [AT_END, FILLER], "field": [WALL] }),
            json!({ "hand": [FILLER] }),
        );
        let wall = lane_id(&s, P1, 1);
        s.play(AT_END, pick::targets(pick::id(&wall)));
        s.end_turn();
        let p1_health = hero_health(&s, P1);
        let p2_health = hero_health(&s, P2);
        s.end_turn();
        assert_eq!(
            hero_health(&s, P2),
            p2_health,
            "p2's end of turn runs nothing on p1's Unit"
        );
        assert_eq!(hero_health(&s, P1), p1_health);
    }

    #[test]
    fn r5237006_absent_is_a_death_grant_that_never_runs_at_the_end_of_turn() {
        install(cards());
        let mut s = board(
            "r5237006-death",
            json!({ "hand": [AT_DEATH, FILLER], "field": [WALL] }),
            json!({ "hand": [FILLER] }),
        );
        let wall = lane_id(&s, P1, 1);
        s.play(AT_DEATH, pick::targets(pick::id(&wall)));
        assert!(
            s.card(wall.as_str())
                .grants
                .as_ref()
                .is_some_and(|grants| !grants.is_empty()),
            "the grant is held"
        );
        assert!(jackioh_engine::grants::end_of_turn_hook_of(s.state(), s.card(wall.as_str())).is_none());
        let health = hero_health(&s, P2);
        s.end_turn();
        assert_eq!(hero_health(&s, P2), health);
    }
}

mod r5237007_give_units_stats {
    use super::*;

    const HUMAN: &str = "fvhl-human";
    const HUMANS_GROW: &str = "fvhl-humans-grow";

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (
                def(HUMAN, 523_050, "Unit", Some((1, 1)), vec![Tag::Human]),
                Script::default(),
            ),
            (
                spell(HUMANS_GROW, 523_051),
                cast(|| {
                    effects::player_mods::give_units_stats(GiveUnitsStatsArgs {
                        player: None,
                        attack: TAG_STATS,
                        health: TAG_STATS,
                        tags: Some(vec![Tag::Human]),
                        expiry: ModifierExpiry::Never,
                    })
                }),
            ),
        ]
    }

    #[test]
    fn r5237007_the_tagged_units_and_later_arrivals_take_the_stats() {
        install(cards());
        let mut s = board(
            "r5237007-tagged",
            json!({ "hand": [HUMANS_GROW, HUMAN, FILLER], "field": [HUMAN] }),
            json!({ "hand": [FILLER] }),
        );
        let standing = lane_id(&s, P1, 1);
        s.play(HUMANS_GROW, json!({}));
        let grown = json!({ "attack": 1 + TAG_STATS, "health": 1 + TAG_STATS });
        s.expect_stats(standing.as_str(), grown.clone());
        s.play(HUMAN, json!({}));
        let later = lane_id(&s, P1, 2);
        s.expect_stats(later.as_str(), grown.clone());
        s.end_turn().end_turn();
        s.expect_stats(standing.as_str(), grown);
    }

    #[test]
    fn r5237007_untagged_and_enemy_units_take_nothing() {
        install(cards());
        let mut s = board(
            "r5237007-untagged",
            json!({ "hand": [HUMANS_GROW], "field": [PAWN] }),
            json!({ "field": [HUMAN] }),
        );
        let pawn = lane_id(&s, P1, 1);
        let theirs = lane_id(&s, P2, 1);
        s.play(HUMANS_GROW, json!({}));
        s.expect_stats(pawn.as_str(), json!({ "attack": 1, "health": 1 }));
        s.expect_stats(theirs.as_str(), json!({ "attack": 1, "health": 1 }));
    }
}

mod r5237008_give_cards_keyword {
    use super::*;

    const LEND: &str = "fvhl-lend-taunt";

    fn cards() -> Vec<(CardDef, Script)> {
        vec![(
            spell(LEND, 523_060),
            cast(|| {
                effects::player_mods::give_cards_keyword(json_as(json!({ "keyword": { "kind": "Taunt" } })))
            }),
        )]
    }

    #[test]
    fn r5237008_every_card_its_player_controls_has_the_keyword_later_ones_too() {
        install(cards());
        let mut s = board(
            "r5237008-lent",
            json!({ "hand": [LEND, PAWN, FILLER], "field": [PAWN] }),
            json!({ "hand": [FILLER] }),
        );
        let standing = lane_id(&s, P1, 1);
        assert!(!has_taunt(&s, &standing));
        s.play(LEND, json!({}));
        assert!(has_taunt(&s, &standing));
        s.play(PAWN, json!({}));
        let later = lane_id(&s, P1, 2);
        assert!(has_taunt(&s, &later), "a card arriving later has it too");
    }

    #[test]
    fn r5237008_the_opponents_cards_take_nothing() {
        install(cards());
        let mut s = board(
            "r5237008-theirs",
            json!({ "hand": [LEND] }),
            json!({ "field": [PAWN] }),
        );
        let theirs = lane_id(&s, P2, 1);
        s.play(LEND, json!({}));
        assert!(!has_taunt(&s, &theirs));
    }
}

mod r5237009_harm_ignores_tags {
    use super::*;

    /// A Felinor Unit that harmful effects naming Felinor pass by.
    const WARDED: &str = "fvhl-warded-felinor";
    const FELINOR: &str = "fvhl-felinor";
    /// A Felinor and Human Unit that harm naming Felinor passes by, and nothing else.
    const BOTH_TAGS: &str = "fvhl-felinor-human";
    const PURGE_FELINORS: &str = "fvhl-purge-felinors";
    const PURGE_HUMANS: &str = "fvhl-purge-humans";

    fn ignores_felinor() -> Script {
        Script {
            static_flags: Some(StaticFlags {
                harm_ignores_tags: Some(vec![Tag::Felinor]),
                ..StaticFlags::default()
            }),
            ..Script::default()
        }
    }

    fn purge(tag: Tag) -> Script {
        cast(move || {
            effects::destroy_all(BoardScope {
                tags: Some(vec![tag]),
                ..BoardScope::default()
            })
        })
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (
                def(WARDED, 523_070, "Unit", Some((1, 3)), vec![Tag::Felinor]),
                ignores_felinor(),
            ),
            (
                def(FELINOR, 523_071, "Unit", Some((1, 3)), vec![Tag::Felinor]),
                Script::default(),
            ),
            (
                def(
                    BOTH_TAGS,
                    523_072,
                    "Unit",
                    Some((1, 3)),
                    vec![Tag::Felinor, Tag::Human],
                ),
                ignores_felinor(),
            ),
            (spell(PURGE_FELINORS, 523_073), purge(Tag::Felinor)),
            (spell(PURGE_HUMANS, 523_074), purge(Tag::Human)),
        ]
    }

    #[test]
    fn r5237009_a_harmful_filter_naming_the_tag_passes_the_card_by() {
        install(cards());
        let mut s = board(
            "r5237009-passed",
            json!({ "hand": [PURGE_FELINORS] }),
            json!({ "field": [WARDED, FELINOR] }),
        );
        let warded = lane_id(&s, P2, 1);
        let plain = lane_id(&s, P2, 2);
        assert!(tribal_hate_cannot_reach(
            s.state(),
            s.card(warded.as_str()),
            Some(&[Tag::Felinor][..]),
            None
        ));
        s.play(PURGE_FELINORS, json!({}));
        s.expect_in_zone(warded.as_str(), "field");
        s.expect_in_zone(plain.as_str(), "graveyard");
    }

    #[test]
    fn r5237009_a_card_without_the_flag_is_reached() {
        install(cards());
        let s = board("r5237009-plain", json!({}), json!({ "field": [FELINOR] }));
        let plain = lane_id(&s, P2, 1);
        assert!(!tribal_hate_cannot_reach(
            s.state(),
            s.card(plain.as_str()),
            Some(&[Tag::Felinor][..]),
            None
        ));
    }

    #[test]
    fn r5237009_a_filter_naming_another_tag_still_reaches_it() {
        install(cards());
        let mut s = board(
            "r5237009-other",
            json!({ "hand": [PURGE_HUMANS] }),
            json!({ "field": [BOTH_TAGS] }),
        );
        let both = lane_id(&s, P2, 1);
        assert!(!tribal_hate_cannot_reach(
            s.state(),
            s.card(both.as_str()),
            Some(&[Tag::Human][..]),
            None
        ));
        s.play(PURGE_HUMANS, json!({}));
        s.expect_in_zone(both.as_str(), "graveyard");
    }
}

mod r5237010_immune_to_tag {
    use super::*;

    /// A 1/6 Unit that no Human card can affect or damage.
    const WARDED: &str = "fvhl-immune-to-humans";
    /// A Human Spell: deal 2 damage to every enemy Unit.
    const HUMAN_BLAST: &str = "fvhl-human-blast";
    /// The same with no tag.
    const PLAIN_BLAST: &str = "fvhl-plain-blast";

    fn blast() -> Script {
        cast(|| effects::damage_all(json_as(json!({ "amount": BLAST_DAMAGE, "side": "enemy" }))))
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (
                def(WARDED, 523_080, "Unit", Some((1, 6)), Vec::new()),
                Script {
                    static_flags: Some(StaticFlags {
                        immune_to_tag: Some(Tag::Human),
                        ..StaticFlags::default()
                    }),
                    ..Script::default()
                },
            ),
            (
                def(HUMAN_BLAST, 523_081, "Spell", None, vec![Tag::Human]),
                blast(),
            ),
            (spell(PLAIN_BLAST, 523_082), blast()),
        ]
    }

    #[test]
    fn r5237010_a_card_whose_source_carries_the_tag_cannot_damage_it() {
        install(cards());
        let mut s = board(
            "r5237010-human",
            json!({ "hand": [HUMAN_BLAST] }),
            json!({ "field": [WARDED, WALL] }),
        );
        let warded = lane_id(&s, P2, 1);
        let wall = lane_id(&s, P2, 2);
        s.play(HUMAN_BLAST, json!({}));
        s.expect_stats(warded.as_str(), json!({ "health": 6 }));
        s.expect_stats(wall.as_str(), json!({ "health": 5 - BLAST_DAMAGE }));
    }

    #[test]
    fn r5237010_a_source_without_the_tag_still_damages_it() {
        install(cards());
        let mut s = board(
            "r5237010-plain",
            json!({ "hand": [PLAIN_BLAST] }),
            json!({ "field": [WARDED] }),
        );
        let warded = lane_id(&s, P2, 1);
        s.play(PLAIN_BLAST, json!({}));
        s.expect_stats(warded.as_str(), json!({ "health": 6 - BLAST_DAMAGE }));
    }

    #[test]
    fn r5237010_tag_cannot_reach_reads_the_sources_tags() {
        install(cards());
        let s = board(
            "r5237010-read",
            json!({ "hand": [HUMAN_BLAST, PLAIN_BLAST] }),
            json!({ "field": [WARDED, WALL] }),
        );
        let warded = s.unit_at(P2, 1);
        let wall = s.unit_at(P2, 2);
        let human = s.card(HUMAN_BLAST).clone();
        let plain = s.card(PLAIN_BLAST).clone();
        assert!(tag_cannot_reach(s.state(), Some(&human), &warded));
        assert!(
            !tag_cannot_reach(s.state(), Some(&plain), &warded),
            "a source without the tag"
        );
        assert!(
            !tag_cannot_reach(s.state(), None, &warded),
            "no source carries no tag"
        );
        assert!(
            !tag_cannot_reach(s.state(), Some(&human), &wall),
            "a card without the flag"
        );
    }
}

mod r5237011_log2_utility_all {
    use super::*;

    const BITS: &str = "fvhl-bits";

    pub fn bits(id: &'static str, index: u32, mode: Log2Utility) -> (CardDef, Script) {
        let script = Script {
            static_flags: Some(StaticFlags {
                log2_utility: Some(mode),
                ..StaticFlags::default()
            }),
            ..Script::default()
        };
        (def(id, index, "Field Spell", None, Vec::new()), script)
    }

    pub fn cost_now(s: &Scenario, card: &CardInstance) -> i32 {
        effective_cost(s.state(), card, CostOptions::default())
    }

    /// The first card of `player`'s hand of `def_id`.
    pub fn held(s: &Scenario, player: PlayerId, def_id: &str) -> CardInstance {
        must(
            s.hand(player).into_iter().find(|card| card.def_id == def_id),
            "the card in hand",
        )
    }

    #[test]
    fn r5237011_log2_utility_is_the_bit_length_and_leaves_n_at_or_below_0() {
        for (n, bits) in [(1, 1), (2, 2), (3, 2), (4, 3), (5, 3), (7, 3), (8, 4), (16, 5)] {
            assert_eq!(log2_utility(n), bits, "{n}");
        }
        for n in [0, -1, -5] {
            assert_eq!(log2_utility(n), n, "{n} is unchanged");
        }
    }

    #[test]
    fn r5237011_attack_health_and_own_cost_count_as_their_bit_length_on_both_sides() {
        install(vec![bits(BITS, 523_090, Log2Utility::All)]);
        let mut s = board(
            "r5237011-all",
            json!({ "hand": [BITS, BIG], "field": [BIG_ON_FIELD] }),
            json!({ "hand": [BIG], "field": [BIG_ON_FIELD] }),
        );
        let mine = lane_id(&s, P1, 1);
        let theirs = lane_id(&s, P2, 1);
        s.play(BITS, json!({}));
        let bit_stats = json!({ "attack": 3, "maxHealth": 4, "health": 4 });
        s.expect_stats(mine.as_str(), bit_stats.clone());
        s.expect_stats(theirs.as_str(), bit_stats);
        assert_eq!(cost_now(&s, &held(&s, P1, BIG)), 3, "your (4) costs (3)");
        assert_eq!(cost_now(&s, &held(&s, P2, BIG)), 3, "and so does theirs");
    }

    #[test]
    fn r5237011_without_it_nothing_changes() {
        install(vec![bits(BITS, 523_090, Log2Utility::All)]);
        let mut s = board(
            "r5237011-none",
            json!({ "hand": [BIG], "field": [BIG_ON_FIELD] }),
            json!({}),
        );
        let mine = lane_id(&s, P1, 1);
        s.expect_stats(mine.as_str(), json!({ "attack": 5, "maxHealth": 8, "health": 8 }));
        assert_eq!(cost_now(&s, &held(&s, P1, BIG)), 4);
    }

    #[test]
    fn r5237011_a_damaged_unit_whose_health_falls_to_0_dies() {
        install(vec![bits(BITS, 523_090, Log2Utility::All)]);
        let mut s = board(
            "r5237011-dies",
            json!({ "hand": [BITS] }),
            json!({ "field": [{ "def": BIG_ON_FIELD, "damage": 4 }, BIG_ON_FIELD] }),
        );
        let damaged = lane_id(&s, P2, 1);
        let whole = lane_id(&s, P2, 2);
        s.play(BITS, json!({}));
        s.expect_in_zone(damaged.as_str(), "graveyard");
        s.expect_in_zone(whole.as_str(), "field");
    }

    #[test]
    fn r5237011_two_do_not_stack() {
        install(vec![bits(BITS, 523_090, Log2Utility::All)]);
        let mut s = board(
            "r5237011-two",
            json!({ "hand": [BITS, BITS], "field": [BIG_ON_FIELD] }),
            json!({}),
        );
        let mine = lane_id(&s, P1, 1);
        s.play(BITS, json!({}));
        s.play(BITS, json!({}));
        s.expect_stats(mine.as_str(), json!({ "attack": 3, "maxHealth": 4 }));
    }
}

mod r5237012_log2_utility_helpful {
    use super::r5237011_log2_utility_all::{bits, cost_now, held};
    use super::*;

    const HELPFUL: &str = "fvhl-bits-helpful";

    #[test]
    fn r5237012_only_your_costs_and_the_enemys_attack_and_health_are_transformed() {
        install(vec![bits(HELPFUL, 523_095, Log2Utility::Helpful)]);
        let mut s = board(
            "r5237012-helpful",
            json!({ "hand": [HELPFUL, BIG], "field": [BIG_ON_FIELD] }),
            json!({ "hand": [BIG], "field": [BIG_ON_FIELD] }),
        );
        let theirs = lane_id(&s, P2, 1);
        s.play(HELPFUL, json!({}));
        s.expect_stats(theirs.as_str(), json!({ "attack": 3, "maxHealth": 4 }));
        assert_eq!(cost_now(&s, &held(&s, P1, BIG)), 3);
    }

    #[test]
    fn r5237012_never_your_units_or_the_enemys_costs() {
        install(vec![bits(HELPFUL, 523_095, Log2Utility::Helpful)]);
        let mut s = board(
            "r5237012-not",
            json!({ "hand": [HELPFUL], "field": [BIG_ON_FIELD] }),
            json!({ "hand": [BIG] }),
        );
        let mine = lane_id(&s, P1, 1);
        s.play(HELPFUL, json!({}));
        s.expect_stats(mine.as_str(), json!({ "attack": 5, "maxHealth": 8 }));
        assert_eq!(cost_now(&s, &held(&s, P2, BIG)), 4);
    }
}

mod r5237013_plagiarized_keeps_the_recursion_log {
    use super::*;

    const PLAGIARIZED: &str = "fvhl-plagiarized";
    const HUMAN_CARD: &str = "fvhl-human-card";

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (
                def(PLAGIARIZED, 523_100, "Spell", None, vec![Tag::Plagiarized]),
                Script::default(),
            ),
            (
                def(HUMAN_CARD, 523_101, "Spell", None, vec![Tag::Human]),
                Script::default(),
            ),
        ]
    }

    #[test]
    fn r5237013_kept_in_a_game_holding_a_plagiarized_card_and_no_infinite_assault_card() {
        assert!(RECURSION_LOG_TAGS.contains(&Tag::Plagiarized));
        install(cards());
        let mut s = board(
            "r5237013-kept",
            json!({ "hand": [FILLER], "library": [PLAGIARIZED, FILLER, FILLER, FILLER] }),
            json!({}),
        );
        assert!(jackioh_engine::recursion_log::kept(s.state()));
        s.play(FILLER, json!({}));
        assert!(s.state().players[P1].recursion.is_some(), "the play is logged");
    }

    #[test]
    fn r5237013_not_kept_in_a_game_with_neither() {
        install(cards());
        let mut s = board(
            "r5237013-not",
            json!({ "hand": [FILLER], "library": [HUMAN_CARD, FILLER, FILLER, FILLER] }),
            json!({}),
        );
        assert!(!jackioh_engine::recursion_log::kept(s.state()));
        s.play(FILLER, json!({}));
        assert!(s.state().players[P1].recursion.is_none());
    }
}
