//! M+ #9 Magic Amplifier (SPEC §8.9 row 9, BUILD M11 row M+ 9): (1, Embiggened 2) Field Spell, Common.
//!   Base:    "Aura: Both players have Spell Damage +{spellDamage}.
//!             Embiggened (2): Also has "Cry: Add a random Spell or Field Spell that deals
//!             damage to your hand."" (2)
//!   Radiant: "… a random Radiant Spell or Field Spell that deals damage …" (4)
//!
//! Spell Damage aura for both players via spellDamageAura (R1180000, §4.4 step 0; M+ #11). Read via declared_or (R386).
//! Embiggened version adds random damage Spell/Field Spell via dealsDamage (#2179, §6.3, R81, R1, R1748001, R380, R60, §2.4).
//! Keeps script_v0_5_2 for replays (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-009";

/// The catalog's declared Spell Damage, read off the face that is up (R386).
fn declared_spell_damage(radiant: bool) -> i32 {
    match crate::card_def(ID)
        .params
        .as_ref()
        .and_then(|params| params.iter().find(|entry| entry.key == "spellDamage"))
    {
        Some(param) if radiant => param.radiant,
        Some(param) => param.base,
        None => panic!("{ID} declares spellDamage (catalog params)"),
    }
}

/// The Embiggened Cry's pool: "a random Spell or Field Spell that deals damage" (R1748001).
fn damage_dealers() -> Value {
    json!({ "type": ["Spell", "Field Spell"], "dealsDamage": true })
}

/// The printed "a random": one card per Cry.
const CARDS: i32 = 1;

fn amplifier(radiant: bool, embiggened_cry: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            spell_damage_aura: Some(SpellDamageAura {
                amount: declared_spell_damage(radiant),
                both: Some(true),
            }),
            ..StaticFlags::default()
        }),
        cry: embiggened_cry.then(|| {
            hook(move |ctx| {
                if !ctx.embiggened {
                    return vec![];
                }
                let mut args = json!({ "query": damage_dealers(), "count": CARDS });
                if radiant {
                    args["radiant"] = json!(true);
                }
                vec![add_random_from_catalog(json_as(args))]
            })
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: amplifier(false, true),
        radiant: amplifier(true, true),
    }
}

/// R767000: through v0.5.2 it had one price and no Cry; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_2() -> CardScripts {
    CardScripts {
        base: amplifier(false, false),
        radiant: amplifier(true, false),
    }
}

// M+ #9 Magic Amplifier — SPEC §8.9 row 9, BUILD M11 row M+ 9.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::damage::spell_damage_of;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TRUE_STRIKE: &str = "core-044";
    const SOLARIUS: &str = "classicplus-038";
    const ECHOES: &str = "core-040";
    const TESLA: &str = "classic-005";
    const COLLATERAL: &str = "core-034";
    const FILLER: &str = "core-008";

    fn game(seed: &str, p1: Value, p2: Value) -> Scenario {
        let mut sides = json!({
            "seed": seed,
            "p1": { "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        });
        for (seat, extra) in [("p1", p1), ("p2", p2)] {
            if let Value::Object(entries) = extra {
                for (key, value) in entries {
                    sides[seat][key.as_str()] = value;
                }
            }
        }
        crate::scenario(sides)
    }

    fn hero(player: PlayerId) -> Value {
        json!([{ "pick": "hero", "player": player.as_str() }])
    }

    fn health(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.health
    }

    #[test]
    fn r1180000_your_spells_deal_2_more_per_hit() {
        let mut s = game(
            "m+009-yours",
            json!({ "backrow": [ID], "hand": [TRUE_STRIKE] }),
            json!({}),
        );
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P2), 24);
    }

    #[test]
    fn r1180000_the_opponents_spells_deal_2_more_too() {
        let mut s = game(
            "m+009-theirs",
            json!({ "backrow": [ID] }),
            json!({ "hand": [TRUE_STRIKE, FILLER] }),
        );
        s.end_turn();
        s.play(TRUE_STRIKE, json!({ "targets": hero(P1) }));
        assert_eq!(health(&s, P1), 24);
    }

    #[test]
    fn r1180000_it_adds_to_units_spell_damage() {
        let mut s = game(
            "m+009-solarius",
            json!({ "backrow": [ID], "field": [SOLARIUS], "hand": [TRUE_STRIKE] }),
            json!({}),
        );
        assert_eq!(spell_damage_of(s.state(), P1), 4);
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P2), 22);
    }

    #[test]
    fn r1180000_a_field_spells_hit_is_not_raised() {
        let mut s = game(
            "m+009-field-spell",
            json!({ "backrow": [ID, ECHOES], "exile": [FILLER] }),
            json!({}),
        );
        s.start_turn();
        assert_eq!(health(&s, P2), 29);
    }

    #[test]
    fn r1180000_a_traps_hit_is_not_raised() {
        let mut s = game(
            "m+009-trap",
            json!({ "backrow": [ID, { "def": TESLA, "faceUp": false, "lane": 3 }] }),
            json!({ "hand": [FILLER, FILLER] }),
        );
        s.end_turn();
        s.play(FILLER, json!({ "zone": 2 }));
        let hits: Vec<i32> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { amount, .. } => Some(*amount),
                _ => None,
            })
            .collect();
        assert_eq!(hits, vec![4]);
    }

    #[test]
    fn r1180000_face_down_it_gives_nothing() {
        let mut s = game("m+009-face-down", json!({ "backrow": [ID] }), json!({}));
        let amp = s.backrow(P1, 1).expect("the amplifier");
        s.card_mut(&amp.id).set_as = Some(SetAs {
            reveal: RevealAt::Cloaked,
            set_turn: 1,
            echo: None,
            revealing: None,
        });
        s.card_mut(&amp.id).face_up = Some(false);
        assert_eq!(spell_damage_of(s.state(), P1), 0);
        assert_eq!(spell_damage_of(s.state(), P2), 0);
    }

    #[test]
    fn r1180000_gone_it_gives_nothing() {
        let mut s = game(
            "m+009-gone",
            json!({ "backrow": [ID] }),
            json!({ "hand": [COLLATERAL, FILLER], "mana": 9 }),
        );
        s.end_turn();
        let amp = s.backrow(P1, 1).expect("the amplifier");
        s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": amp.id }] }));
        s.expect_in_zone(amp.id.as_str(), "exile");
        assert_eq!(spell_damage_of(s.state(), P1), 0);
        assert_eq!(spell_damage_of(s.state(), P2), 0);
    }

    #[test]
    fn spell_damage_reads_through_param() {
        let mut s = game("m+009-param", json!({ "backrow": [ID] }), json!({}));
        let amp = s.backrow(P1, 1).expect("the amplifier").id.clone();
        set_param(s.card_mut(&amp), "spellDamage", 3);
        assert_eq!(spell_damage_of(s.state(), P1), 3);
        assert_eq!(spell_damage_of(s.state(), P2), 3);
    }

    #[test]
    fn radiant_both_players_have_spell_damage_4() {
        let mut s = game(
            "m+009-radiant",
            json!({ "backrow": [{ "def": ID, "radiant": true }], "hand": [TRUE_STRIKE] }),
            json!({}),
        );
        assert_eq!(spell_damage_of(s.state(), P2), 4);
        s.play(TRUE_STRIKE, json!({ "targets": hero(P2) }));
        assert_eq!(health(&s, P2), 22);
    }

    fn added(s: &Scenario) -> Vec<CardInstance> {
        let ids: Vec<String> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: PlayerId::P1, instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        ids.iter().map(|id| s.card(id.as_str()).clone()).collect()
    }

    fn deals_damage(card: &CardInstance) -> bool {
        let def = crate::card_def(&card.def_id);
        def.deals_damage == Some(true) && matches!(def.type_, CardType::Spell | CardType::FieldSpell) && !def.token
    }

    #[test]
    fn r81_played_at_1_it_adds_nothing() {
        let mut s = game("m+009-plain", json!({ "hand": [ID, FILLER] }), json!({}));
        s.play(ID, json!({}));
        assert!(added(&s).is_empty());
        assert_eq!(spell_damage_of(s.state(), P1), 2);
    }

    #[test]
    fn r81_r1748001_embiggened_its_cry_adds_one_random_spell_or_field_spell_that_deals_damage() {
        let mut types: Vec<CardType> = Vec::new();
        for n in 0..30 {
            let mut s = game(&format!("m+009-embiggened-{n}"), json!({ "hand": [ID, FILLER] }), json!({}));
            s.play(ID, json!({ "embiggen": true }));
            let made = added(&s);
            assert_eq!(made.len(), 1);
            assert!(deals_damage(&made[0]), "{}", made[0].def_id);
            assert!(!made[0].radiant);
            assert_eq!(spell_damage_of(s.state(), P1), 2);
            let type_ = crate::card_def(&made[0].def_id).type_;
            if !types.contains(&type_) {
                types.push(type_);
            }
        }
        assert!(types.contains(&CardType::Spell), "{types:?}");
    }

    #[test]
    fn r81_radiant_embiggened_the_card_is_radiant() {
        let mut s = game(
            "m+009-radiant-embiggened",
            json!({ "hand": [{ "def": ID, "radiant": true }, FILLER] }),
            json!({}),
        );
        s.play(ID, json!({ "embiggen": true }));
        let made = added(&s);
        assert_eq!(made.len(), 1);
        assert!(made[0].radiant);
        assert!(deals_damage(&made[0]));
        assert_eq!(spell_damage_of(s.state(), P1), 4);
    }

    #[test]
    fn r767000_a_game_on_v0_5_2_adds_nothing_played_at_either_price() {
        crate::under_kept(ID, super::script_v0_5_2(), || {
            let mut s = game("m+009-kept", json!({ "hand": [ID, FILLER] }), json!({}));
            s.play(ID, json!({ "embiggen": true }));
            assert!(added(&s).is_empty());
        });
    }
}
