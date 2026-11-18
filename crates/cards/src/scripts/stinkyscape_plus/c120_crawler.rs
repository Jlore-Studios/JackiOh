//! SPEC §8.12 S+ #120 Crawler, a card of the Stinkyscape+ set (R1420, R3535000). Built by S13 (#3560).
//! May be played from your GY for (1)
//! 3/1
//! **Death:** Add a **Toxopid** to your hand. If you already have one there, reduce its cost by (2) instead.
//! \~\~\~
//! May be played from your GY for (1)
//! 6/2
//! **Death:** Add a **Radiant Toxopid** to your hand. If you already have one there, reduce its cost by
//! (3) instead.
//!
//! Issue #3996 made it cost (1), from (0), and its Death deal 1 (Radiant 2), from 2 (Radiant 4). Issue #4154
//! made it a 3/1 (Radiant 6/2) whose Death adds or cheapens a Toxopid; through v0.6.6e (`script_v0_6_6e`,
//! R767000) it was a 1/3 (Radiant 2/6) with Can't Defend whose first arrival each game added a Toxopid
//! (R3560000), which died after it attacked (R3560002) and whose Death hit the last Unit it damaged
//! (R3560003).
//!
//! R3560001: the graveyard play at (1), any number of times. R4154301: its Death reads its controller's
//! hand as it resolves: with a Toxopid there (either face), the first in hand order, the one held longest,
//! takes a lasting `costMod` of minus `reduce`; with none, a Toxopid is added, Radiant on the Radiant face.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-120";

/// The Toxopid its Death adds or cheapens.
const TOXOPID: &str = "stinkyscapeplus-121";
/// R3560000: the once-a-game first-arrival mark (through v0.6.6e).
const BOARD_KEY: &str = "stinkyscapeplus-120:board";
/// R3560003: the last Unit this damaged (through v0.6.6e).
const LAST_DAMAGED: &str = "lastDamaged";
/// R3560001: "May be played from your graveyard for (1)".
const GRAVEYARD_PRICE: i32 = 1;

/// This card arrived on the field (played or summoned): its `Summoned` names its own id.
fn arrived(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = ctx.live_self() else {
        return false;
    };
    matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == me.id)
}

/// A `Damage` event this card dealt to a Unit: its source is this card, it dealt more than 0, and its
/// target is a card (heroes are no instances).
fn dealt_to_unit(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let me = ctx.self_.as_ref()?;
    let GameEvent::Damage {
        source_id,
        target_id,
        amount,
        ..
    } = event
    else {
        return None;
    };
    if source_id.as_deref() != Some(me.id.as_str()) || *amount <= 0 {
        return None;
    }
    find_instance(ctx.state, target_id).map(|_| target_id.clone())
}

/// R4154301: the Toxopid already in its controller's hand, the first in hand order (the hand keeps
/// arrival order, so the one held longest), either face.
fn held_toxopid(ctx: &EffectContext<'_>) -> Option<String> {
    zone_cards(ctx.state, ctx.controller, OffFieldZone::Hand)
        .into_iter()
        .find(|card| card.def_id == TOXOPID)
        .map(|card| card.id)
}

/// R4154301: "Add a (Radiant) Toxopid to your hand. If you already have one there, reduce its cost by
/// (N) instead."
fn toxopid_or_discount(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    match held_toxopid(ctx) {
        Some(held) => vec![set_cost_mod(json_as(json!({
            "target": { "of": "instance", "instanceId": held },
            "amount": -param(ctx, "reduce"),
        })))],
        None => vec![add_to_hand(json_as(json!({ "defId": TOXOPID, "radiant": radiant })))],
    }
}

/// The face: today's when `through_v0_6_6e` is false, else the one v0.6.6e and before ran.
fn crawler(radiant: bool, through_v0_6_6e: bool) -> Script {
    let static_flags = Some(StaticFlags {
        graveyard_price: Some(GRAVEYARD_PRICE),
        ..StaticFlags::default()
    });
    if !through_v0_6_6e {
        return Script {
            static_flags,
            death: Some(hook(move |ctx| toxopid_or_discount(ctx, radiant))),
            ..Script::default()
        };
    }
    Script {
        static_flags,
        triggers: vec![
            TriggerDef::new("crawler-board", &[GameEventType::Summoned], move |ctx, event| {
                if !arrived(ctx, event) {
                    return vec![];
                }
                if once_a_game_used(ctx.state, ctx.controller, BOARD_KEY) {
                    return vec![];
                }
                vec![
                    mark_once_a_game(json_as(json!({ "key": BOARD_KEY }))),
                    add_to_hand(json_as(json!({ "defId": TOXOPID, "radiant": radiant }))),
                ]
            })
            .with_when(|ctx, event| arrived(ctx, event)),
            TriggerDef::new(
                "crawler-damaged",
                &[GameEventType::Damage],
                |ctx, event| match dealt_to_unit(ctx, event) {
                    None => vec![],
                    Some(target_id) => vec![remember(json_as(json!({
                        "key": LAST_DAMAGED,
                        "value": target_id,
                    })))],
                },
            ),
        ],
        after_attack: Some(hook(|_ctx| {
            vec![destroy(json_as(json!({ "target": { "of": "self" } })))]
        })),
        death: Some(hook(|ctx| {
            // A kill in the same pass as its own death never reached the trigger: read it off this
            // action's events first (Frostspatula's same-pass fallback), then the memory.
            let late = ctx
                .events
                .iter()
                .rev()
                .filter_map(|event| dealt_to_unit(ctx, event))
                .next();
            let remembered = recalled(ctx, LAST_DAMAGED).and_then(|value| value.as_str().map(String::from));
            let Some(target_id) = late.or(remembered) else {
                return vec![];
            };
            vec![damage(json_as(json!({
                "to": { "of": "instance", "instanceId": target_id },
                "amount": param(ctx, "damage"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: crawler(false, false),
        radiant: crawler(true, false),
    }
}

/// R767000: what it ran through v0.6.6e, before issue #4154: the first-arrival Toxopid, the death after
/// an attack and the Death that hit the last Unit it damaged.
pub fn script_v0_6_6e() -> CardScripts {
    CardScripts {
        base: crawler(false, true),
        radiant: crawler(true, true),
    }
}

// S+ #120 Crawler — SPEC §8.12 row 120.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const BODY: &str = "stand-in (1) 1/10";
    const WALL: &str = "stand-in (1) 0/10";
    const BIG: &str = "stand-in (1) 9/9";

    fn toxopids(s: &Scenario) -> Vec<String> {
        s.hand(P1)
            .iter()
            .filter(|card| card.def_id == TOXOPID)
            .map(|card| card.id.clone())
            .collect()
    }

    /// p1's Crawler of `radiant`'s face in lane 1, p1's hand `hand`, a 9/9 on p2's side to kill it: p2's
    /// turn, so the 9/9 may attack.
    fn doomed(seed: &str, radiant: bool, hand: Value) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({ "hand": hand, "field": [{ "def": ID, "radiant": radiant, "lane": 1 }], "library": filler(4) }),
            json!({ "field": [{ "def": BIG, "lane": 1 }], "library": filler(4) }),
        );
        crate::next_turn_of(&mut s, P2);
        s
    }

    /// p2's 9/9 kills p1's Crawler.
    fn kill(s: &mut Scenario) {
        let big = s.unit_at(P2, 1).id;
        let crawler = s.unit_at(P1, 1).id;
        s.attack(big.as_str(), crawler.as_str());
        s.expect_in_zone(crawler.as_str(), "graveyard");
    }

    #[test]
    fn r3560001_it_is_played_from_the_graveyard_for_1() {
        // From the graveyard at (1): offered, and the play costs 1.
        let s = weapon_board(
            "s120-grave",
            json!({ "mana": 1, "graveyard": [ID], "library": filler(4) }),
            json!({}),
        );
        let id = s
            .pile(P1, "graveyard")
            .into_iter()
            .find(|card| card.def_id == ID)
            .expect("setup")
            .id;
        let offered: Vec<String> = legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, .. } => Some(instance_id),
                _ => None,
            })
            .collect();
        assert!(offered.contains(&id));
        let mana = s.state().players[P1].mana.current;
        let action = json!({ "type": "play", "playerId": "p1", "instanceId": id, "zone": { "row": "units", "lane": 2 }, "nonce": "s120-grave-1" });
        let result = reduce(s.state(), &json_as::<Action>(action));
        assert!(result.error.is_none(), "{:?}", result.error);
        assert_eq!(result.state.players[P1].mana.current, mana - 1);
    }

    #[test]
    fn r4154301_its_arrival_adds_nothing_and_it_lives_through_its_own_attack() {
        let mut s = weapon_board(
            "s120-arrival",
            json!({ "mana": 10, "hand": [ID, ID], "library": filler(4) }),
            json!({ "field": [{ "def": WALL, "lane": 1 }], "library": filler(4) }),
        );
        s.play(ID, json!({ "zone": 1 }));
        s.play(ID, json!({ "zone": 2 }));
        assert!(toxopids(&s).is_empty(), "no Toxopid comes with an arrival");
        let crawler = s.unit_at(P1, 1);
        assert!(!unit_has(s.state(), &crawler, KeywordKind::CantDefend), "it may defend");
        crate::next_turn_of(&mut s, P1);
        let wall = s.unit_at(P2, 1).id;
        s.attack(crawler.id.as_str(), wall.as_str());
        // The 0/10 strikes back for nothing: the Crawler stands, and nothing died to add a Toxopid.
        s.expect_in_zone(crawler.id.as_str(), "field");
        assert_eq!(s.stats(wall.as_str()).health, 10 - 3);
        assert!(toxopids(&s).is_empty());
    }

    #[test]
    fn r4154301_its_death_adds_a_toxopid_when_none_is_held() {
        let mut s = doomed("s120-add", false, json!([]));
        kill(&mut s);
        let held = toxopids(&s);
        assert_eq!(held.len(), 1);
        let toxopid = s.card(&held[0]);
        assert!(!toxopid.radiant);
        assert_eq!(toxopid.cost_mod, 0, "a fresh card at its printed price");
    }

    #[test]
    fn r4154301_its_death_cheapens_the_toxopid_held_longest_instead() {
        let mut s = doomed(
            "s120-reduce",
            false,
            json!([TOXOPID, "core-005", { "def": TOXOPID, "radiant": true }]),
        );
        let held = toxopids(&s);
        assert_eq!(held.len(), 2);
        let (first, second) = (s.card(&held[0]).cost_mod, s.card(&held[1]).cost_mod);
        kill(&mut s);
        // No Toxopid added: the first in hand order took the whole reduction, a lasting cost change.
        assert_eq!(toxopids(&s), held);
        assert_eq!(s.card(&held[0]).cost_mod, first - 2);
        assert_eq!(s.card(&held[1]).cost_mod, second);
        assert!(s.last_events_of(GameEventType::CostChanged).iter().any(|event| {
            matches!(event, GameEvent::CostChanged { instance_id, .. } if *instance_id == held[0])
        }));
    }

    #[test]
    fn r4154301_the_reduction_is_its_declared_number_and_lasts() {
        let mut s = doomed("s120-declared", false, json!([TOXOPID]));
        let reduce = param_value(s.state(), Some(s.card(ID)), "reduce", ParamValueOptions::default());
        assert_eq!(reduce, 2);
        let held = toxopids(&s);
        let before = s.card(&held[0]).cost_mod;
        kill(&mut s);
        assert_eq!(s.card(&held[0]).cost_mod, before - reduce);
        // It lasts: p2's turn ends and p1's begins, and the price stays where the Death put it.
        s.end_turn();
        assert_eq!(s.card(&held[0]).cost_mod, before - reduce);
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4154301_radiant_death_adds_a_radiant_toxopid() {
            let mut s = doomed("s120-radiant-add", true, json!([]));
            kill(&mut s);
            let held = toxopids(&s);
            assert_eq!(held.len(), 1);
            assert!(s.card(&held[0]).radiant);
        }

        #[test]
        fn r4154301_radiant_death_cheapens_a_held_toxopid_by_3_whichever_its_face() {
            let mut s = doomed("s120-radiant-reduce", true, json!([TOXOPID]));
            let held = toxopids(&s);
            let before = s.card(&held[0]).cost_mod;
            kill(&mut s);
            assert_eq!(toxopids(&s), held, "nothing added");
            assert_eq!(s.card(&held[0]).cost_mod, before - 3);
        }
    }

    /// R767000: the script kept for the replays of v0.6.6e and before (issue #4154), under its v0.6.6e
    /// definition: a 1/3 (Radiant 2/6) with Can't Defend whose Death dealt `damage`, 1 (Radiant 2).
    mod kept_v0_6_6e {
        use super::*;

        fn v0_6_6e(def: &mut CardDef) {
            def.base.attack = Some(1);
            def.base.health = Some(3);
            def.radiant.attack = Some(2);
            def.radiant.health = Some(6);
            let cant_defend: Vec<Keyword> =
                serde_json::from_value(json!([{ "kind": "Can't Defend" }])).expect("the v0.6.6e keywords");
            def.base.keywords = cant_defend.clone();
            def.radiant.keywords = cant_defend;
            def.params = serde_json::from_value(json!([
                { "key": "damage", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1 }
            ]))
            .expect("the v0.6.6e params");
        }

        #[test]
        fn r767000_r3560000_its_first_arrival_added_a_toxopid_once_per_game() {
            crate::under_kept_as(ID, script_v0_6_6e(), v0_6_6e, || {
                for radiant in [false, true] {
                    let mut s = weapon_board(
                        "s120-kept-arrival",
                        json!({
                            "mana": 10,
                            "hand": [{ "def": ID, "radiant": radiant }, { "def": ID, "radiant": radiant }],
                            "library": filler(4),
                        }),
                        json!({ "library": filler(4) }),
                    );
                    s.play(ID, json!({ "zone": 1 }));
                    assert_eq!(toxopids(&s).len(), 1, "radiant {radiant}");
                    assert_eq!(s.card(&toxopids(&s)[0]).radiant, radiant);
                    s.play(ID, json!({ "zone": 2 }));
                    assert_eq!(toxopids(&s).len(), 1, "radiant {radiant}: the mark is spent");
                }
            });
        }

        #[test]
        fn r767000_r3560002_r3560003_after_it_attacked_it_died_and_its_death_hit_the_unit_it_damaged() {
            crate::under_kept_as(ID, script_v0_6_6e(), v0_6_6e, || {
                let mut s = weapon_board(
                    "s120-kept-death",
                    json!({ "field": [{ "def": ID, "lane": 1 }] }),
                    json!({ "field": [{ "def": BODY, "lane": 1 }, { "def": BODY, "lane": 2 }] }),
                );
                crate::next_turn_of(&mut s, P1);
                let first = s.unit_at(P2, 1).id;
                let second = s.unit_at(P2, 2).id;
                s.attack(ID, first.as_str());
                s.expect_in_zone(ID, "graveyard");
                // Its 1, then its Death's 1 on the Unit it damaged; the other is untouched.
                assert_eq!(s.stats(first.as_str()).health, 10 - 1 - 1);
                assert_eq!(s.stats(second.as_str()).health, 10);
            });
        }
    }
}
