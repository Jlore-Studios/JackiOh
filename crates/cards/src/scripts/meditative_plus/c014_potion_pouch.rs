//! M+ #14 Potion Pouch (SPEC §8.9 row 14, BUILD M11 row M+ 14): (1) Spell, Rare.
//!   Base:    "Add {cards|random Potion card|random Potion cards} to your hand." (2)
//!   Radiant: "Add {cards|random Radiant Potion card|random Radiant Potion cards} to your hand."
//!            (2)
//!
//! `add_random_from_catalog { query: { tags: [Potion] }, count }`: two picks from the Potion
//! pool — the non-token Potion cards of the sets that ship (R380, R1179003) — repeats allowed
//! (R60); a crafted potion is transient and in no pool. The Radiant face adds them Radiant. The
//! count is declared and read through `param` (R386). Tunes: cards 2 ↑.

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-014";

fn pouch(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Potion"] },
                "count": param(&*ctx, "cards"),
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: pouch(false),
        radiant: pouch(true),
    }
}

// M+ #14 Potion Pouch — SPEC §8.9 row 14, BUILD M11 row M+ 14: "adds 2 random Potion cards (M+
// 11, 12 or 13, previewed, R1178000), repeats seen over the seeds; a full hand burns the second;
// never a crafted potion or a non-Potion card; cards reads through `param()`; radiant both
// Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const POUCH: &str = "meditativeplus-014";
    /// The Potion pool under a preview of the set (R1179003): M+ #11 to #13, nothing else.
    const POTIONS: [&str; 3] = ["meditativeplus-011", "meditativeplus-012", "meditativeplus-013"];
    const FILLER: &str = "core-005"; // (1) Spell

    use crate::scenario;

    /// The def ids Pouch added to p1's hand, beside the Filler held throughout.
    fn added(s: &Scenario) -> Vec<String> {
        s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).map(|card| card.def_id).collect()
    }

    fn pouched(seed: &str, radiant: bool) -> Scenario {
        let _preview = preview_sets(&[SetName::MeditativePlus]);
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": POUCH, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(POUCH, json!({}));
        s
    }

    #[test]
    fn r1179003_adds_2_random_potion_cards() {
        crate::register_all();
        let s = pouched("m+014-two", false);
        let added = added(&s);
        assert_eq!(added.len(), 2);
        for def in &added {
            assert!(POTIONS.contains(&def.as_str()), "{def} is no Potion card");
        }
        for card in s.hand(P1) {
            if card.def_id != FILLER {
                assert!(!card.radiant);
            }
        }
    }

    #[test]
    fn r1179003_repeats_are_allowed() {
        crate::register_all();
        let seed = seed_where("m+014-repeat", |seed| {
            let _preview = preview_sets(&[SetName::MeditativePlus]);
            let mut s = scenario(json!({
                "seed": seed,
                "p1": { "hand": [POUCH, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(POUCH, json!({}));
            let added = added(&s);
            added.len() == 2 && added[0] == added[1]
        });
        let s = pouched(&seed, false);
        let added = added(&s);
        assert_eq!(added.len(), 2);
        assert_eq!(added[0], added[1]);
    }

    #[test]
    fn r1179003_nothing_while_the_set_is_held() {
        crate::register_all();
        // No preview: the Potion pool is empty (R1179003), so the Pouch fizzles.
        let mut s = scenario(json!({
            "seed": "m+014-held",
            "p1": { "hand": [POUCH, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(POUCH, json!({}));
        assert_eq!(added(&s), Vec::<String>::new());
    }

    #[test]
    fn s2_4_a_full_hand_burns_the_second() {
        crate::register_all();
        // The Pouch and nine more: after the Pouch is played the hand holds nine, so the first
        // potion fills it and the second burns.
        let mut hand = vec![json!(POUCH)];
        hand.extend((0..HAND_CAP - 1).map(|_| json!(FILLER)));
        let _preview = preview_sets(&[SetName::MeditativePlus]);
        let mut s = scenario(json!({
            "seed": "m+014-burn",
            "p1": { "hand": hand },
            "p2": { "hand": [FILLER] },
        }));
        s.play(POUCH, json!({}));
        assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
        let burned: Vec<CardInstance> =
            s.pile(P1, "graveyard").into_iter().filter(|card| POTIONS.contains(&card.def_id.as_str())).collect();
        assert_eq!(burned.len(), 1);
    }

    #[test]
    fn r386_cards_reads_through_param() {
        crate::register_all();
        let _preview = preview_sets(&[SetName::MeditativePlus]);
        let mut s = scenario(json!({
            "seed": "m+014-param",
            "p1": { "hand": [POUCH, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        step_param(s.card_mut(POUCH), "cards", 1);
        s.play(POUCH, json!({}));
        assert_eq!(added(&s).len(), 3);
    }

    #[test]
    fn radiant_both_are_radiant() {
        crate::register_all();
        let s = pouched("m+014-radiant", true);
        let added = added(&s);
        assert_eq!(added.len(), 2);
        for def in &added {
            assert!(POTIONS.contains(&def.as_str()), "{def} is no Potion card");
        }
        for card in s.hand(P1) {
            if card.def_id != FILLER {
                assert!(card.radiant, "the added potion is Radiant");
            }
        }
    }
}
