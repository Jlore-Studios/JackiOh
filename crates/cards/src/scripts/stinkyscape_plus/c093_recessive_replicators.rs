//! SPEC §8.12 S+ #93 Recessive Replicators (Unit 4/4 → 8/8, cost 2, Rare), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Evolves when a friendly Unit dies. / Evolve: Gain “Death: Resummon that Unit.”"
//!   Radiant: "Evolves when a friendly Unit dies. / Evolve: Gain “Death: Resummon that Unit. Make it Radiant.”"
//!
//! R3542002: "that Unit" is the first friendly card that died as a Unit (R4423000, superseding R3542002's "whose
//! definition is a Unit"; a token included) whose death evolved this, remembered with its face (`Destroyed` is
//! the card's own trigger evolving itself, R3536001). A Unit set face-down by its Cloaked and destroyed before
//! it reveals died as a Field Trap and evolves nothing; an Animated Field Spell or Trap killed in a unit zone
//! died as a Unit and evolves it, and is resummoned as its printed card, which a summon places in the backrow.
//! `script_v0_6_9i` keeps the definition's type for v0.6.9l and before (R767000).
//! One that is already evolved, or that dies in the same pass as the Unit, remembers nothing more. Its Evolve
//! text gains one Death each run (`gainedDeaths`, a list in its memory): under High Evolutionary's Aura it
//! gains two or three, and the Death, read as it dies (R78), resummons a fresh copy of that Unit for each
//! (`summon`, R1: no Cry, R64's zone). A copy is Radiant when that Unit died Radiant, and always on the
//! Radiant face. A Replicators evolved with no such death (High Evolutionary's Cry) gains nothing. The preview
//! names that Unit on the field, to both seats: the death was public (R2476000).

use jackioh_engine::effects::{remember, summon};
use jackioh_engine::prelude::*;
use serde::{Deserialize, Serialize};

pub const ID: &str = "stinkyscapeplus-093";

/// The memory key of "that Unit".
const THAT_UNIT: &str = "thatUnit";
/// The memory key of the Deaths gained: one `ThatUnit` per run of its Evolve text.
const GAINED: &str = "gainedDeaths";
/// R2476000: the words of the text the preview follows.
const THAT_UNIT_LABEL: &str = "that Unit";

/// The Unit whose death evolved it, as it rides the card's memory (plain JSON).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ThatUnit {
    def_id: String,
    radiant: bool,
}

fn that_unit_in(stored: Option<&Value>) -> Option<ThatUnit> {
    serde_json::from_value(stored?.clone()).ok()
}

/// The Deaths gained so far; anything but a list reads as none.
fn gained_in(stored: Option<&Value>) -> Vec<ThatUnit> {
    match stored {
        Some(Value::Array(list)) => list.iter().filter_map(|unit| that_unit_in(Some(unit))).collect(),
        _ => Vec::new(),
    }
}

/// R2476000: "that Unit" after its words, once it is remembered, else the dash; on the field only.
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Field {
            return vec![];
        }
        let unit = that_unit_in(ctx.self_.memory.get(THAT_UNIT));
        vec![card_preview(
            ctx.state,
            THAT_UNIT_LABEL,
            unit.as_ref().map(|unit| (unit.def_id.as_str(), unit.radiant)),
        )]
    })
}

/// `by_print` reads a death by the dead card's definition, as v0.6.9l and before did (R767000).
fn replicators(radiant: bool, by_print: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "replicators-mourn",
            &[GameEventType::Destroyed],
            // The condition is read in `run`, as a `when` would be for a trigger that is no trap (R2271000).
            move |ctx, event| {
                let view: &EffectContext = ctx;
                let GameEvent::Destroyed {
                    instance_id,
                    def_id,
                    controller,
                    radiant,
                    ..
                } = event
                else {
                    return Vec::new();
                };
                let Some(this) = view.live_self() else {
                    return Vec::new();
                };
                let unit = if by_print {
                    def_of(Some(&*view.state), def_id).type_ == CardType::Unit
                } else {
                    is_unit_death(&*view.state, event)
                };
                if *controller != view.controller || *instance_id == this.id || this.evolved == Some(true) || !unit
                {
                    return Vec::new();
                }
                let unit = ThatUnit {
                    def_id: def_id.clone(),
                    radiant: *radiant == Some(true),
                };
                vec![
                    remember(json_as(json!({ "key": THAT_UNIT, "value": unit }))),
                    evolve(json_as(json!({ "target": { "of": "self" } }))),
                ]
            },
        )],
        evolve: Some(hook(|ctx| {
            let view: &EffectContext = ctx;
            let Some(unit) = that_unit_in(recalled(view, THAT_UNIT).as_ref()) else {
                return Vec::new();
            };
            let mut gained = gained_in(recalled(view, GAINED).as_ref());
            gained.push(unit);
            vec![remember(json_as(json!({ "key": GAINED, "value": gained })))]
        })),
        death: Some(hook(move |ctx| {
            let view: &EffectContext = ctx;
            gained_in(recalled(view, GAINED).as_ref())
                .iter()
                .map(|unit| {
                    summon(json_as(json!({ "defId": unit.def_id, "radiant": radiant || unit.radiant })))
                })
                .collect()
        })),
        preview: Some(preview()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: replicators(false, false),
        radiant: replicators(true, false),
    }
}

/// R767000: a Unit's death read by the dead card's definition (R4423000), through v0.6.9l; `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_9i() -> CardScripts {
    CardScripts {
        base: replicators(false, true),
        radiant: replicators(true, true),
    }
}

// S+ #93 Recessive Replicators — SPEC §8.12 row 93 (R3542002; R1, R64, R78, R2476000, R3536000, R3536003).
#[cfg(test)]
mod tests {
    use super::{ID, THAT_UNIT_LABEL};
    use jackioh_engine::testkit::*;

    use crate::{preview_seen, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HIT_JOB: &str = "core-016"; // Hit Job: destroy a Unit
    const NETHER: &str = "core-088"; // Twisting Nether: destroy all permanents
    const MAGIC_JAMMED: &str = "core-036"; // Magic Jammed: destroy target backrow card
    const FIELD_SPELL: &str = "stand-in (1) Field Spell";
    const EVOLUTIONARY: &str = "stinkyscapeplus-003"; // High Evolutionary: Cry: evolve a friendly permanent; evolutions twice.
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const OTHER: &str = "stand-in (2) 5/5 R10/10";

    /// p1's Replicators (Radiant if `radiant`) in lane 1 and `field` after it, with `hand` and three Hit Jobs.
    fn board(seed: &str, radiant: bool, field: Value, hand: Value) -> Scenario {
        let mut units = vec![json!({ "def": ID, "radiant": radiant })];
        units.extend(field.as_array().cloned().unwrap_or_default());
        let mut hand = hand.as_array().cloned().unwrap_or_default();
        hand.extend([json!(HIT_JOB), json!(HIT_JOB), json!(HIT_JOB)]);
        scenario(json!({
            "seed": seed,
            "p1": { "mana": 10, "hand": hand, "field": units, "library": crate::filler(6) },
            "p2": { "hand": crate::filler(1), "field": [VANILLA], "library": crate::filler(4) },
        }))
    }

    /// p1 kills the Unit of `player`'s in `lane` with a Hit Job.
    fn kill(s: &mut Scenario, player: PlayerId, lane: i32) {
        let target = pick::unit_at(s, player, lane);
        s.play(HIT_JOB, json!({ "targets": target }));
    }

    fn units_of(s: &Scenario, def: &str) -> Vec<CardInstance> {
        (1..=5).filter_map(|lane| s.unit(P1, lane)).filter(|unit| unit.def_id == def).collect()
    }

    fn summoned(s: &Scenario) -> usize {
        s.events().iter().filter(|event| matches!(event, GameEvent::Summoned { .. })).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r3542002_a_friendly_death_evolves_it_and_its_death_resummons_that_unit() {
            let mut s = board("s093-base", false, json!([VANILLA]), json!([]));
            let replicators = s.unit_at(P1, 1);
            let dead = s.unit_at(P1, 2);
            kill(&mut s, P1, 2);
            assert_eq!(s.card(&replicators).evolved, Some(true));
            assert_eq!(evolutions_of(s.state(), P1), 1);
            assert!(units_of(&s, VANILLA).is_empty(), "the Unit stays dead until the Replicators dies");

            kill(&mut s, P1, 1);
            s.expect_in_zone(&replicators, "graveyard");
            let copies = units_of(&s, VANILLA);
            assert_eq!(copies.len(), 1, "one Death gained: one fresh copy");
            assert_ne!(copies[0].id, dead.id, "a copy, not the dead instance itself");
            assert!(!copies[0].radiant);
            assert_eq!(copies[0].evolved, None);
            assert_eq!(s.unit_at(P1, 1).id, copies[0].id, "R64: the leftmost open zone");
            assert_eq!(summoned(&s), 1);
        }

        #[test]
        fn r3542002_a_radiant_unit_returns_radiant_on_the_base_face() {
            let mut s = board("s093-radiant-unit", false, json!([{ "def": VANILLA, "radiant": true }]), json!([]));
            kill(&mut s, P1, 2);
            kill(&mut s, P1, 1);
            let copies = units_of(&s, VANILLA);
            assert_eq!(copies.len(), 1);
            assert!(copies[0].radiant, "that Unit died Radiant");
        }

        #[test]
        fn r3542002_only_the_first_death_is_remembered() {
            let mut s = board("s093-first", false, json!([VANILLA, OTHER]), json!([]));
            let replicators = s.unit_at(P1, 1).id;
            kill(&mut s, P1, 2);
            kill(&mut s, P1, 3);
            let first = def_of(Some(s.state()), VANILLA).name.clone();
            assert_eq!(
                preview_seen(&s, P1, &replicators),
                vec![(THAT_UNIT_LABEL.to_string(), 1, Some(first))],
                "still the first Unit"
            );
            kill(&mut s, P1, 1);
            assert_eq!(units_of(&s, VANILLA).len(), 1);
            assert!(units_of(&s, OTHER).is_empty(), "the second death evolved nothing and was forgotten");
            assert_eq!(evolutions_of(s.state(), P1), 1);
        }

        #[test]
        fn r3542002_an_enemy_death_evolves_nothing() {
            let mut s = board("s093-enemy", false, json!([]), json!([]));
            let replicators = s.unit_at(P1, 1);
            kill(&mut s, P2, 1);
            assert_ne!(s.card(&replicators).evolved, Some(true));
            assert_eq!(evolutions_of(s.state(), P1), 0);
        }

        #[test]
        fn r3542002_under_high_evolutionary_it_resummons_twice() {
            let mut s = board("s093-twice", false, json!([EVOLUTIONARY, VANILLA]), json!([]));
            kill(&mut s, P1, 3);
            assert_eq!(evolutions_of(s.state(), P1), 1, "one evolving, two runs");
            kill(&mut s, P1, 1);
            assert_eq!(units_of(&s, VANILLA).len(), 2, "each run gained a Death");
        }

        #[test]
        fn r3542002_evolved_by_high_evolutionary_with_no_death_it_resummons_nothing() {
            let mut s = board("s093-no-death", false, json!([]), json!([EVOLUTIONARY]));
            let replicators = s.unit_at(P1, 1);
            s.play(EVOLUTIONARY, json!({ "zone": 2, "targets": pick::instance(&replicators) }));
            assert_eq!(s.card(&replicators).evolved, Some(true));
            let before = summoned(&s);
            kill(&mut s, P1, 1);
            s.expect_in_zone(&replicators, "graveyard");
            assert_eq!(summoned(&s), before, "its Death summoned nothing");
            assert_eq!(s.unit_at(P1, 2).def_id, EVOLUTIONARY);
            assert!(s.unit(P1, 1).is_none());
        }

        #[test]
        fn r3542002_one_that_dies_in_the_same_pass_never_evolved() {
            let mut s = board("s093-same-pass", false, json!([VANILLA]), json!([NETHER]));
            let replicators = s.unit_at(P1, 1);
            s.play(NETHER, json!({}));
            s.expect_in_zone(&replicators, "graveyard");
            assert_ne!(s.card(&replicators).evolved, Some(true));
            assert_eq!(evolutions_of(s.state(), P1), 0);
            assert_eq!(summoned(&s), 0);
        }
    }

    #[test]
    fn r3542002_a_friendly_card_that_is_no_unit_by_its_definition_evolves_nothing() {
        let mut s = scenario(json!({
            "seed": "s093-field-spell",
            "p1": {
                "mana": 10,
                "hand": [MAGIC_JAMMED],
                "field": [ID],
                "backrow": [{ "def": FIELD_SPELL, "lane": 3 }],
                "library": crate::filler(6),
            },
            "p2": { "hand": crate::filler(1), "library": crate::filler(4) },
        }));
        let replicators = s.unit_at(P1, 1);
        let spell = s.backrow(P1, 3).expect("the Field Spell");
        s.play(MAGIC_JAMMED, json!({ "targets": pick::instance(&spell) }));
        assert!(
            s.events().iter().any(|event| matches!(event, GameEvent::Destroyed { def_id, .. } if def_id == FIELD_SPELL)),
            "it died"
        );
        assert_ne!(s.card(&replicators).evolved, Some(true));
        assert_eq!(evolutions_of(s.state(), P1), 0);
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3542002_the_copy_is_radiant() {
            let mut s = board("s093-radiant", true, json!([VANILLA]), json!([]));
            kill(&mut s, P1, 2);
            kill(&mut s, P1, 1);
            let copies = units_of(&s, VANILLA);
            assert_eq!(copies.len(), 1);
            assert!(copies[0].radiant, "Make it Radiant");
            s.expect_stats(&copies[0], json!({ "attack": 12, "health": 12 }));
        }
    }

    /// R2476000: it names that Unit after "that Unit" on the field, to both seats, a Radiant one as Radiant;
    /// before it evolves it shows the dash.
    #[test]
    fn r2476000_it_names_that_unit_on_the_field_to_both_seats() {
        let mut s = board("s093-preview", false, json!([{ "def": VANILLA, "radiant": true }]), json!([]));
        let replicators = s.unit_at(P1, 1).id;
        let dash = vec![(THAT_UNIT_LABEL.to_string(), 0, Some("—".to_string()))];
        assert_eq!(preview_seen(&s, P1, &replicators), dash);
        let name = def_of(Some(s.state()), VANILLA).name.clone();
        kill(&mut s, P1, 2);
        let named = vec![(THAT_UNIT_LABEL.to_string(), 1, Some(format!("Radiant {name}")))];
        assert_eq!(preview_seen(&s, P1, &replicators), named);
        assert_eq!(preview_seen(&s, P2, &replicators), named);
    }

    /// R4423000: a friendly death evolves it by what the card was on the field as it died.
    mod r4423000_what_it_died_as {
        use super::*;

        const FILLER: &str = "core-005";

        /// p1 sets a Cloaked Toe Cracker face-down and destroys it with Magic Jammed before it reveals,
        /// its Replicators in unit lane 1; the Replicators' id and the set card's.
        fn jammed() -> (Scenario, String, String) {
            let mut s = scenario(json!({
                "seed": "s093-cloaked",
                "p1": {
                    "mana": 10,
                    "hand": [crate::CLOAKED_UNIT, MAGIC_JAMMED, FILLER],
                    "field": [ID],
                    "library": crate::filler(6),
                },
                "p2": { "hand": crate::filler(1), "library": crate::filler(4) },
            }));
            let replicators = s.unit_at(P1, 1).id;
            let set = crate::set_cloaked(&mut s, P1, crate::CLOAKED_UNIT, 2);
            s.play(MAGIC_JAMMED, json!({ "targets": pick::id(&set) }));
            (s, replicators, set)
        }

        #[test]
        fn r4423000_a_set_cloaked_unit_destroyed_before_it_reveals_is_no_unit_death_for_it() {
            let (s, replicators, set) = jammed();
            assert_eq!(crate::died_as_of(&s, &set), Some(CardType::FieldTrap));
            assert_ne!(s.card(replicators.as_str()).evolved, Some(true));
            assert_eq!(evolutions_of(s.state(), P1), 0);
        }

        /// p1's set Catnip animates after p2's Spell and p2's Hit Job kills it in its unit zone: it evolves
        /// the Replicators, whose Death resummons it as the printed Field Trap it is, in the backrow.
        #[test]
        fn r4423000_an_animated_field_trap_killed_in_a_unit_zone_is_a_unit_death_for_it() {
            let mut s = scenario(json!({
                "seed": "s093-animated",
                "active": "p2",
                "p1": {
                    "hand": [FILLER],
                    "field": [ID],
                    "backrow": [{ "def": crate::ANIMATED_TRAP, "faceUp": false, "lane": 2 }],
                    "library": crate::filler(6),
                },
                "p2": { "mana": 10, "hand": [FILLER, HIT_JOB, HIT_JOB, FILLER], "library": crate::filler(4) },
            }));
            let replicators = s.unit_at(P1, 1).id;
            s.play(FILLER, json!({}));
            let catnip = s.card(crate::ANIMATED_TRAP).id.clone();
            assert_eq!(s.unit_at(P1, 2).id, catnip, "Catnip animated");
            s.play(HIT_JOB, json!({ "targets": pick::id(&catnip) }));

            assert_eq!(crate::died_as_of(&s, &catnip), Some(CardType::Unit));
            let evolved = s.card(replicators.as_str()).clone();
            assert_eq!(evolved.evolved, Some(true));
            assert_eq!(evolved.memory["thatUnit"]["defId"], crate::ANIMATED_TRAP);

            s.play(HIT_JOB, json!({ "targets": pick::id(&replicators) }));
            s.expect_in_zone(&replicators, "graveyard");
            let back = (1..=5).filter_map(|lane| s.backrow(P1, lane)).find(|card| card.def_id == crate::ANIMATED_TRAP);
            assert!(back.is_some_and(|card| card.id != catnip), "a fresh Catnip in p1's backrow");
        }

        /// R767000: v0.6.9l and before evolved it on the set card's death, by the definition's type.
        #[test]
        fn r767000_the_script_kept_for_v0_6_9i_counts_a_set_cloaked_unit_s_death() {
            crate::under_kept(ID, super::super::script_v0_6_9i(), || {
                let (s, replicators, _) = jammed();
                assert_eq!(s.card(replicators.as_str()).evolved, Some(true));
            });
        }
    }
}
