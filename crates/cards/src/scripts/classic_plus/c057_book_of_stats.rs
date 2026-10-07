//! C+ #57 Book of Stats (SPEC §8.7 row 57). (1) Spell, Book, Epic.
//!   Base:    "Give a Unit +{buff}/+{buff}." — buff 5
//!   Radiant: the same text, buff 10.
//!   Engine:  "A target Unit on either side, declared at play (R81); a permanent buff (§10.4 layer 4).
//!            Tunes: buff 5 ↑."

use jackioh_engine::effects::buff;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-057";

// R656: a buff helps, so a random cast that targets enemies aims this at friends.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))
    }]
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "buff");
            vec![buff(json_as(json!({ "target": { "of": "chosen" }, "attack": amount, "health": amount })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 10 is its declared `buff`, which `param` reads.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #57 Book of Stats — SPEC §8.7 row 57, BUILD M9 Classic+ row C+ 57: "A target Unit on either side
// gets +5/+5 permanently, kept until it leaves the field; an Immune to Spells Unit is no legal target;
// the buff reads through `param()`; radiant +10/+10".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const BOOK: &str = "classicplus-057";
    const VANILLA: &str = "core-008"; // Mr. Vanilla 4/4
    const FLOOD: &str = "core-017"; // (4) Spell: bounce all Units
    const FILLER: &str = "core-005";

    /// `_harness.ts` registers every card on import; the engine's testkit cannot, so this does.
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    /// TS `stepParam(s.card(ref), key, steps)`: TS stepped the live card; here the card under its id.
    fn step(s: &mut Scenario, card: &str, key: &str, steps: i32) {
        let id = s.card(card).id.clone();
        let live = find_instance_mut(s.state_mut(), &id).expect("the card is in the state");
        step_param(live, key, steps);
    }

    /// TS `s.unit(p, lane) ?? ""`: the unit's instance id, or "" (which no card answers to).
    fn unit_id(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|card| card.id).unwrap_or_default()
    }

    fn stats(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": BOOK, "radiant": radiant }, FLOOD, FILLER], "field": [VANILLA] },
            "p2": { "hand": [FILLER], "field": [VANILLA] },
        }))
    }

    /// The play's `targets`: the unit in lane 1 of `player`'s side.
    fn at(s: &Scenario, player: PlayerId) -> Value {
        let unit = s.unit(player, 1).unwrap_or_else(|| panic!("no unit for {player}"));
        json!([{ "pick": "instance", "instanceId": unit.id }])
    }

    /// The instance ids `legalActions` offers Book of Stats as targets.
    fn offered(s: &Scenario) -> Vec<String> {
        let book = s.card(BOOK).id.clone();
        legal_actions(s.state(), PlayerId::P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == book => targets
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|pick| match pick {
                        Selection::Instance { instance_id } => Some(instance_id),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect()
    }

    #[test]
    fn is_a_1_spell_book_that_declares_one_unit_target_on_either_side() {
        crate::register_all();
        assert_eq!(crate::card_def(super::ID).id, BOOK);
        let scripts = super::script();
        assert_eq!(
            serde_json::to_value(&scripts.base.targets).unwrap(),
            json!([{ "kind": "target", "min": 1, "max": 1, "aim": "help", "filter": { "side": "any", "of": ["unit"] } }])
        );
        // TS `expect(radiant).toBe(base)`: the one Cry hook.
        assert!(std::sync::Arc::ptr_eq(
            scripts.radiant.cry.as_ref().expect("a Cry"),
            scripts.base.cry.as_ref().expect("a Cry"),
        ));
    }

    mod base {
        use super::*;

        #[test]
        fn s10_4_your_unit_gets_5_5() {
            let mut s = stats(false);
            let targets = at(&s, PlayerId::P1);
            s.play(BOOK, json!({ "targets": targets }));
            let unit = unit_id(&s, PlayerId::P1, 1);
            s.expect_stats(&unit, json!({ "attack": 9, "health": 9, "maxHealth": 9 }));
        }

        #[test]
        fn r81_an_enemy_unit_is_a_legal_target_too() {
            let mut s = stats(false);
            let targets = at(&s, PlayerId::P2);
            s.play(BOOK, json!({ "targets": targets }));
            let unit = unit_id(&s, PlayerId::P2, 1);
            s.expect_stats(&unit, json!({ "attack": 9, "health": 9 }));
        }

        #[test]
        fn s10_4_the_buff_is_permanent_through_the_turns_and_is_lost_when_the_unit_leaves_the_field_r78() {
            let mut s = stats(false);
            let targets = at(&s, PlayerId::P1);
            s.play(BOOK, json!({ "targets": targets }));
            let unit = unit_id(&s, PlayerId::P1, 1);
            s.end_turn().end_turn();
            s.expect_stats(&unit, json!({ "attack": 9, "health": 9 }));
            s.play(FLOOD, json!({}));
            assert_eq!(s.card(&unit).zone.z(), ZoneName::Hand);
            assert_eq!(s.card(&unit).buffs, AttackHealth { attack: 0, health: 0 });
        }

        #[test]
        fn r23_an_immutable_unit_still_takes_the_buff_a_buff_is_no_change_to_its_text() {
            let mut s = scenario(json!({
                "p1": { "hand": [BOOK, FILLER], "field": [{ "def": "core-019", "radiant": true }] },
                "p2": { "hand": [FILLER] },
            }));
            let targets = at(&s, PlayerId::P1);
            s.play(BOOK, json!({ "targets": targets }));
            let unit = unit_id(&s, PlayerId::P1, 1);
            s.expect_stats(&unit, json!({ "attack": 23, "health": 23 }));
        }

        #[test]
        fn e35_an_immune_to_spells_unit_is_no_legal_target() {
            let mut s = stats(false);
            let immune = s.unit(PlayerId::P2, 1).unwrap_or_else(|| panic!("no unit"));
            find_instance_mut(s.state_mut(), &immune.id)
                .expect("the unit is in the state")
                .granted_keywords
                .push(Keyword::ImmuneToSpells);
            assert!(!offered(&s).contains(&immune.id));
            assert!(offered(&s).contains(&unit_id(&s, PlayerId::P1, 1)));
            let targets = at(&s, PlayerId::P2);
            s.expect_refused(|s| s.play(BOOK, json!({ "targets": targets })));
        }

        #[test]
        fn r386_an_upgrade_gives_6_6_a_degrade_4_4() {
            let mut up = stats(false);
            step(&mut up, BOOK, "buff", 1);
            let targets = at(&up, PlayerId::P1);
            up.play(BOOK, json!({ "targets": targets }));
            let unit = unit_id(&up, PlayerId::P1, 1);
            up.expect_stats(&unit, json!({ "attack": 10, "health": 10 }));

            let mut down = stats(false);
            step(&mut down, BOOK, "buff", -1);
            let targets = at(&down, PlayerId::P1);
            down.play(BOOK, json!({ "targets": targets }));
            let unit = unit_id(&down, PlayerId::P1, 1);
            down.expect_stats(&unit, json!({ "attack": 8, "health": 8 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s10_4_the_unit_gets_10_10() {
            let mut s = stats(true);
            let targets = at(&s, PlayerId::P2);
            s.play(BOOK, json!({ "targets": targets }));
            let unit = unit_id(&s, PlayerId::P2, 1);
            s.expect_stats(&unit, json!({ "attack": 14, "health": 14, "maxHealth": 14 }));
        }

        #[test]
        fn r386_the_radiant_buff_steps_by_its_declared_step_an_upgrade_gives_11_11() {
            let mut s = stats(true);
            step(&mut s, BOOK, "buff", 1);
            let targets = at(&s, PlayerId::P1);
            s.play(BOOK, json!({ "targets": targets }));
            let unit = unit_id(&s, PlayerId::P1, 1);
            s.expect_stats(&unit, json!({ "attack": 15, "health": 15 }));
        }
    }
}
