//! SPEC §8.12 S+ #121 Toxopid, a card of the Stinkyscape+ set (R1420, R3535000). Built by S13 (#3560).
//! The first time this game that this enters your hand, add a **Crawler** to your hand.
//! **Float** reduces mana. Friendly **Crawlers Dying** reduces mana cost by (1)
//! **Cry:** Summon 3 **Toxopid Legs** ANYWHERE on the field
//! **Crater**
//! **Immutable**
//! **Indestructible** while **Toxopid Legs** are on the field
//! 10/10
//! **Rush**
//! **Ignores Taunt**
//! **End of turn: Toxic 2** all enemies
//! \~\~\~
//! The first time this game that this enters your hand, add a **Crawler** to your hand.
//! **Float** reduces mana. Friendly **Crawlers Dying** reduces mana cost by (1)
//! **Cry:** Summon 4 **Radiant Toxopid Legs** on your opponents field
//! **Crater**
//! **Immutable**
//! **Indestructible** while **Toxopid Legs** are on the field
//! 20/20
//! **Rush**
//! **Ignores Taunt**
//! **End of turn: Toxic 4** all enemies
//!
//! R3560000: the first hand arrival each game is one mark per player (the opening deal counts); both
//! faces share it. R3560004: each friendly Crawler death takes 1 off every Toxopid in hand and deck
//! as a `costMod`, never below 0. R3560005: one Mode prompt per Leg; base both sides' unit zones,
//! Radiant the opponent's; never its own zone, a chosen zone, or one the summon can't land on; a Leg
//! belongs to the side it lands on, exiles the pile below, then Craters (R3554009). R3560006:
//! Indestructible while any Leg stands. R3560007: Ignores Taunt. Float and Toxic are the engine's.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-121";

/// The Crawler its arrival adds.
const CRAWLER: &str = "stinkyscapeplus-120";
/// The Leg its Cry summons.
const LEG: &str = "stinkyscapeplus-121-1";
/// R3560000: the once-a-game hand-arrival mark.
const HAND_KEY: &str = "stinkyscapeplus-121:hand";
/// R3560004: what each friendly Crawler death takes off.
const DISCOUNT: i32 = 1;
/// The Cry's resume step.
const LEG_STEP: &str = "toxopidLeg";
/// The Cry's data keys.
const LEFT_KEY: &str = "legsLeft";
const TAKEN_KEY: &str = "legsTaken";

/// R3560004: a friendly Crawler death cheapens this in hand and deck.
fn crawler_died(id: &str) -> TriggerDef {
    TriggerDef::new(id, &[GameEventType::Destroyed], move |ctx, event| {
        let GameEvent::Destroyed {
            def_id, controller, ..
        } = event
        else {
            return vec![];
        };
        if *def_id != CRAWLER || *controller != ctx.controller {
            return vec![];
        }
        vec![set_cost_mod(json_as(json!({
            "target": { "of": "self" },
            "amount": -DISCOUNT,
        })))]
    })
}

/// R3560005: the prompt for one more Leg, `left` to go, `taken` already chosen.
fn ask_leg(ctx: &EffectContext<'_>, left: i32, taken: Vec<String>) -> Vec<Effect> {
    if left <= 0 {
        return vec![];
    }
    let Some(me) = ctx.live_self().cloned().or_else(|| ctx.self_.clone()) else {
        return vec![];
    };
    let enemy = opponent_of(ctx.controller);
    let sides: Vec<(PlayerId, &str)> = if ctx.radiant {
        vec![(enemy, "enemy")]
    } else {
        vec![(ctx.controller, "own"), (enemy, "enemy")]
    };
    let mut options: Vec<String> = Vec::new();
    let mut labels: IndexMap<String, String> = IndexMap::new();
    for (side, prefix) in sides {
        for lane in 1..=UNIT_ZONES {
            let slot = ZoneSlot {
                player: side,
                row: Row::Units,
                lane,
            };
            let key = format!("{prefix}-{lane}");
            if taken.contains(&key) {
                continue;
            }
            if let Some(found) = occupant_at(ctx.state, slot).cloned()
                && found.id == me.id
            {
                continue;
            }
            if !exiling_summon_lands(ctx.state, &me, slot) {
                continue;
            }
            let label = if side == ctx.controller {
                format!("Your lane {lane}")
            } else {
                format!("Enemy lane {lane}")
            };
            labels.insert(key.clone(), label);
            options.push(key);
        }
    }
    if options.is_empty() {
        return vec![];
    }
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(LEFT_KEY.to_string(), json!(left));
    data.insert(TAKEN_KEY.to_string(), json!(taken));
    vec![choose_mode(ChooseModeArgs {
        options,
        step: LEG_STEP.into(),
        prompt: Some("Choose where a Toxopid Leg lands".into()),
        data: Some(data),
        by: None,
        labels: Some(labels),
    })]
}

fn toxopid() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            ignores_taunt: Some(true),
            ..StaticFlags::default()
        }),
        enters_hand: Some(hook(move |ctx| {
            if once_a_game_used(ctx.state, ctx.controller, HAND_KEY) {
                return vec![];
            }
            vec![
                mark_once_a_game(json_as(json!({ "key": HAND_KEY }))),
                add_to_hand(json_as(json!({ "defId": CRAWLER }))),
            ]
        })),
        hand_triggers: vec![
            float_reduces_cost("toxopid-float"),
            crawler_died("toxopid-crawler-hand"),
        ],
        deck_triggers: vec![crawler_died("toxopid-crawler-deck")],
        conditional_keywords: Some(read_hook(|args| {
            if def_on_field(args.state, LEG) {
                vec![Keyword::Indestructible]
            } else {
                vec![]
            }
        })),
        end_of_turn: Some(hook(|ctx| {
            vec![toxic_all(json_as(json!({
                "amount": param(ctx, "toxic"),
                "side": "enemy",
                "heroes": true,
            })))]
        })),
        cry: Some(hook(|ctx| ask_leg(ctx, param(ctx, "legs"), vec![]))),
        resume: IndexMap::from([(
            LEG_STEP,
            hook(|ctx| {
                let Some(choice) = chosen_options(ctx).first().cloned() else {
                    return vec![];
                };
                let (prefix, lane) = match choice.split_once('-') {
                    Some((prefix, lane)) => (prefix, lane.parse::<i32>().unwrap_or(0)),
                    None => return vec![],
                };
                let player = if prefix == "own" { "self" } else { "enemy" };
                let left = ctx.data.get(LEFT_KEY).and_then(Value::as_i64).unwrap_or(0) as i32;
                let mut taken: Vec<String> = ctx
                    .data
                    .get(TAKEN_KEY)
                    .and_then(|value| serde_json::from_value::<Vec<String>>(value.clone()).ok())
                    .unwrap_or_default();
                taken.push(choice);
                let mut out = vec![summon(json_as(json!({
                    "defId": LEG,
                    "lane": lane,
                    "player": player,
                    "exileBelow": true,
                    "radiant": ctx.radiant,
                })))];
                out.extend(ask_leg(ctx, left - 1, taken));
                out
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: toxopid(),
        radiant: toxopid(),
    }
}

// S+ #121 Toxopid — SPEC §8.12 row 121.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::query::{hero_toxic_of, toxic_on};
    use jackioh_engine::subsystems::ai_policy::policy_actions;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CRAWLER_DEF: &str = "stinkyscapeplus-120";
    const TOUGH: &str = "stand-in (1) 2/30";
    const FILLER: &str = "core-005";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn crawler_count(s: &Scenario) -> usize {
        s.hand(P1)
            .iter()
            .filter(|card| card.def_id == CRAWLER_DEF)
            .count()
    }

    #[test]
    fn r3560000_its_first_hand_arrival_adds_a_crawler_once() {
        for radiant in [false, true] {
            let mut s = weapon_board(
                "s121-hand",
                json!({ "hand": [{ "def": ID, "radiant": radiant }], "library": filler(4) }),
                json!({ "library": filler(4) }),
            );
            // A scenario's hand is set up, not dealt: it arrives nothing, so no Crawler yet.
            assert_eq!(crawler_count(&s), 0, "radiant {radiant}: a set-up hand is no arrival");
            let add = || {
                jackioh_engine::effects::add_to_hand(json_as(json!({ "defId": ID, "radiant": radiant })))
            };
            // The first arrival adds the Crawler; a second Toxopid adds none: once per game.
            crate::run_as(&mut s, add(), P1);
            assert_eq!(crawler_count(&s), 1, "radiant {radiant}: its first arrival adds one");
            crate::run_as(&mut s, add(), P1);
            assert_eq!(crawler_count(&s), 1, "radiant {radiant}: once per game");
        }
    }

    #[test]
    fn float_reduces_its_cost() {
        let mut s = weapon_board(
            "s121-float",
            json!({ "mana": 5, "hand": [ID], "library": filler(4) }),
            json!({ "library": filler(4) }),
        );
        let before = play_cost(s.state(), s.card(ID));
        s.end_turn();
        s.end_turn();
        let after = play_cost(s.state(), s.card(ID));
        assert!(after < before, "a Float took something off");
    }

    #[test]
    fn r3560004_a_friendly_crawler_dying_cheapens_it_in_hand_and_deck() {
        let mut s = weapon_board(
            "s121-discount",
            json!({
                "hand": [ID, FILLER],
                "field": [{ "def": CRAWLER_DEF, "lane": 1 }],
                "library": [{ "def": ID }, FILLER, FILLER, FILLER],
            }),
            json!({ "field": [VANILLA], "library": filler(4) }),
        );
        // The enemy's turn, on which its Unit attacks; the Float is read once it has begun.
        crate::next_turn_of(&mut s, P2);
        let in_deck = |s: &Scenario| {
            s.pile(P1, "library")
                .into_iter()
                .find(|card| card.def_id == ID)
                .expect("in deck")
        };
        let hand_before = play_cost(s.state(), s.card(ID));
        // The deck copy has not Floated (only the hand's trigger does), so it costs what it printed.
        let deck_before = play_cost(s.state(), &in_deck(&s));
        let killer = s.unit_at(P2, 1).id;
        let victim = s.unit_at(P1, 1).id;
        s.attack(killer.as_str(), victim.as_str());
        let hand_after = play_cost(s.state(), s.card(ID));
        assert_eq!(hand_after, (hand_before - 1).max(0));
        // The deck copy cheapened too.
        assert_eq!(play_cost(s.state(), &in_deck(&s)), (deck_before - 1).max(0));
    }

    #[test]
    fn r3560004_an_enemy_crawler_dying_changes_nothing() {
        let mut s = weapon_board(
            "s121-enemy",
            json!({ "hand": [ID], "field": [VANILLA], "library": filler(4) }),
            json!({ "field": [{ "def": CRAWLER_DEF, "lane": 1 }, VANILLA], "library": filler(4) }),
        );
        let before = play_cost(s.state(), s.card(ID));
        let killer = s.unit_at(P1, 1).id;
        let victim = s.unit_at(P2, 1).id;
        s.attack(killer.as_str(), victim.as_str());
        assert_eq!(play_cost(s.state(), s.card(ID)), before);
    }

    #[test]
    fn r3560005_cry_lands_three_legs_where_chosen_exiling_below() {
        let mut s = weapon_board(
            "s121-cry",
            json!({ "mana": 10, "hand": [ID], "library": filler(4) }),
            json!({ "field": [{ "def": VANILLA, "lane": 2 }], "library": filler(4) }),
        );
        s.play(ID, json!({ "zone": 1 }));
        // Three prompts, one per Leg.
        // Its own zone (lane 1) is no offer, so the Legs go to lanes 2, 3 and 4.
        for (n, lane) in [2, 3, 4].into_iter().enumerate() {
            assert!(s.state().pending.is_some(), "prompt {n}");
            s.answer(json!([format!("own-{lane}")]));
        }
        assert!(s.state().pending.is_none());
        // Three Legs stand where chosen.
        for lane in [2, 3, 4] {
            let leg = s.unit(P1, lane).expect("a Leg there");
            assert_eq!(leg.def_id, LEG);
        }
        // An enemy's exiled unit: the Leg on the enemy side exiled the pile below.
        let mut s = weapon_board(
            "s121-exile",
            json!({ "mana": 10, "hand": [{ "def": ID, "radiant": false }], "library": filler(4) }),
            json!({ "field": [{ "def": VANILLA, "lane": 4 }], "library": filler(4) }),
        );
        s.play(ID, json!({ "zone": 2 }));
        s.answer(json!(["enemy-4"]));
        s.answer(json!(["enemy-1"]));
        s.answer(json!(["enemy-2"]));
        let leg = s.unit(P2, 4).expect("the enemy's Leg");
        assert_eq!(leg.def_id, LEG);
        assert!(s.pile(P2, "exile").iter().any(|card| card.def_id == VANILLA));
        assert!(is_locked(
            s.state(),
            ZoneSlot {
                player: P2,
                row: Row::Units,
                lane: 4
            }
        ));
        // Its own zone and chosen zones are not offered.
        let mut s = weapon_board(
            "s121-offers",
            json!({ "mana": 10, "hand": [ID], "library": filler(4) }),
            json!({}),
        );
        s.play(ID, json!({ "zone": 1 }));
        let pending = s.state().pending.clone().expect("a Leg prompt");
        // A mode's key is `mode:<zone>`.
        let offered: Vec<String> = pending
            .options
            .iter()
            .map(|option| option.key.trim_start_matches("mode:").to_string())
            .collect();
        assert!(!offered.contains(&"own-1".to_string()), "its own zone");
        assert!(offered.contains(&"own-2".to_string()));
    }

    #[test]
    fn r3560005_radiant_lands_four_radiant_legs_on_the_opponents_field() {
        let mut s = weapon_board(
            "s121-radiant-cry",
            json!({ "mana": 10, "hand": [{ "def": ID, "radiant": true }], "library": filler(4) }),
            json!({}),
        );
        s.play(ID, json!({ "zone": 1 }));
        let pending = s.state().pending.clone().expect("a Leg prompt");
        assert!(
            pending
                .options
                .iter()
                .all(|option| option.key.trim_start_matches("mode:").starts_with("enemy-")),
            "the opponent's field only"
        );
        for lane in 1..=4 {
            s.answer(json!([format!("enemy-{lane}")]));
        }
        assert!(s.state().pending.is_none());
        for lane in 1..=4 {
            let leg = s.unit(P2, lane).expect("a Radiant Leg there");
            assert_eq!(leg.def_id, LEG);
            assert!(leg.radiant);
        }
    }

    #[test]
    fn r3560005_the_leg_prompt_is_answered_by_the_policy_and_the_timeout() {
        let mut s = weapon_board(
            "s121-policy",
            json!({ "mana": 10, "hand": [ID], "library": filler(4) }),
            json!({}),
        );
        s.play(ID, json!({ "zone": 1 }));
        assert!(!legal_actions(s.state(), P1).is_empty());
        assert!(
            policy_actions(s.state(), P1, Default::default())
                .iter()
                .any(|action| matches!(action, ActionBody::Answer { .. })),
            "the AI policy answers"
        );
        let timed_out = reduce(s.state(), &Action::new(ActionBody::Timeout, P1, "s121-t"));
        assert_eq!(timed_out.error, None);
    }

    #[test]
    fn r3560006_indestructible_while_a_leg_stands() {
        let mut s = weapon_board(
            "s121-indestructible",
            json!({ "mana": 10, "hand": [ID], "library": filler(4) }),
            json!({ "hand": [FILLER], "field": [TOUGH], "library": filler(4) }),
        );
        s.play(ID, json!({ "zone": 1 }));
        s.answer(json!(["own-2"]));
        s.answer(json!(["own-3"]));
        s.answer(json!(["own-4"]));
        assert!(unit_has(s.state(), s.card(ID), KeywordKind::Indestructible));
        // Without a Leg it is destructible: a fresh board has none.
        let s = weapon_board("s121-mortal", json!({ "field": [ID] }), json!({}));
        assert!(!unit_has(s.state(), s.card(ID), KeywordKind::Indestructible));
    }

    #[test]
    fn r3560007_it_ignores_taunt() {
        let s = weapon_board(
            "s121-taunt",
            json!({ "field": [ID] }),
            json!({ "field": [{ "def": "core-019", "lane": 1 }, VANILLA] }),
        );
        let toxopid = s.unit_at(P1, 1);
        let plain = s.unit_at(P2, 2);
        assert!(can_attack(
            s.state(),
            &toxopid,
            &DamageTarget::Hero { player: P2 }
        ));
        assert!(can_attack(
            s.state(),
            &toxopid,
            &DamageTarget::Unit { instance: plain }
        ));
    }

    #[test]
    fn end_of_turn_gives_every_enemy_toxic() {
        for radiant in [false, true] {
            let toxic = if radiant { 4 } else { 2 };
            let mut s = weapon_board(
                "s121-toxic",
                json!({ "field": [{ "def": ID, "radiant": radiant }] }),
                // A sturdy enemy: Toxic 4 ticks at its owner's turn start and would kill a 4/4.
                json!({ "field": ["stand-in (1) 10/10 R20/20"], "library": filler(4) }),
            );
            s.end_turn();
            let enemy = s.unit_at(P2, 1).id;
            assert_eq!(toxic_on(s.card(&enemy)), toxic, "radiant {radiant}");
            assert_eq!(hero_toxic_of(s.state(), P2), toxic, "radiant {radiant}");
        }
    }
}
