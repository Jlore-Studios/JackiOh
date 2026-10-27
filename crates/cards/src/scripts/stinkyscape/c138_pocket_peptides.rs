//! S #138 Pocket Peptides (SPEC §8.10 row 138; R70, R460, R946, R1875006): (1) Trap, Common, of the held
//! Stinkyscape set (R1420).
//!   Base:    "Reveals when one of your Units would die in combat: Cast CN Peptides on it."
//!   Radiant: "Reveals when one of your Units would die in combat: Cast a Radiant CN Peptides on each of
//!            your Units."
//! Engine: set face-down, it is a `wouldDie` replacement (§4.5 step 1, before any card moves; §6.3
//! Replacement): it answers when one of the Units the check collected is its controller's and would die
//! in combat — a hit killed it and the last hit it took was combat damage (`DyingUnit.combat`,
//! R1875006), never a Spell's hit, a text's or a destroy. It fires and is spent, and its follow-up
//! resolves at once (`resolveFirst`): a free cast of M #36 CN Peptides (R70) aimed at the first such
//! Unit in R68's order (R946, an aim it cannot take casting nothing, R1180005), so 50% it is Buffed its
//! times, which may lift its Health past its damage, and otherwise it is destroyed; the check after the
//! cast's effect judges it on the board the cast left (R460): a Unit the Buffs lifted lives. Radiant: a
//! Radiant CN Peptides (Lucky 1, so its coin rolls twice, R1440) cast once at each of its controller's
//! Units, the dying one first and then the rest in lane order (R1875006). Declares no numbers: CN
//! Peptides' 50% and its count are its own.

use jackioh_engine::effects::{ForEachCardArgs, cast_new, for_each_card};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-138";

/// M #36 CN Peptides, what it casts.
const CN_PEPTIDES: &str = "meditative-036";

/// The follow-up its firing resolves (`ReplacementDef.then`).
const CAST_STEP: &str = "peptides";

/// R1875006: the first of `controller`'s Units the event names that would die in combat.
fn first_in_combat(event: &ReplacedEvent, controller: PlayerId) -> Option<String> {
    let ReplacedEvent::WouldDie { units } = event else {
        return None;
    };
    units
        .iter()
        .find(|unit| unit.controller == controller && unit.combat == Some(true))
        .map(|unit| unit.instance_id.clone())
}

/// R1875006: the Units it casts at — the one dying in combat first, so its own Buffs decide it; on the
/// Radiant face then each other Unit of its controller's, in lane order. Each cast is aimed only, so a
/// Unit an earlier cast took off the field has nothing cast at it.
fn aims(ctx: &EffectContext<'_>, radiant: bool) -> Vec<String> {
    let Some(dying) = replacement_of(ctx).and_then(|record| first_in_combat(&record.event, ctx.controller)) else {
        return Vec::new();
    };
    let mut aims = vec![dying.clone()];
    if radiant {
        aims.extend(
            active_units_of(&*ctx.state, ctx.controller)
                .iter()
                .map(|unit| unit.id.clone())
                .filter(|id| *id != dying),
        );
    }
    aims
}

fn pocket_peptides(radiant: bool) -> Script {
    Script {
        replacements: vec![ReplacementDef {
            id: "pocket-peptides".into(),
            on: ReplacementMoment::WouldDie,
            where_: None,
            when: Some(replacement_when(|rc| first_in_combat(rc.event, rc.controller).is_some())),
            instead: ReplacementInstead {
                resolve_first: Some(true),
                ..ReplacementInstead::default()
            },
            then: Some(CAST_STEP.into()),
            by: None,
        }],
        resume: IndexMap::from([(
            CAST_STEP,
            hook(move |_ctx| {
                // Read once as the list reaches it (R113), so a pause inside a cast resumes over the same
                // Units: the casts' number hangs on the board.
                vec![for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |ctx: &mut EffectContext<'_>| aims(ctx, radiant)),
                    each: Arc::new(move |aim: &str| {
                        cast_new(json_as(json!({
                            "def": CN_PEPTIDES,
                            "radiant": radiant,
                            "aimAt": aim,
                            "aimOnly": true,
                        })))
                    }),
                })]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: pocket_peptides(false),
        radiant: pocket_peptides(true),
    }
}

// S #138 Pocket Peptides — SPEC §8.10 row 138, BUILD M12 row S 138: "set, it fires when a Unit of its
// controller's would die from combat damage, not from a Spell's hit or a destroy; the CN Peptides cast on
// it Buffs it or destroys it (both seen over the seeds); a Buff that lifts its Health past its damage keeps
// it alive; radiant each of its controller's Units has a Radiant CN Peptides cast on it, its Lucky 1
// rolling twice".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{CN_PEPTIDES, ID};

    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MOTHS: &str = "core-009"; // (2) 1/14.
    const GENN: &str = "classic-086"; // (4) 14/14.
    const HIT_JOB: &str = "core-016"; // (3) Spell: "Destroy a Unit."
    const SORCERER: &str = "core-068"; // (2) 5/5: "Cry: Deal 4 damage."

    /// p1, active with 10 mana, has a Vanilla in unit zone 1 and holds `hand`; p2 has Pocket Peptides
    /// (`radiant` its face) set in backrow zone 1 and `field` in its unit zones.
    fn trapped(seed: &str, radiant: bool, hand: Value, field: Value) -> Scenario {
        let mut cards = vec![json!(FILLER)];
        cards.extend(hand.as_array().cloned().unwrap_or_default());
        let mut s = crate::scenario(json!({
            "seed": seed,
            "p1": {
                "hand": cards,
                "field": [{ "def": VANILLA, "lane": 1 }],
                "library": crate::filler(6),
                "mana": 10,
            },
            "p2": {
                "hand": [FILLER],
                "field": field,
                "backrow": [{ "def": ID, "radiant": radiant, "lane": 1 }],
                "library": crate::filler(6),
            },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    fn fired(s: &Scenario) -> bool {
        s.last_events().iter().any(|event| event.event_type() == GameEventType::TrapFired)
    }

    /// The CN Peptides casts among the last events, each with whether it was Radiant.
    fn peptides_cast(s: &Scenario) -> Vec<bool> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardAnnounced { def_id, instance_id, .. } if def_id == CN_PEPTIDES => {
                    Some(s.card(instance_id.as_str()).radiant)
                }
                _ => None,
            })
            .collect()
    }

    /// p1's Vanilla attacks p2's Vanilla in unit zone 2 on the seed `seed`; whether p2's Vanilla lives.
    fn defender_lives(seed: &str) -> bool {
        let mut s = trapped(seed, false, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));
        let defender = s.unit_at(P2, 2);
        s.attack(s.unit_at(P1, 1), &defender);
        s.card(defender.id.as_str()).zone.z() == ZoneName::Field
    }

    mod base {
        use super::*;

        #[test]
        fn r1875006_a_unit_that_would_die_in_combat_has_cn_peptides_cast_on_it() {
            let mut s = trapped("pocket-peptides", false, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));
            let defender = s.unit_at(P2, 2);
            s.attack(s.unit_at(P1, 1), &defender);
            assert!(fired(&s));
            s.expect_in_zone(ID, "graveyard");
            assert_eq!(peptides_cast(&s), vec![false]);
            // p2 cast it, and it lands in p2's graveyard.
            assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == CN_PEPTIDES));
        }

        #[test]
        fn r1875006_the_cast_buffs_it_alive_or_destroys_it_both_over_the_seeds() {
            let lives = seed_where("pocket-peptides-lives", defender_lives);
            let dies = seed_where("pocket-peptides-dies", |seed| !defender_lives(seed));
            // A Buff that lifted its Health past its damage kept it: its Health is above 0.
            let mut s = trapped(&lives, false, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));
            let defender = s.unit_at(P2, 2);
            s.attack(s.unit_at(P1, 1), &defender);
            assert!(s.stats(defender.id.as_str()).health > 0);
            assert!(s.last_events().iter().any(|event| event.event_type() == GameEventType::Upgraded));
            let mut s = trapped(&dies, false, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));
            let defender = s.unit_at(P2, 2);
            s.attack(s.unit_at(P1, 1), &defender);
            s.expect_in_zone(defender.id.as_str(), "graveyard");
        }

        #[test]
        fn r1875006_a_spells_hit_or_a_destroy_does_not_fire_it() {
            let mut s = trapped(
                "pocket-peptides-spell",
                false,
                json!([SORCERER, HIT_JOB]),
                json!([{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 3 }]),
            );
            let ping = s.unit_at(P2, 3);
            s.play(SORCERER, json!({ "zone": 2, "targets": pick::instance(&ping) }));
            assert!(!fired(&s), "a Cry's hit is not combat");
            s.expect_in_zone(ping.id.as_str(), "graveyard");
            let vanilla = s.unit_at(P2, 2);
            s.play(HIT_JOB, json!({ "targets": pick::instance(&vanilla) }));
            assert!(!fired(&s), "a destroy is not combat");
            assert_eq!(s.backrow(P2, 1).map(|card| card.def_id), Some(ID.to_string()));
        }

        #[test]
        fn r1875006_the_enemys_unit_dying_in_combat_does_not_fire_it() {
            let mut s = trapped("pocket-peptides-enemy", false, json!([]), json!([{ "def": GENN, "lane": 1 }]));
            // p1's Vanilla dies in combat against the 14/14; the trap is p2's, and only p2's Units count.
            let attacker = s.unit_at(P1, 1);
            s.attack(&attacker, s.unit_at(P2, 1));
            s.expect_in_zone(attacker.id.as_str(), "graveyard");
            assert!(!fired(&s));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1875006_each_of_its_controllers_units_has_a_radiant_cn_peptides_cast_on_it() {
            let mut s = trapped(
                "pocket-peptides-radiant",
                true,
                json!([]),
                json!([{ "def": MOTHS, "lane": 1 }, { "def": VANILLA, "lane": 2 }, { "def": MOTHS, "lane": 4 }]),
            );
            // The dying Vanilla's cast first, then the two Moths' in lane order.
            let lanes: Vec<Vec<String>> = [2, 1, 4].iter().map(|lane| vec![s.unit_at(P2, *lane).id]).collect();
            let defender = s.unit_at(P2, 2);
            s.attack(s.unit_at(P1, 1), &defender);
            assert!(fired(&s));
            // Three Radiant casts (Lucky 1: each coin rolls twice, R1440), one at each Unit.
            assert_eq!(peptides_cast(&s), vec![true; 3]);
            let aimed: Vec<Vec<String>> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::CardAnnounced { def_id, targets, .. } if def_id == CN_PEPTIDES => Some(targets.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(aimed, lanes);
            assert!(s.state().pending.is_none());
        }
    }
}
