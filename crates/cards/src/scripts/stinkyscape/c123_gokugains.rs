//! Stinkyscape #123 Gokugains (SPEC §8.10 row 123). (1) Unit, Rare, 2/1 → 4/2, Cloaked.
//!   Base:    "Cloaked\nCry: Secretly Nerf a random Unit in your opponent's hand {times|time|times}, never
//!            its cost. No one sees the Nerfs until it is played." — times 3
//!   Radiant: the same text — times 30
//!
//! Cloaked is the engine's (R783004): set face-down, its Cry runs as it is revealed (R783006). Cry (R1):
//! `degrade_secretly { scope: the enemy's hand Units, random: 1, times }` (R1845000): one random Unit
//! card of the opponent's hand that can take Nerfs (not Immutable) gets a hidden count, nothing drawn or
//! reported, no view showing it to either seat, Gokugains' controller included. It lands as that card is
//! next played, at §10.5 step 3, each Nerf drawn from every row but the cost's (R1845001). A hand with no
//! such Unit takes nothing and draws nothing (R129). The count is the declared `times` (R386), its Cry's.
//! The play pipeline runs the Cry of a card it sets face-down too (§10.5 step 5), so, as S #103 Vxyl's,
//! the Cry does nothing while the card is set, and runs once, at the reveal, which clears the stamp.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-123";

/// R783004, R783006: set by its Cloaked, its text is inert until it reveals.
fn is_set(ctx: &EffectContext<'_>) -> bool {
    ctx.self_.as_ref().is_some_and(|card| card.set_as.is_some())
}

fn gokugains() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            if is_set(ctx) {
                return vec![];
            }
            vec![degrade_secretly(json_as(json!({
                "scope": { "side": "enemy", "zones": ["hand"], "types": ["Unit"] },
                "random": 1,
                "times": param(&*ctx, "times"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = gokugains();
    // The same script: the Radiant face's 30 is its declared `times`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #123 Gokugains — SPEC §8.10 row 123, BUILD M12 row S 123 (R1845000, R1845001).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const FILLER: &str = "core-005"; // (1) Spell: a non-Unit hand card.

    fn board(radiant: bool, seed: &str, theirs: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(6), "mana": 10 },
            "p2": { "hand": theirs, "library": filler(6) },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s
    }

    fn hidden(s: &Scenario) -> Vec<(String, Option<i32>)> {
        s.hand(P2)
            .into_iter()
            .map(|card| (card.def_id, card.hidden_nerfs))
            .collect()
    }

    fn no_nerf_reported(s: &Scenario) -> bool {
        !s.events()
            .iter()
            .any(|event| matches!(event, GameEvent::Degraded { .. }))
    }

    fn no_view_shows_them(s: &Scenario) {
        for viewer in [P1, P2] {
            let view = serde_json::to_string(&s.view(viewer)).expect("a view");
            assert!(!view.contains("hiddenNerfs"), "{viewer:?}'s view");
        }
    }

    #[test]
    fn r1845000_its_cry_secretly_nerfs_the_one_unit_of_their_hand_three_times_said_to_nobody() {
        let mut s = board(false, "gokugains", json!([FILLER, VANILLA, FILLER]));
        s.play(ID, json!({ "zone": 1 }));
        assert_eq!(
            hidden(&s),
            vec![
                (FILLER.to_string(), None),
                (VANILLA.to_string(), Some(3)),
                (FILLER.to_string(), None),
            ]
        );
        assert!(no_nerf_reported(&s));
        no_view_shows_them(&s);
    }

    #[test]
    fn r1845000_one_random_unit_of_several_the_same_seed_the_same_one() {
        let run = || {
            let mut s = board(false, "gokugains-pick", json!([VANILLA, MENACE, VANILLA]));
            s.play(ID, json!({ "zone": 1 }));
            hidden(&s)
        };
        let marked = run();
        assert_eq!(marked.iter().filter(|(_, count)| count.is_some()).count(), 1);
        assert_eq!(run(), marked);
    }

    #[test]
    fn r1845000_no_unit_in_their_hand_nothing_and_no_draw() {
        let mut s = board(false, "gokugains-none", json!([FILLER, FILLER]));
        let cursor = s.state().rng_cursor;
        s.play(ID, json!({ "zone": 1 }));
        assert!(hidden(&s).iter().all(|(_, count)| count.is_none()));
        assert_eq!(s.state().rng_cursor, cursor, "R129");
    }

    #[test]
    fn times_reads_through_param() {
        let mut s = board(false, "gokugains-param", json!([VANILLA]));
        let times = crate::upgrade_number(&mut s, ID, "times");
        s.play(ID, json!({ "zone": 1 }));
        assert_eq!(hidden(&s), vec![(VANILLA.to_string(), Some(times))]);
    }

    #[test]
    fn r1845000_radiant_is_a_4_2_and_nerfs_thirty_times() {
        let mut s = board(true, "gokugains-radiant", json!([VANILLA]));
        s.play(ID, json!({ "zone": 1 }));
        assert_eq!(hidden(&s), vec![(VANILLA.to_string(), Some(30))]);
        let me = s.unit(P1, 1).expect("Gokugains").id;
        let stats = s.stats(me.as_str());
        assert_eq!((stats.attack, stats.health), (4, 2));
        no_view_shows_them(&s);
    }

    #[test]
    fn r783006_set_by_its_cloaked_the_cry_runs_as_it_is_revealed() {
        let mut s = board(false, "gokugains-cloaked", json!([VANILLA]));
        let id = s.hand(P1).into_iter().find(|card| card.def_id == ID).expect("in hand").id;
        s.play(&id, json!({ "zone": 1, "row": "backrow", "faceDown": "cloaked" }));
        assert_eq!(hidden(&s), vec![(VANILLA.to_string(), None)], "set, nothing yet");
        let set = s.backrow(P1, 1).expect("the set Gokugains").id;
        s.activate(&set, json!({ "ability": jackioh_engine::alt_play::CLOAKED_REVEAL }));
        assert_eq!(hidden(&s), vec![(VANILLA.to_string(), Some(3))]);
    }

    #[test]
    fn r1845001_played_the_nerfs_land_and_spare_its_cost() {
        let mut s = board(false, "gokugains-land", json!([MENACE, FILLER]));
        s.play(ID, json!({ "zone": 1 }));
        s.end_turn();
        let menace = s.hand(P2).into_iter().find(|card| card.def_id == MENACE).expect("in hand").id;
        s.state_mut().players.p2.mana.current = 10;
        s.play(menace.as_str(), json!({ "zone": 1 }));
        assert_eq!(s.state().players.p2.mana.current, 10 - 3, "its printed cost");
        assert_eq!(s.card(menace.as_str()).hidden_nerfs, None, "spent");
        let nerfs: Vec<&TuningChange> = s
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Degraded { instance_id, change, .. } if *instance_id == menace => Some(change),
                _ => None,
            })
            .collect();
        assert_eq!(nerfs.len(), 3, "seen as it is played");
        assert!(!nerfs.iter().any(|change| matches!(change, TuningChange::Cost { .. })));
    }
}
