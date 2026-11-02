//! TG #32 Selfless Protector (SPEC §8.11 row 32): (1) Unit, Human, Epic, 2/1 → 4/2, a card of the held
//! Training Grounds set (R1420, R2194000).
//!   Base:    "Death: Give your Units Divine Shield."
//!   Radiant: "Cry: Give your Units Divine Shield. / Death: Give your Units Divine Shield."
//! Engine: C+ #12.5 Anti-Waffle Shell's Cry: Divine Shield (§10.4) to each Unit its controller has on the
//!   field then (`active_units_of`: pile tops, R13; standing Units, R1081000). The Death runs after the
//!   state check took this card off (R78, R89), so it reaches neither itself nor a Unit dying in that
//!   check; the Radiant Cry shields itself too (Hearthstone's "your minions"). A spent shield is up again.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-032";

/// "Give your Units Divine Shield."
fn shield_your_units(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    active_units_of(ctx.state, ctx.controller)
        .iter()
        .map(|unit| {
            grant_keyword(json_as(json!({
                "target": { "of": "instance", "instanceId": unit.id },
                "keyword": { "kind": "Divine Shield" },
            })))
        })
        .collect()
}

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(shield_your_units)),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(shield_your_units)),
        death: Some(hook(shield_your_units)),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// TG #32 Selfless Protector — SPEC §8.11 row 32, BUILD M13 row TG 32 (R13).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const VANILLA: &str = "core-008"; // (1) Mr. Vanilla, 4/4.
    const FIENDER: &str = "core-092"; // (2) Felinor Fiender, 5/7 Stack.
    const STATE_OF_THE_GAME: &str = "classic-041"; // (1) 3/3 Indestructible.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const PLAGUE_NUKE: &str = "classic-043"; // (4) Spell: Destroy all Units.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.

    fn board(radiant: bool, mine: Value, hand: Value) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant, "lane": 1 })];
        field.extend(mine.as_array().cloned().unwrap_or_default());
        crate::scenario(json!({
            "seed": "selfless-protector",
            "p1": { "field": field, "hand": hand, "library": crate::filler(4), "mana": 30 },
            "p2": { "field": [VANILLA], "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    fn shielded(s: &Scenario, id: &str) -> bool {
        unit_has(s.state(), s.card(id), KeywordKind::DivineShield)
    }

    fn kill_protector(s: &mut Scenario) {
        let me = s.unit_at(P1, 1).id;
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": me }] }));
        s.expect_in_zone(me.as_str(), "graveyard");
    }

    mod base {
        use super::*;

        #[test]
        fn its_death_gives_every_other_unit_of_yours_divine_shield() {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 4 }]), json!([HIT_JOB, FILLER]));
            let mine = [s.unit_at(P1, 2).id, s.unit_at(P1, 4).id];
            assert!(mine.iter().all(|id| !shielded(&s, id)));
            kill_protector(&mut s);
            assert!(mine.iter().all(|id| shielded(&s, id)));
        }

        #[test]
        fn enemy_units_get_none() {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 2 }]), json!([HIT_JOB, FILLER]));
            let theirs = s.unit_at(P2, 1).id;
            kill_protector(&mut s);
            assert!(!shielded(&s, &theirs));
        }

        #[test]
        fn r89_units_dying_in_the_same_check_get_none() {
            let mut s = board(
                false,
                json!([{ "def": VANILLA, "lane": 2 }, { "def": STATE_OF_THE_GAME, "lane": 3 }]),
                json!([PLAGUE_NUKE, FILLER]),
            );
            let dying = s.unit_at(P1, 2).id;
            let warded = s.unit_at(P1, 3).id;
            s.play(PLAGUE_NUKE, json!({}));
            s.expect_in_zone(ID, "graveyard");
            s.expect_in_zone(dying.as_str(), "graveyard");
            assert!(shielded(&s, &warded), "the Indestructible one stood through the check");
            let granted: Vec<String> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::KeywordGranted { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect();
            assert!(!granted.contains(&dying), "{granted:?}");
        }

        #[test]
        fn r13_a_piles_lower_unit_is_not_reached() {
            let mut s = board(
                false,
                json!([{ "def": VANILLA, "lane": 2 }, { "def": FIENDER, "lane": 2, "stack": true }]),
                json!([HIT_JOB, FILLER]),
            );
            let top = s.unit_at(P1, 2).id;
            let under = s.card(VANILLA).id.clone();
            kill_protector(&mut s);
            assert!(shielded(&s, &top));
            assert!(s.card(under.as_str()).granted_keywords.is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_cry_gives_your_units_this_one_included_divine_shield() {
            let mut s = crate::scenario(json!({
                "seed": "selfless-protector-cry",
                "p1": {
                    "field": [{ "def": VANILLA, "lane": 2 }],
                    "hand": [{ "def": ID, "radiant": true }, FILLER],
                    "library": crate::filler(4),
                    "mana": 10,
                },
                "p2": { "field": [VANILLA], "hand": [FILLER], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            let me = s.unit_at(P1, 1).id;
            let view = s.stats(me.as_str());
            assert_eq!((view.attack, view.max_health), (4, 2));
            assert!(shielded(&s, &me));
            assert!(shielded(&s, &s.unit_at(P1, 2).id));
            assert!(!shielded(&s, &s.unit_at(P2, 1).id));
        }

        #[test]
        fn its_death_gives_your_other_units_divine_shield() {
            let mut s = board(true, json!([{ "def": VANILLA, "lane": 2 }]), json!([HIT_JOB, FILLER]));
            let mine = s.unit_at(P1, 2).id;
            kill_protector(&mut s);
            assert!(shielded(&s, &mine));
            assert!(!shielded(&s, &s.unit_at(P2, 1).id));
        }
    }
}
