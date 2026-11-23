//! SPEC §8.12 S+ #133 Great Paperclip Theory III (§6.3 Fuse; R169, R275, R1420, R1863000, R3535000, R3565002):
//! (4) Spell, Mindustrial, Paperclip, Mythic, a card of the Stinkyscape+ set.
//!   Base:    "Add a Daglare to your hand. The first time you play each Automa-ζ cost card, add the
//!            subsequent cost Automa-ζ to your hand."
//!   Radiant: "Add a Radiant Dagger and Flare to your hand. The first time you play each Radiant Automa-ζ
//!            cost card, add the subsequent cost Radiant Automa-ζ to your hand."
//!
//! Engine: adds the first Automa-ζ card, Daglare (S+ #133.1, the fusion of Dagger and Flare), and sets one
//! `watch_first_plays` on its caster over the rungs (0) to (3): Daglare, Macizon, Fortnith and Sceptumbra
//! (S+ #133.1 to #133.5, one for each cost (0) to (4); R3565002). The first time they play each of them, the
//! next one up is added to their hand (R1863000). The Radiant face's "Radiant Dagger and Flare" is the
//! Radiant fusion of the two, a Radiant Daglare (R3565002), and every rung it adds is Radiant. The watch's
//! badge is the face's text (R169). The "Radiant … cost card" does not narrow which plays count (R3565002,
//! rejecting a watch only a Radiant play answers: R275).

use jackioh_engine::effects::{WatchFirstPlaysArgs, watch_first_plays};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-133";

/// Daglare to Reignclipse (S+ #133.1 to #133.5), by cost (0) to (4).
const ZETA: [&str; 5] = [
    "stinkyscapeplus-133-1",
    "stinkyscapeplus-133-2",
    "stinkyscapeplus-133-3",
    "stinkyscapeplus-133-4",
    "stinkyscapeplus-133-5",
];

/// The step the watcher re-enters for each rung's first resolved play.
const NEXT_RUNG: &str = "next-rung";

/// R1863000: the rung after the one the watcher answered (`defId`), Radiant on `radiant`; none after (4).
fn next_rung(ctx: &EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let Some(played) = ctx.data.get("defId").and_then(Value::as_str) else {
        return vec![];
    };
    let Some(next) = ZETA
        .iter()
        .position(|rung| *rung == played)
        .and_then(|at| ZETA.get(at + 1))
    else {
        return vec![];
    };
    vec![add_to_hand(json_as(json!({ "defId": next, "radiant": radiant })))]
}

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let kind = if radiant { FaceKind::Radiant } else { FaceKind::Base };
            let label = fill_params(def_of(Some(&*ctx.state), ID), kind, None);
            vec![
                add_to_hand(json_as(json!({ "defId": ZETA[0], "radiant": radiant }))),
                watch_first_plays(WatchFirstPlaysArgs {
                    // The last rung adds none, so the watch names the rungs before it.
                    def_ids: ZETA[..ZETA.len() - 1].iter().map(|rung| rung.to_string()).collect(),
                    step: NEXT_RUNG.to_string(),
                    label,
                    hook: None,
                    data: None,
                }),
            ]
        })),
        resume: IndexMap::from([(NEXT_RUNG, hook(move |ctx| next_rung(ctx, radiant)))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S+ #133 Great Paperclip Theory III — SPEC §8.12 row S+ 133 (R3565002; R169, R1863000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn setup(hand: Value) -> Scenario {
        let mut cards = hand.as_array().cloned().unwrap_or_default();
        cards.extend([json!(FILLER), json!(FILLER), json!(FILLER)]);
        scenario(json!({
            "seed": "s133",
            "p1": { "hand": cards, "library": filler(6), "mana": 40 },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }))
    }

    fn held(s: &Scenario, def_id: &str) -> Vec<bool> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id == def_id)
            .map(|card| card.radiant)
            .collect()
    }

    /// The Automa-ζ cards in p1's hand.
    fn zetas_held(s: &Scenario) -> usize {
        s.hand(P1).iter().filter(|card| ZETA.contains(&card.def_id.as_str())).count()
    }

    fn watched(s: &Scenario) -> Vec<Vec<String>> {
        s.state().players.p1.mods
            .iter()
            .filter_map(|modifier| match &modifier.kind {
                ModifierKind::PlayWatch { def_ids, .. } => Some(def_ids.clone()),
                _ => None,
            })
            .collect()
    }

    fn rung_ids(rungs: &[&str]) -> Vec<String> {
        rungs.iter().map(|rung| rung.to_string()).collect()
    }

    #[test]
    fn r3565002_the_ladder_is_the_cost_ordered_zeta_cards() {
        crate::register_all();
        for (cost, id) in ZETA.iter().enumerate() {
            let def = crate::card_def(id);
            assert!(def.tags.contains(&Tag::AutomaZeta), "{id}");
            assert_eq!(def.cost, CardCost::Fixed(cost as i32), "{id}");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r3565002_adds_a_daglare_and_watches_daglare_to_sceptumbra() {
            let mut s = setup(json!([ID]));
            s.play(ID, json!({}));
            assert_eq!(held(&s, ZETA[0]), vec![false]);
            assert_eq!(zetas_held(&s), 1);
            assert_eq!(watched(&s), vec![rung_ids(&ZETA[..4])]);
        }

        #[test]
        fn r3565002_each_first_play_adds_the_next_zeta() {
            let mut s = setup(json!([ID]));
            s.play(ID, json!({}));
            s.play(ZETA[0], json!({ "zone": 1 }));
            assert_eq!(held(&s, ZETA[1]), vec![false], "Daglare added Macizon");
            assert_eq!(zetas_held(&s), 1);
            s.play(ZETA[1], json!({ "zone": 2 }));
            assert_eq!(held(&s, ZETA[2]), vec![false], "Macizon added Fortnith");
            assert_eq!(zetas_held(&s), 1);
            assert_eq!(watched(&s), vec![rung_ids(&ZETA[2..4])]);
        }

        #[test]
        fn r3565002_a_second_play_adds_nothing() {
            let mut s = setup(json!([ID]));
            s.play(ID, json!({}));
            s.play(ZETA[0], json!({ "zone": 1 }));
            crate::run_as(&mut s, jackioh_engine::effects::add_to_hand(json_as(json!({ "defId": ZETA[0] }))), P1);
            s.play(ZETA[0], json!({ "zone": 2 }));
            assert_eq!(held(&s, ZETA[1]), vec![false], "still the one");
        }

        #[test]
        fn r3565002_r169_the_badge_is_the_face_text() {
            let mut s = setup(json!([ID]));
            s.play(ID, json!({}));
            let text = crate::card_def(ID).base.text;
            assert_eq!(s.view(P1).you.modifiers[0].label, text);
            assert_eq!(s.view(PlayerId::P2).opponent.modifiers[0].label, text);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3565002_radiant_adds_a_radiant_daglare_and_radiant_rungs() {
            let mut s = setup(json!([{ "def": ID, "radiant": true }]));
            s.play(ID, json!({}));
            assert_eq!(held(&s, ZETA[0]), vec![true]);
            assert_eq!(zetas_held(&s), 1, "one Radiant Daglare, the fusion of Dagger and Flare");
            assert_eq!(watched(&s), vec![rung_ids(&ZETA[..4])]);
            s.play(ZETA[0], json!({ "zone": 1 }));
            assert_eq!(held(&s, ZETA[1]), vec![true]);
            s.play(ZETA[1], json!({ "zone": 2 }));
            assert_eq!(held(&s, ZETA[2]), vec![true]);
            let text = crate::card_def(ID).radiant.text;
            assert_eq!(s.view(P1).you.modifiers[0].label, text);
        }
    }
}
