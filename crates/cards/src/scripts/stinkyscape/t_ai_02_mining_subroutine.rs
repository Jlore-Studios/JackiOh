//! S T-AI-2 Mining Subroutine (SPEC §7, §8.10 row T-AI-2; R482, R1840002, R1881000, R2250000): (0) Spell,
//! AI, Token (no printed rarity), an AI generated token of the Stinkyscape set, built by S21.
//!   Base:    "Gain a random Common or Rare Stinkyscape Resource."
//!   Radiant: "Gain a random Radiant Common or Rare Stinkyscape Resource."
//! Engine: `gain_resources` (R1840002) from a pool of the Common and Rare Resources named by id: the
//!   rarity roll over the two rarities the pool holds (55 against 25), then one of the rolled rarity, into
//!   the hand, which a full hand burns (R2250000). It never gains Axnit,
//!   Narcor or Red Narcor, so a bought copy never pays for itself in Credits (R1881000). Radiant: a
//!   Radiant Resource. Declares no numbers (R482).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-ai-02";

/// R1881000: the Common and Rare Resources (§8.10 T-reknite, T-korrelite, T-gellium).
const POOL: [&str; 3] = [
    "stinkyscape-t-reknite",
    "stinkyscape-t-korrelite",
    "stinkyscape-t-gellium",
];

/// "a random … Resource": one.
const GAINED: i32 = 1;

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![gain_resources(GainResourcesArgs {
                count: GAINED,
                player: None,
                radiant: radiant.then_some(true),
                pool: Some(POOL.iter().map(|id| (*id).to_string()).collect()),
                fill: None,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S T-AI-2 Mining Subroutine — SPEC §8.10 row T-AI-2, BUILD M12 row S T-AI-2 (R1881000, R2250000).
#[cfg(test)]
mod tests {
    use super::POOL;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MINING: &str = "stinkyscape-t-ai-02";
    const FILLER: &str = "core-005";

    fn cast(radiant: bool, seed: &str, more: Value) -> Scenario {
        let mut hand = vec![json!({ "def": MINING, "radiant": radiant }), json!(FILLER)];
        hand.extend(more.as_array().cloned().unwrap_or_default());
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(MINING, json!({}));
        s
    }

    #[test]
    fn r1881000_it_gains_one_common_or_rare_resource_into_the_hand() {
        let mut seen: IndexSet<String> = IndexSet::new();
        for i in 0..40 {
            let s = cast(false, &format!("mining-{i}"), json!([]));
            let in_hand = resources_in_hand_of(s.state(), P1);
            assert_eq!(in_hand.len(), 1, "one Resource, in the hand");
            assert!(POOL.contains(&in_hand[0].def_id.as_str()), "{}", in_hand[0].def_id);
            assert!(!in_hand[0].radiant);
            seen.insert(in_hand[0].def_id.clone());
        }
        assert_eq!(seen.len(), POOL.len(), "each of the three over the seeds: {seen:?}");
    }

    #[test]
    fn a_full_hand_burns_the_gain() {
        let more = json!(vec![FILLER; HAND_CAP as usize - 1]);
        let s = cast(false, "mining-full", more);
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        assert!(resources_in_hand_of(s.state(), P1).is_empty());
        assert!(
            s.last_events()
                .iter()
                .any(|event| event.event_type() == GameEventType::Burned)
        );
    }

    #[test]
    fn radiant_a_radiant_resource() {
        let s = cast(true, "mining-radiant", json!([]));
        let in_hand = resources_in_hand_of(s.state(), P1);
        assert_eq!(in_hand.len(), 1);
        assert!(in_hand[0].radiant);
        assert!(POOL.contains(&in_hand[0].def_id.as_str()));
    }

    #[test]
    fn r1840000_the_opponent_sees_only_that_one_was_gained() {
        let s = cast(false, "mining-hidden", json!([]));
        let gained = resources_in_hand_of(s.state(), P1)[0].clone();
        let count = s.hand(P1).len() as i32;
        assert_eq!(s.view(P2).opponent.hand, HandView::Count { count });
        let gain_seen_by = |viewer: PlayerId| -> GameEvent {
            s.view(viewer)
                .events
                .into_iter()
                .find(|event| event.event_type() == GameEventType::AddedToHand)
                .expect("the view carries the gain")
        };
        assert!(matches!(
            gain_seen_by(P1),
            GameEvent::AddedToHand { instance_id, def_id, .. }
                if instance_id == gained.id && def_id == gained.def_id
        ));
        assert!(
            matches!(
                gain_seen_by(P2),
                GameEvent::AddedToHand { instance_id, def_id, .. }
                    if instance_id == HIDDEN_ID && def_id == HIDDEN_ID
            ),
            "one was gained, not which"
        );
    }

    #[test]
    fn r1840002_its_pool_rolls_common_55_against_rare_25() {
        const ROLLS: i32 = 10_000;
        const SLACK: f64 = 2.0;
        let s = cast(false, "mining-odds", json!([]));
        let pool: Vec<String> = POOL.iter().map(|id| (*id).to_string()).collect();
        let mut rng = Rng::new("mining-odds", 0);
        let mut by_def: IndexMap<String, i32> = IndexMap::new();
        for _ in 0..ROLLS {
            let id = roll_resource(s.state(), &mut rng, Some(&pool)).expect("the pool holds Resources");
            *by_def.entry(id).or_default() += 1;
        }
        let share = |ids: &[&str]| -> f64 {
            let seen: i32 = ids.iter().map(|id| by_def.get(*id).copied().unwrap_or(0)).sum();
            f64::from(seen) * 100.0 / f64::from(ROLLS)
        };
        let rare = 25.0 * 100.0 / 80.0;
        assert!(
            (share(&[POOL[2]]) - rare).abs() <= SLACK,
            "Gellium {}%",
            share(&[POOL[2]])
        );
        assert!((share(&POOL[..2]) - (100.0 - rare)).abs() <= SLACK);
        assert!((share(&[POOL[0]]) - share(&[POOL[1]])).abs() <= 2.0 * SLACK);
        assert_eq!(by_def.len(), POOL.len(), "never another Resource: {by_def:?}");
    }
}
