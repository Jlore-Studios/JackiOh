//! M #45.1 Knowledge Breaker Prime (SPEC §8.8 row 45.1, SPEC §7 token): (4) Unit, CN, KY,
//! Prime, Token (printed Legendary), 6/18 → 12/36.
//!
//! Base:    "Rush
//!           Cry: Summon {traps|random Trap|random Traps}. Destroy every other CN or KY permanent.
//!           Aura: You may play your Units face-down as Animated Field Traps that reveal at the
//!           start of your next turn."
//! Radiant: "Rush, Poisonous
//!           Cry: Summon {traps|random Radiant Trap|random Radiant Traps}. Exile every other CN or KY
//!           permanent.
//!           Aura: You may play your Units face-down as Animated Field Traps that reveal at the
//!           start of your next turn."
//! Engine: five `summon_random` Traps then `destroy_all` (Radiant: `exile_all`), then the Aura.

use jackioh_engine::prelude::*;

use crate::query::TRAP_TYPES;

pub const ID: &str = "meditative-045-1";

fn face_down_units() -> FaceDownPlayHook {
    Arc::new(|_args| {
        vec![FaceDownPlayPermission {
            units: Some(true),
            ..FaceDownPlayPermission::default()
        }]
    })
}

fn prime(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let count = param(&*ctx, "traps");
            let mut effects: Vec<Effect> = (0..count)
                .map(|_| {
                    let mut args = json!({
                        "query": { "type": TRAP_TYPES },
                        "player": "self",
                    });
                    if radiant {
                        args["radiant"] = json!(true);
                    }
                    summon_random(json_as(args))
                })
                .collect();
            let scope = json!({
                "side": "any",
                "rows": ["units", "backrow"],
                "tags": ["CN", "KY"],
                "excludeSelf": true,
            });
            if radiant {
                effects.push(exile_all(json_as(scope)));
            } else {
                effects.push(destroy_all(json_as(scope)));
            }
            effects
        })),
        face_down_play: Some(face_down_units()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: prime(false),
        radiant: prime(true),
    }
}

// M #45.1 Knowledge Breaker Prime — SPEC §8.8 row 45.1, BUILD M10 row M 45.1.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PRIME: &str = "meditative-045-1";
    const BREAKER: &str = "meditative-045";
    const CN_UNIT: &str = "meditative-039"; // 赌石 Addict, a CN Unit.
    const KY_UNIT: &str = "meditative-079"; // Touched by KY, a KY Unit.
    const VANILLA: &str = "core-008";
    const ROCK: &str = "core-066"; // Indestructible.
    const MANA_WELL: &str = "core-006"; // An untagged Field Spell.
    const BEAUTY: &str = "meditative-039-5"; // Jade Beauty: CN, Indestructible.

    fn trap_count(s: &Scenario, player: PlayerId) -> usize {
        s.state().players[player]
            .backrow
            .iter()
            .flatten()
            .filter(|card| {
                matches!(
                    jackioh_engine::faces::card_type_of(s.state(), card),
                    CardType::Trap | CardType::FieldTrap
                )
            })
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn cry_sets_five_random_traps_fewer_on_a_full_backrow() {
            // Empty backrow: five random Traps arrive.
            let mut s = scenario(json!({
                "seed": "prime-traps",
                "p1": { "mana": 8, "hand": [PRIME], "library": [VANILLA] },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let prime = s.hand(P1).iter().find(|card| card.def_id == PRIME).cloned().unwrap();
            s.play(&prime.id, json!({ "zone": 1 }));
            assert_eq!(trap_count(&s, P1), 5);

            // Three backrow zones taken (by Field Spells, which no Trap count sees): only two
            // Traps fit.
            let mut full = scenario(json!({
                "seed": "prime-traps-full",
                "p1": {
                    "mana": 8,
                    "hand": [PRIME],
                    "backrow": [
                        { "def": MANA_WELL },
                        { "def": MANA_WELL },
                        { "def": MANA_WELL },
                    ],
                    "library": [VANILLA],
                },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let prime =
                full.hand(P1).iter().find(|card| card.def_id == PRIME).cloned().unwrap();
            full.play(&prime.id, json!({ "zone": 1 }));
            assert_eq!(trap_count(&full, P1), 2);
        }

        #[test]
        fn r1042_destroys_other_cn_ky_permanents_face_down_included_indestructible_stays() {
            let mut s = scenario(json!({
                "seed": "prime-destroy",
                "p1": {
                    "mana": 8,
                    "hand": [PRIME, CN_UNIT],
                    "field": [
                        { "def": BREAKER, "lane": 1 },
                        { "def": KY_UNIT, "lane": 2 },
                        { "def": ROCK, "lane": 3 },
                    ],
                    "library": [VANILLA],
                },
                "p2": {
                    "hand": [VANILLA],
                    "field": [
                        { "def": CN_UNIT, "lane": 1 },
                        { "def": BEAUTY, "lane": 2 },
                    ],
                    "library": [VANILLA],
                },
            }));
            // A CN/KY unit set face-down is a permanent too: set the spare CN Unit first, under
            // the Breaker's permission.
            let spare = s.hand(P1).iter().find(|card| card.def_id == CN_UNIT).cloned().unwrap();
            s.play(&spare.id, json!({ "zone": 1, "row": "backrow", "faceDown": "startOfNextTurn" }));
            let prime = s.hand(P1).iter().find(|card| card.def_id == PRIME).cloned().unwrap();
            s.play(&prime.id, json!({ "zone": 4 }));
            // Every other CN or KY permanent on both fields is gone: the CN and the KY Unit, the
            // set CN Unit, and the Breaker granter itself.
            assert!(s.unit(P1, 1).is_none());
            assert!(s.unit(P1, 2).is_none());
            assert!(s.unit(P2, 1).is_none());
            assert!(s.backrow(P1, 1).is_none());
            // The Indestructible Jade Beauty, CN, stays (R46); the untagged Rock is not reached;
            // the Prime is not "other".
            assert_eq!(s.unit(P2, 2).map(|card| card.def_id), Some(BEAUTY.to_string()));
            assert_eq!(s.unit(P1, 3).map(|card| card.def_id), Some(ROCK.to_string()));
            assert_eq!(s.unit(P1, 4).map(|card| card.def_id), Some(PRIME.to_string()));
        }

        #[test]
        fn r1043_its_aura_grants_the_same_play() {
            let s = scenario(json!({
                "seed": "prime-aura",
                "p1": {
                    "mana": 8,
                    "hand": [VANILLA],
                    "field": [{ "def": PRIME, "lane": 1 }],
                    "library": [VANILLA],
                },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let unit = s.hand(P1).iter().find(|card| card.def_id == VANILLA).cloned().unwrap();
            let set_plays: Vec<_> = legal_actions(s.state(), P1)
                .into_iter()
                .filter(|action| {
                    matches!(action, ActionBody::Play { instance_id, face_down: Some(_), .. } if instance_id == &unit.id)
                })
                .collect();
            assert!(!set_plays.is_empty());
            assert!(
                set_plays.iter().all(|action| {
                    matches!(
                        action,
                        ActionBody::Play { face_down: Some(RevealAt::StartOfNextTurn), .. }
                    )
                })
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1042_radiant_exiles_with_radiant_traps() {
            let def = crate::card_def(PRIME);
            assert!(def.radiant.keywords.contains(&Keyword::Poisonous));
            let mut s = scenario(json!({
                "seed": "prime-exile",
                "p1": {
                    "mana": 8,
                    "hand": [{ "def": PRIME, "radiant": true }],
                    "field": [{ "def": CN_UNIT, "lane": 1 }],
                    "library": [VANILLA],
                },
                "p2": {
                    "hand": [VANILLA],
                    "field": [{ "def": KY_UNIT, "lane": 1 }, { "def": BEAUTY, "lane": 2 }],
                    "library": [VANILLA],
                },
            }));
            let prime = s.hand(P1).iter().find(|card| card.def_id == PRIME).cloned().unwrap();
            s.play(&prime.id, json!({ "zone": 2 }));
            // Exiled, not destroyed: neither graveyard holds them, and the exile beats the
            // Indestructible Jade Beauty.
            assert!(s.unit(P1, 1).is_none());
            assert!(s.unit(P2, 1).is_none());
            assert!(s.unit(P2, 2).is_none());
            for player in [P1, P2] {
                assert!(
                    !s.state().players[player]
                        .graveyard
                        .iter()
                        .any(|card| [CN_UNIT, KY_UNIT, BEAUTY].contains(&card.def_id.as_str()))
                );
            }
            assert!(
                s.state().players[P1]
                    .exile
                    .iter()
                    .any(|card| card.def_id == CN_UNIT)
            );
            assert!(
                s.state().players[P2]
                    .exile
                    .iter()
                    .any(|card| card.def_id == KY_UNIT)
            );
            // Its Traps arrive Radiant: five set, all from the Trap pool.
            assert_eq!(trap_count(&s, P1), 5);
            assert!(s.state().players[P1].backrow.iter().flatten().all(|card| card.radiant));
        }
    }
}
