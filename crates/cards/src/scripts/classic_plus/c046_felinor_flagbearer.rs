//! C+ #46 Felinor Flagbearer (SPEC §8.7 row 46). (2) Unit, Felinor, Legendary, 4/4 → 8/8.
//!   Base:    "Rush. Aura: Your other Felinors have +{aura}/+{aura}. Death: Shuffle a Felinor Flagbearer
//!            Prime into your deck." — aura 1 (no Cry, no Cleave, balance patch 1)
//!   Radiant: the same with "Aura: Your Felinors have +{aura}/+{aura}." — aura 2
//!   Engine:  "The aura (§10.4 layer 5) reaches your Felinor-tagged Units, the Radiant's including
//!            itself. The Death shuffles a base C+ #46.1 in at a random position (§6.3), turned away at
//!            R80's cap. Tunes: aura 1 ↑."
//!
//! The aura is computed on read (§10.4 layer 5): `applies` reads instance data only — controller,
//! zone, def tags — never `unitView`, or the layers would recurse; a card dormant under a Stack pile
//! is no unit on the field for it (§3.2, R13). The shuffle goes through `shuffleIntoLibrary`, so its
//! owner is shown the card going in (R311) and a full library turns it away (R80).

use jackioh_engine::effects::shuffle_into;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-046";

/// TS `cardDef("classicplus-046-1").id`: the Prime the Death shuffles in.
const PRIME: &str = "classicplus-046-1";

/// "Your (other) Felinors have +N/+N": Felinor-tagged Units its controller has on the field.
fn felinor_aura(include_self: bool) -> AuraHook {
    aura_hook(move |ctx| {
        let amount = param(&ctx, "aura");
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == ctx.self_.controller
                    && matches!(unit.zone, Zone::Field { row: Row::Units, .. })
                    && (include_self || unit.id != ctx.self_.id)
                    && def_of(Some(ctx.state), &unit.def_id).tags.contains(&Tag::Felinor)
            }),
            mod_: StatMod {
                attack: Some(amount),
                max_health: Some(amount),
                ..StatMod::default()
            },
        }]
    })
}

fn flagbearer(include_self: bool) -> Script {
    Script {
        aura: Some(felinor_aura(include_self)),
        death: Some(hook(|_ctx| vec![shuffle_into(json_as(json!({ "defId": PRIME, "count": 1 })))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = flagbearer(false);

    let radiant = flagbearer(true);

    CardScripts { base, radiant }
}

// C+ #46 Felinor Flagbearer — SPEC §8.7 row 46, BUILD M9 Classic+ row C+ 46: "Rush (no Cleave, no Cry,
// balance patch 1); Aura: your other Felinor Units have +1/+1 while it is on the field (not itself, not
// the opponent's); Death shuffles a Felinor Flagbearer Prime (C+ #46.1) into your deck (R80's cap),
// shown in your library list; the aura reads through `param()`; radiant +2/+2 to every Felinor Unit
// you control, itself included".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FLAG: &str = "classicplus-046";
    const PRIME: &str = "classicplus-046-1";
    const DUPE: &str = "core-012"; // Duplicating Felinors 3/4, a Felinor
    const VANILLA: &str = "core-008"; // Mr. Vanilla 4/4, no tag
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy target Unit
    const TRUE_STRIKE: &str = "core-044"; // (1) Spell: Pierce, deal 4 damage
    const FILLER: &str = "core-005";

    fn kinds(s: &Scenario, card: &str) -> Vec<String> {
        s.stats(card)
            .keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    /// TS `rally({ radiant?, armor?, library? })`'s options.
    #[derive(Default)]
    struct Rally {
        radiant: bool,
        armor: Option<i32>,
        library: Option<Vec<&'static str>>,
    }

    /// The Flagbearer in p1's hand, beside a Felinor and a non-Felinor of p1's and a Felinor of p2's.
    fn rally(opts: Rally) -> Scenario {
        let mut flag = json!({ "def": FLAG });
        if opts.radiant {
            flag["radiant"] = json!(true);
        }
        let mut p1 = json!({
            "hand": [flag, HIT_JOB, FILLER],
            "field": [DUPE, VANILLA],
            "library": opts.library.unwrap_or_else(|| vec![FILLER]),
            "mana": 10,
        });
        if let Some(armor) = opts.armor {
            p1["armor"] = json!(armor);
        }
        scenario(json!({
            "p1": p1,
            "p2": { "hand": [TRUE_STRIKE, FILLER], "field": [DUPE, VANILLA] },
        }))
    }

    fn destroy_flag(s: &mut Scenario) {
        let id = s.card(FLAG).id.clone();
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": id }] }));
    }

    fn step_flag(s: &mut Scenario, steps: i32) {
        let id = s.card(FLAG).id.clone();
        step_param(find_instance_mut(s.state_mut(), &id).expect("the Flagbearer"), "aura", steps);
    }

    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialises")
    }

    #[test]
    fn is_a_2_felinor_catalyst_unit_4_4_rush_8_8_radiant_no_cleave_naming_its_prime() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(js(&def.tags), json!(["Felinor", "Catalyst"]));
        assert!(def.refs.clone().unwrap_or_default().contains(&PRIME.to_string()));
        let mut s = rally(Rally::default());
        s.play(FLAG, json!({}));
        s.expect_stats(FLAG, json!({ "attack": 4, "health": 4 }));
        assert_eq!(kinds(&s, FLAG), ["Rush"]);
    }

    mod base {
        use super::*;

        #[test]
        fn no_cry_playing_grants_no_armor_to_either_hero_in_any_view() {
            crate::register_all();
            let mut s = rally(Rally::default());
            s.play(FLAG, json!({}));
            assert_eq!(s.state().players.p1.hero.armor, 0);
            assert_eq!(s.view(P1).you.hero.armor, 0);
            assert_eq!(s.view(P2).opponent.hero.armor, 0);
            assert_eq!(s.state().players.p2.hero.armor, 0);
        }

        #[test]
        fn r124_hero_armor_3_stays_3_there_is_no_cry_to_stack_with_it() {
            crate::register_all();
            let mut s = rally(Rally {
                armor: Some(3),
                ..Rally::default()
            });
            s.play(FLAG, json!({}));
            assert_eq!(s.view(P1).you.hero.armor, 3);
        }

        #[test]
        fn s4_4_step_2_with_no_armor_from_it_a_4_attack_hit_deals_its_full_4() {
            crate::register_all();
            let mut s = rally(Rally::default());
            s.play(FLAG, json!({}));
            destroy_flag(&mut s);
            s.expect_in_zone(FLAG, "graveyard");
            s.end_turn();
            let attacker = s.unit(P2, 2).expect("p2's lane 2");
            s.attack(&attacker, "hero");
            s.expect_health(P1, 26);
            assert_eq!(s.view(P1).you.hero.armor, 0);
        }

        #[test]
        fn s4_4_r346_a_pierce_hit_skips_it_true_strike_deals_its_full_4() {
            crate::register_all();
            let mut s = rally(Rally::default());
            s.play(FLAG, json!({}));
            s.end_turn();
            s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
            s.expect_health(P1, 26);
        }

        #[test]
        fn s10_4_its_aura_gives_your_other_felinors_1_1_not_itself_not_a_non_felinor_not_the_opponent_s() {
            crate::register_all();
            let mut s = rally(Rally::default());
            s.play(FLAG, json!({}));
            let mine = s.unit(P1, 1).expect("p1's lane 1");
            s.expect_stats(&mine, json!({ "attack": 4, "health": 5 }));
            s.expect_stats(FLAG, json!({ "attack": 4, "health": 4 }));
            let vanilla = s.unit(P1, 2).expect("p1's lane 2");
            s.expect_stats(&vanilla, json!({ "attack": 4, "health": 4 }));
            let theirs = s.unit(P2, 1).expect("p2's lane 1");
            s.expect_stats(&theirs, json!({ "attack": 3, "health": 4 }));
        }

        #[test]
        fn s3_2_r13_a_felinor_dormant_under_a_stack_pile_gets_no_aura_the_one_on_top_does() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FLAG, FILLER], "field": [DUPE, { "def": DUPE, "stack": true }], "mana": 10 },
                "p2": { "hand": [FILLER] },
            }));
            let top = s.unit(P1, 1);
            let buried = s
                .state()
                .players
                .p1
                .units
                .first()
                .and_then(|pile| pile.as_ref())
                .and_then(|pile| {
                    pile.iter()
                        .find(|card| Some(&card.id) != top.as_ref().map(|top| &top.id))
                })
                .cloned();
            let (Some(top), Some(buried)) = (top, buried) else {
                panic!("no pile");
            };
            s.play(FLAG, json!({}));
            s.expect_stats(&top, json!({ "attack": 4, "health": 5 }));
            s.expect_stats(&buried, json!({ "attack": 3, "health": 4 }));
        }

        #[test]
        fn s10_4_the_aura_lasts_only_while_it_is_on_the_field() {
            crate::register_all();
            let mut s = rally(Rally::default());
            s.play(FLAG, json!({}));
            destroy_flag(&mut s);
            let mine = s.unit(P1, 1).expect("p1's lane 1");
            s.expect_stats(&mine, json!({ "attack": 3, "health": 4 }));
        }

        #[test]
        fn s6_3_its_death_shuffles_a_felinor_flagbearer_prime_into_your_deck_shown_in_your_library_list_r311() {
            crate::register_all();
            let mut s = rally(Rally::default());
            s.play(FLAG, json!({}));
            destroy_flag(&mut s);
            let prime: Vec<CardInstance> = s
                .pile(P1, "library")
                .into_iter()
                .filter(|card| card.def_id == PRIME)
                .collect();
            assert_eq!(prime.len(), 1);
            assert!(!prime[0].radiant);
            let listed = s.view(P1).you.own_library.map(|library| library.cards).unwrap_or_default();
            assert!(listed.contains(&LibraryEntryView {
                def_id: PRIME.to_string(),
                radiant: false,
                count: 1,
            }));
            let id = prime.first().map(|card| card.id.clone()).unwrap_or_else(|| "?".to_string());
            assert!(!serde_json::to_string(&s.view(P2)).expect("serialises").contains(&id));
        }

        #[test]
        fn r80_a_full_deck_turns_the_prime_away() {
            crate::register_all();
            let mut s = rally(Rally {
                library: Some(vec![FILLER; LIBRARY_CAP as usize]),
                ..Rally::default()
            });
            s.play(FLAG, json!({}));
            destroy_flag(&mut s);
            assert!(!s.pile(P1, "library").iter().any(|card| card.def_id == PRIME));
            assert_eq!(s.pile(P1, "library").len(), LIBRARY_CAP as usize);
        }

        #[test]
        fn r386_an_upgrade_makes_the_aura_2_2_no_armor_left_to_tune() {
            crate::register_all();
            let mut s = rally(Rally::default());
            step_flag(&mut s, 1);
            s.play(FLAG, json!({}));
            assert_eq!(s.view(P1).you.hero.armor, 0);
            let mine = s.unit(P1, 1).expect("p1's lane 1");
            s.expect_stats(&mine, json!({ "attack": 5, "health": 6 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s5_2_8_8_rush_no_cleave_no_cry_armor_on_the_radiant_face_either() {
            crate::register_all();
            let mut s = rally(Rally {
                radiant: true,
                ..Rally::default()
            });
            s.play(FLAG, json!({}));
            assert_eq!(kinds(&s, FLAG), ["Rush"]);
            assert_eq!(s.view(P1).you.hero.armor, 0);
        }

        #[test]
        fn s10_4_its_aura_gives_every_felinor_unit_you_control_2_2_itself_included() {
            crate::register_all();
            let mut s = rally(Rally {
                radiant: true,
                ..Rally::default()
            });
            s.play(FLAG, json!({}));
            s.expect_stats(FLAG, json!({ "attack": 10, "health": 10 }));
            let mine = s.unit(P1, 1).expect("p1's lane 1");
            s.expect_stats(&mine, json!({ "attack": 5, "health": 6 }));
            let vanilla = s.unit(P1, 2).expect("p1's lane 2");
            s.expect_stats(&vanilla, json!({ "attack": 4, "health": 4 }));
            let theirs = s.unit(P2, 1).expect("p2's lane 1");
            s.expect_stats(&theirs, json!({ "attack": 3, "health": 4 }));
        }

        #[test]
        fn s6_3_its_death_shuffles_in_a_base_prime() {
            crate::register_all();
            let mut s = rally(Rally {
                radiant: true,
                ..Rally::default()
            });
            s.play(FLAG, json!({}));
            destroy_flag(&mut s);
            let prime = s.pile(P1, "library").into_iter().find(|card| card.def_id == PRIME);
            assert_eq!(prime.map(|card| card.radiant), Some(false));
        }

        #[test]
        fn r386_the_radiant_aura_steps_from_2() {
            crate::register_all();
            let mut s = rally(Rally {
                radiant: true,
                ..Rally::default()
            });
            step_flag(&mut s, 1);
            s.play(FLAG, json!({}));
            assert_eq!(s.view(P1).you.hero.armor, 0);
            s.expect_stats(FLAG, json!({ "attack": 11, "health": 11 }));
        }
    }
}
