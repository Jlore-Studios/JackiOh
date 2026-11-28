//! SPEC §8.14 FvH #144 Plagiarized: Doomsday Clock, a frozen copy of IA #205 Doomsday Clock (R2195003, FV4),
//! a card of the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Doomsday Clock`, tagged Plagiarized only, and a token it makes stays the IA token. Its
//! change from IA #205: it deals 20 (Radiant 40), from 10 (Radiant 20), its declared `damage`. The IA file's
//! notes, as they stood:
//!
//! IA #205 Doomsday Clock (SPEC §8.13 row 205), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Can't attack\nIn {turns|turn|turns}: Deal {damage} damage to the enemy hero."
//!   Radiant: "Can't attack\nIn {turns|turn|turns}: Deal {damage} damage to the enemy hero."
//!
//! Engine: Can't attack is the catalog's. "In N turns" starts when the card arrives on the field,
//! played or summoned (R4593005): the arrival's `Summoned` names it, and the trigger plants the
//! arrival controller's watched `delay` (R1181000) of the declared `turns` with its badge, built from
//! the declared `damage`, which the view follows with the countdown (R4992000), so it drops when the
//! card leaves the field (R174); due, it deals the declared `damage` to the enemy hero. Built by C9
//! (#4593).

use jackioh_engine::effects::{damage, delay};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-144";

/// The delayed step, which the `delayed` hook runs.
const STEP: &str = "doomsday";

/// This card arrived on the field (played or summoned): its `Summoned` names its own id.
fn arrived(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = ctx.live_self() else {
        return false;
    };
    matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == me.id)
}

fn doomsday() -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("doomsday", &[GameEventType::Summoned], |ctx, _event| {
                let Some(me) = ctx.live_self() else {
                    return vec![];
                };
                let id = me.id.clone();
                // The badge (R169) names the effect: the view adds " in N turns" (R4992000).
                let damage = param(&*ctx, "damage");
                let name = def_of(Some(&*ctx.state), ID).name.clone();
                vec![delay(json_as(json!({
                    "at": { "phase": "start", "player": "self" },
                    "step": STEP,
                    "turns": param(&*ctx, "turns"),
                    "label": format!("{name}: {damage} damage to the enemy hero"),
                    "watch": id,
                    "data": { "damage": damage },
                })))]
            })
            .with_when(arrived),
        ],
        delayed: Some(hook(|ctx| {
            let damage_amount = ctx.data.get("damage").and_then(Value::as_i64).unwrap_or(0) as i32;
            vec![damage(json_as(json!({
                "to": { "of": "enemyHero" },
                "amount": damage_amount,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's 40 (FV4; IA #205's 20), in its badge too, is its declared
    // `damage`.
    let radiant = doomsday();
    CardScripts {
        base: doomsday(),
        radiant,
    }
}

// FvH #144 Plagiarized: Doomsday Clock — SPEC §8.14 row 144; IA #205's tests (SPEC §8.13 row 205, R4593005,
// R4992000), on this card.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{next_turn_of, scenario};

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit.
    /// The card's name, which its badge reads off its catalog entry (FV4: `Plagiarized: <IA name>`).
    const NAME: &str = "Plagiarized: Doomsday Clock";

    fn dealt(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, HIT_JOB, FILLER],
                "library": crate::filler(6),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(8) },
        }))
    }

    fn played(radiant: bool, seed: &str) -> Scenario {
        let mut s = dealt(radiant, seed);
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    /// The Clock's badges on `viewer`'s view of the board (both seats see the same ones).
    fn badges(s: &Scenario, viewer: PlayerId) -> Vec<String> {
        let view = s.view(viewer);
        let seat = if viewer == P1 { view.you } else { view.opponent };
        seat.modifiers
            .into_iter()
            .map(|modifier| modifier.label)
            .filter(|label| label.starts_with(NAME))
            .collect()
    }

    /// R4992000: both seats read `damage` and the countdown once, then the next turn's.
    fn assert_badge_counts_down_once(s: &mut Scenario, damage: i32) {
        for viewer in [P1, P2] {
            assert_eq!(
                badges(s, viewer),
                vec![format!("{NAME}: {damage} damage to the enemy hero in 4 turns")],
                "{viewer:?} reads the damage and the countdown once"
            );
        }
        next_turn_of(s, P1);
        for viewer in [P1, P2] {
            let shown = badges(s, viewer);
            assert_eq!(
                shown,
                vec![format!("{NAME}: {damage} damage to the enemy hero in 3 turns")],
                "{viewer:?} reads the next turn's countdown once"
            );
            assert!(shown.iter().all(|label| !label.contains("turns in")), "no doubled count");
        }
    }

    fn hero(s: &Scenario) -> i32 {
        s.state().players[P2].hero.health
    }

    mod base {
        use super::*;

        #[test]
        fn r4593005_deals_20_at_the_4th_own_start_and_nothing_before() {
            let mut s = played(false, "doomsday-base");
            for _ in 0..3 {
                next_turn_of(&mut s, P1);
                assert_eq!(hero(&s), HERO_HEALTH, "the first three starts are quiet");
            }
            next_turn_of(&mut s, P1);
            // FV4: 20 (IA #205's base face deals 10).
            assert_eq!(hero(&s), HERO_HEALTH - 20);
        }

        #[test]
        fn r4992000_the_badge_says_20_damage_and_counts_down_once() {
            let mut s = played(false, "doomsday-badge");
            assert_badge_counts_down_once(&mut s, 20);
        }

        #[test]
        fn r4992000_a_buff_to_damage_changes_the_badge_and_the_hit() {
            let mut s = dealt(false, "doomsday-buff");
            // One better step of the declared `damage` (the card's step is 2, higher is better).
            s.card_mut(ID).tuning = Some(json_as(json!({ "numbers": { "damage": 1 } })));
            s.play(ID, json!({ "zone": 1 }));
            assert_badge_counts_down_once(&mut s, 22);
            for _ in 0..3 {
                next_turn_of(&mut s, P1);
            }
            assert_eq!(hero(&s), HERO_HEALTH - 22);
        }

        #[test]
        fn r4593005_nothing_fires_once_it_leaves_the_field() {
            let mut s = played(false, "doomsday-gone");
            let unit = s.unit_at(P1, 1);
            s.play(HIT_JOB, pick::targets(pick::instance(&unit)));
            for _ in 0..5 {
                next_turn_of(&mut s, P1);
            }
            assert_eq!(hero(&s), HERO_HEALTH);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593005_deals_40_at_the_4th_own_start() {
            let mut s = played(true, "doomsday-radiant");
            for _ in 0..3 {
                next_turn_of(&mut s, P1);
            }
            assert_eq!(hero(&s), HERO_HEALTH, "the first three starts are quiet");
            next_turn_of(&mut s, P1);
            // FV4: 40 (IA #205's Radiant face deals 20), past the enemy hero's Health: p1 wins.
            assert!(
                crate::hits(s.events()).iter().any(|(_, amount)| *amount == 40),
                "one hit of 40"
            );
            assert_eq!(s.state().result.map(|result| result.winner), Some(Winner::P1));
        }

        #[test]
        fn r4992000_the_badge_says_40_damage_on_the_radiant_face() {
            let mut s = played(true, "doomsday-badge-radiant");
            assert_badge_counts_down_once(&mut s, 40);
        }
    }
}
