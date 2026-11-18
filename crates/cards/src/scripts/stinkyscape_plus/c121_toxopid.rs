//! SPEC §8.12 S+ #121 Toxopid, a card of the Stinkyscape+ set (R1420, R3535000). Built by S13 (#3560).
//! **Float** reduces mana.
//! **Cry:** Summon 3 **Toxopid Legs** in random empty unit zones on the field
//! **Crater**
//! **Immutable**
//! **Indestructible** while **Toxopid Legs** are on the field
//! 10/10
//! **Rush**
//! **Ignores Taunt**
//! **End of turn: Toxic 2** all enemies
//! \~\~\~
//! **Float** reduces mana.
//! **Cry:** Summon 4 **Radiant Toxopid Legs** in random empty unit zones on your opponent's field
//! **Crater**
//! **Immutable**
//! **Indestructible** while **Toxopid Legs** are on the field
//! 20/20
//! **Rush**
//! **Ignores Taunt**
//! **End of turn: Toxic 4** all enemies
//!
//! R4154300: the Cry draws its Legs' zones together as it resolves, from the empty unit zones of both
//! sides (the Radiant face: the opponent's), no zone twice, and summons a Leg into each; with fewer zones
//! fewer Legs land, and no prompt opens. A Leg belongs to the side it lands on and Craters. R3560006:
//! Indestructible while any Leg stands. R3560007: Ignores Taunt. Float and Toxic are the engine's.
//!
//! Issue #4154 placed the Legs at random and took out its first-hand-arrival Crawler and the discount
//! each friendly Crawler death gave it. Through v0.6.6g (`script_v0_6_6g`, R767000) its first hand arrival
//! each game added a Crawler (R3560000), each friendly Crawler death took 1 off it in hand and deck
//! (R3560004), and its Cry opened one Mode prompt per Leg, each Leg exiling the pile below (R3560005).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-121";

/// The Crawler its arrival added through v0.6.6g.
const CRAWLER: &str = "stinkyscapeplus-120";
/// The Leg its Cry summons.
const LEG: &str = "stinkyscapeplus-121-1";
/// R3560000: the once-a-game hand-arrival mark (through v0.6.6g).
const HAND_KEY: &str = "stinkyscapeplus-121:hand";
/// R3560004: what each friendly Crawler death took off (through v0.6.6g).
const DISCOUNT: i32 = 1;
/// The v0.6.6g Cry's resume step.
const LEG_STEP: &str = "toxopidLeg";
/// The v0.6.6g Cry's data keys.
const LEFT_KEY: &str = "legsLeft";
const TAKEN_KEY: &str = "legsTaken";
/// A unit zone's key, `own-<lane>` or `enemy-<lane>`: the v0.6.6g Cry's Mode options and the zones
/// R4154300's draw names.
const OWN: &str = "own";
const ENEMY: &str = "enemy";

/// R4154300: the empty unit zones a Leg may land in, each keyed by its side and lane: both sides' on
/// the base face, the opponent's on the Radiant face.
fn empty_zones(ctx: &EffectContext<'_>) -> Vec<String> {
    let Some(me) = ctx.live_self().cloned().or_else(|| ctx.self_.clone()) else {
        return vec![];
    };
    let enemy = opponent_of(ctx.controller);
    let sides: Vec<(PlayerId, &str)> = if ctx.radiant {
        vec![(enemy, ENEMY)]
    } else {
        vec![(ctx.controller, OWN), (enemy, ENEMY)]
    };
    let mut zones: Vec<String> = Vec::new();
    for (side, prefix) in sides {
        for lane in 1..=UNIT_ZONES {
            let slot = ZoneSlot {
                player: side,
                row: Row::Units,
                lane,
            };
            if jackioh_engine::zones::is_empty(ctx.state, slot) && exiling_summon_lands(ctx.state, &me, slot) {
                zones.push(format!("{prefix}-{lane}"));
            }
        }
    }
    zones
}

/// A Leg into the zone `zone` names (`own-<lane>` or `enemy-<lane>`), Radiant on the Radiant face.
fn leg_into(zone: &str, radiant: bool) -> Effect {
    let (side, lane) = zone.split_once('-').unwrap_or((ENEMY, "0"));
    let player = if side == OWN { "self" } else { "enemy" };
    summon(json_as(json!({
        "defId": LEG,
        "lane": lane.parse::<i32>().unwrap_or(0),
        "player": player,
        "radiant": radiant,
    })))
}

/// R4154300: `legs` different empty unit zones, drawn together as the Cry reaches them, a Leg into each.
fn summon_legs(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let legs = param(ctx, "legs").max(0) as usize;
    let radiant = ctx.radiant;
    vec![for_each_card(ForEachCardArgs {
        cards: Arc::new(move |c: &mut EffectContext<'_>| -> Vec<String> {
            let zones = empty_zones(c);
            c.sink.rng.shuffle(&zones).into_iter().take(legs).collect()
        }),
        each: Arc::new(move |zone: &str| leg_into(zone, radiant)),
    })]
}

/// R3560004 (through v0.6.6g): a friendly Crawler death cheapens this in hand and deck.
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

/// R3560005 (through v0.6.6g): the prompt for one more Leg, `left` to go, `taken` already chosen.
fn ask_leg(ctx: &EffectContext<'_>, left: i32, taken: Vec<String>) -> Vec<Effect> {
    if left <= 0 {
        return vec![];
    }
    let Some(me) = ctx.live_self().cloned().or_else(|| ctx.self_.clone()) else {
        return vec![];
    };
    let enemy = opponent_of(ctx.controller);
    let sides: Vec<(PlayerId, &str)> = if ctx.radiant {
        vec![(enemy, ENEMY)]
    } else {
        vec![(ctx.controller, OWN), (enemy, ENEMY)]
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

/// The face: today's when `through_v0_6_6g` is false, else the one v0.6.6g and before ran.
fn toxopid(through_v0_6_6g: bool) -> Script {
    let mut script = Script {
        static_flags: Some(StaticFlags {
            ignores_taunt: Some(true),
            ..StaticFlags::default()
        }),
        hand_triggers: vec![float_reduces_cost("toxopid-float")],
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
        cry: Some(hook(summon_legs)),
        ..Script::default()
    };
    if !through_v0_6_6g {
        return script;
    }
    script.enters_hand = Some(hook(move |ctx| {
        if once_a_game_used(ctx.state, ctx.controller, HAND_KEY) {
            return vec![];
        }
        vec![
            mark_once_a_game(json_as(json!({ "key": HAND_KEY }))),
            add_to_hand(json_as(json!({ "defId": CRAWLER }))),
        ]
    }));
    script.hand_triggers.push(crawler_died("toxopid-crawler-hand"));
    script.deck_triggers = vec![crawler_died("toxopid-crawler-deck")];
    script.cry = Some(hook(|ctx| ask_leg(ctx, param(ctx, "legs"), vec![])));
    script.resume = IndexMap::from([(
        LEG_STEP,
        hook(|ctx| {
            let Some(choice) = chosen_options(ctx).first().cloned() else {
                return vec![];
            };
            let (prefix, lane) = match choice.split_once('-') {
                Some((prefix, lane)) => (prefix, lane.parse::<i32>().unwrap_or(0)),
                None => return vec![],
            };
            let player = if prefix == OWN { "self" } else { "enemy" };
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
    )]);
    script
}

pub fn script() -> CardScripts {
    CardScripts {
        base: toxopid(false),
        radiant: toxopid(false),
    }
}

/// R767000: what it ran through v0.6.6g, before issue #4154: the first-hand-arrival Crawler, the
/// Crawler-death discount and the Leg prompts.
pub fn script_v0_6_6g() -> CardScripts {
    CardScripts {
        base: toxopid(true),
        radiant: toxopid(true),
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
    const BODY: &str = "stand-in (0) 1/1";
    /// How many seeds the random placement is watched over.
    const SEEDS: usize = 16;

    fn crawler_count(s: &Scenario) -> usize {
        s.hand(P1)
            .iter()
            .filter(|card| card.def_id == CRAWLER_DEF)
            .count()
    }

    /// The lanes of `player`'s side a Leg stands in.
    fn legs_of(s: &Scenario, player: PlayerId) -> Vec<i32> {
        (1..=UNIT_ZONES)
            .filter(|lane| s.unit(player, *lane).is_some_and(|unit| unit.def_id == LEG))
            .collect()
    }

    #[test]
    fn r4154300_the_cry_lands_three_legs_in_empty_zones_of_either_side_with_no_prompt() {
        let (mut own, mut theirs) = (0, 0);
        for n in 0..SEEDS {
            let mut s = weapon_board(
                &format!("s121-random-{n}"),
                json!({ "mana": 10, "hand": [ID], "library": filler(4) }),
                json!({ "field": [{ "def": VANILLA, "lane": 2 }], "library": filler(4) }),
            );
            let vanilla = s.unit_at(P2, 2).id;
            s.play(ID, json!({ "zone": 1 }));
            assert!(s.state().pending.is_none(), "seed {n}: no prompt opens");
            let (mine, enemy) = (legs_of(&s, P1), legs_of(&s, P2));
            assert_eq!(mine.len() + enemy.len(), 3, "seed {n}: three Legs");
            // Only empty zones: never the Toxopid's own, never the Vanilla's, which still stands.
            assert!(!mine.contains(&1) && !enemy.contains(&2), "seed {n}");
            assert_eq!(s.unit_at(P1, 1).def_id, ID);
            assert_eq!(s.unit_at(P2, 2).id, vanilla, "seed {n}: nothing exiled");
            assert!(s.pile(P2, "exile").is_empty(), "seed {n}");
            let legs: Vec<CardInstance> = mine
                .iter()
                .map(|lane| s.unit_at(P1, *lane))
                .chain(enemy.iter().map(|lane| s.unit_at(P2, *lane)))
                .collect();
            assert!(legs.iter().all(|leg| !leg.radiant), "seed {n}: base Legs");
            own += mine.len();
            theirs += enemy.len();
        }
        assert!(own > 0 && theirs > 0, "both sides take Legs over the seeds: {own} own, {theirs} enemy");
    }

    #[test]
    fn r4154300_radiant_lands_four_radiant_legs_in_the_opponents_empty_zones_only() {
        for n in 0..SEEDS {
            let mut s = weapon_board(
                &format!("s121-radiant-{n}"),
                json!({ "mana": 10, "hand": [{ "def": ID, "radiant": true }], "library": filler(4) }),
                json!({ "field": [{ "def": VANILLA, "lane": 1 }], "library": filler(4) }),
            );
            s.play(ID, json!({ "zone": 1 }));
            assert!(s.state().pending.is_none());
            assert!(legs_of(&s, P1).is_empty(), "seed {n}: none on its own side");
            // Four empty zones on the opponent's side, so all four take one.
            assert_eq!(legs_of(&s, P2), vec![2, 3, 4, 5], "seed {n}");
            for lane in 2..=5 {
                assert!(s.unit_at(P2, lane).radiant, "seed {n}: Radiant Legs");
            }
            assert_eq!(s.unit_at(P2, 1).def_id, VANILLA);
        }
    }

    #[test]
    fn r4154300_fewer_empty_zones_land_fewer_legs() {
        let mut s = weapon_board(
            "s121-crowded",
            json!({
                "mana": 10,
                "hand": [ID],
                "field": [
                    { "def": BODY, "lane": 2 },
                    { "def": BODY, "lane": 3 },
                    { "def": BODY, "lane": 4 },
                    { "def": BODY, "lane": 5 },
                ],
                "library": filler(4),
            }),
            json!({
                "field": [
                    { "def": BODY, "lane": 1 },
                    { "def": BODY, "lane": 2 },
                    { "def": BODY, "lane": 3 },
                    { "def": BODY, "lane": 4 },
                ],
                "library": filler(4),
            }),
        );
        s.play(ID, json!({ "zone": 1 }));
        assert!(s.state().pending.is_none());
        // One empty zone on the whole field: one Leg, the other two land nowhere.
        assert!(legs_of(&s, P1).is_empty());
        assert_eq!(legs_of(&s, P2), vec![5]);
        assert!(s.pile(P1, "exile").is_empty() && s.pile(P2, "exile").is_empty());
    }

    #[test]
    fn r4154300_its_hand_arrival_adds_no_crawler_and_a_crawler_death_leaves_its_cost() {
        let mut s = weapon_board(
            "s121-no-crawler",
            json!({ "hand": [FILLER], "field": [{ "def": CRAWLER_DEF, "lane": 1 }], "library": filler(4) }),
            json!({ "field": [VANILLA], "library": filler(4) }),
        );
        crate::run_as(
            &mut s,
            jackioh_engine::effects::add_to_hand(json_as(json!({ "defId": ID }))),
            P1,
        );
        assert_eq!(crawler_count(&s), 0, "no Crawler comes with it");
        let toxopid = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == ID)
            .expect("in hand");
        // A friendly Crawler dies on the enemy's turn: the Crawler's own Death may cheapen it (R4154301),
        // but the Toxopid itself takes nothing off for it.
        crate::next_turn_of(&mut s, P2);
        let before = s.card(&toxopid.id).cost_mod;
        let killer = s.unit_at(P2, 1).id;
        let victim = s.unit_at(P1, 1).id;
        s.attack(killer.as_str(), victim.as_str());
        s.expect_in_zone(victim.as_str(), "graveyard");
        let reduce = param_value(s.state(), Some(s.card(&victim)), "reduce", ParamValueOptions::default());
        assert_eq!(s.card(&toxopid.id).cost_mod, before - reduce, "the Crawler's reduction alone");
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
    fn r3560006_indestructible_while_a_leg_stands() {
        let mut s = weapon_board(
            "s121-indestructible",
            json!({ "mana": 10, "hand": [ID], "library": filler(4) }),
            json!({ "hand": [FILLER], "field": [TOUGH], "library": filler(4) }),
        );
        s.play(ID, json!({ "zone": 1 }));
        assert_eq!(legs_of(&s, P1).len() + legs_of(&s, P2).len(), 3);
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

    /// R767000: the script kept for the replays of v0.6.6g and before (issue #4154).
    mod kept_v0_6_6g {
        use super::*;

        #[test]
        fn r767000_r3560000_its_first_hand_arrival_added_a_crawler_once() {
            crate::under_kept(ID, script_v0_6_6g(), || {
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
                    crate::run_as(&mut s, add(), P1);
                    assert_eq!(crawler_count(&s), 1, "radiant {radiant}: its first arrival adds one");
                    crate::run_as(&mut s, add(), P1);
                    assert_eq!(crawler_count(&s), 1, "radiant {radiant}: once per game");
                }
            });
        }

        #[test]
        fn r767000_r3560004_a_friendly_crawler_dying_cheapened_it_in_the_deck() {
            crate::under_kept(ID, script_v0_6_6g(), || {
                let mut s = weapon_board(
                    "s121-discount",
                    json!({
                        "hand": [FILLER],
                        "field": [{ "def": CRAWLER_DEF, "lane": 1 }],
                        "library": [{ "def": ID }, FILLER, FILLER, FILLER],
                    }),
                    json!({ "field": [VANILLA], "library": filler(4) }),
                );
                crate::next_turn_of(&mut s, P2);
                let in_deck = |s: &Scenario| {
                    s.pile(P1, "library")
                        .into_iter()
                        .find(|card| card.def_id == ID)
                        .expect("in deck")
                };
                let before = play_cost(s.state(), &in_deck(&s));
                let killer = s.unit_at(P2, 1).id;
                let victim = s.unit_at(P1, 1).id;
                s.attack(killer.as_str(), victim.as_str());
                s.expect_in_zone(victim.as_str(), "graveyard");
                assert_eq!(play_cost(s.state(), &in_deck(&s)), (before - DISCOUNT).max(0));
            });
        }

        #[test]
        fn r767000_r3560005_its_cry_asked_where_each_leg_lands() {
            crate::under_kept(ID, script_v0_6_6g(), || {
                let mut s = weapon_board(
                    "s121-cry",
                    json!({ "mana": 10, "hand": [ID], "library": filler(4) }),
                    json!({ "field": [{ "def": VANILLA, "lane": 4 }], "library": filler(4) }),
                );
                s.play(ID, json!({ "zone": 1 }));
                let pending = s.state().pending.clone().expect("a Leg prompt");
                let offered: Vec<String> = pending
                    .options
                    .iter()
                    .map(|option| option.key.trim_start_matches("mode:").to_string())
                    .collect();
                assert!(!offered.contains(&"own-1".to_string()), "its own zone");
                assert!(
                    policy_actions(s.state(), P1, Default::default())
                        .iter()
                        .any(|action| matches!(action, ActionBody::Answer { .. })),
                    "the AI policy answers"
                );
                s.answer(json!(["enemy-4"]));
                s.answer(json!(["own-2"]));
                s.answer(json!(["own-3"]));
                assert!(s.state().pending.is_none());
                assert_eq!(legs_of(&s, P1), vec![2, 3]);
                assert_eq!(legs_of(&s, P2), vec![4]);
                // The enemy's Leg exiled the Unit below it.
                assert!(s.pile(P2, "exile").iter().any(|card| card.def_id == VANILLA));
            });
        }
    }
}
