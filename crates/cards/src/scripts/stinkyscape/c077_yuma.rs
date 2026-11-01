//! Stinkyscape #77 Yuma (SPEC §8.10 row 77). (2) Unit, Epic, 1/8 → 2/16, Flying.
//!   Base:    "Flying\nCry: At the start of your opponent's next turn, make them a trade offer: you give
//!            them a card from your hand, then they Discover one of theirs to give you."
//!   Radiant: "Flying, Lucky 1\nCry: …" (the same Cry)
//!
//! Flying is a printed keyword the engine plays; since #2521 it is no Starship Piece, so its death fuses
//! nothing (R1852000 reads the keyword). Cry (R1): `trade_offer`
//! (R1845002): a delayed effect due at the start of the opponent's next turn, a countdown badge on its
//! side, that comes whatever becomes of Yuma (R76); with a card in each hand its controller picks one of
//! theirs, it goes to the opponent's hand as theirs (R12), then the opponent Discovers one of their other
//! hand cards and it comes to its controller's; either hand empty, nothing is asked. R1877001: the offer
//! carries the Lucky Yuma has as it is made (§6.1, R1438), so the Radiant face's Lucky 1 draws the
//! Discover's options twice and keeps the set whose cheapest card costs most. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-077";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| vec![trade_offer()])),
        ..Script::default()
    };
    // The same script: the Radiant face's Lucky 1 is printed, and the offer reads it (R1877001).
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #77 Yuma — SPEC §8.10 row 77, BUILD M12 row S 77 (R1845002). The prompts' answerers (legal
// actions, the policy, the timeout, the AI) are the engine's (R1845002); the Lucky draw's comparator is
// proven on the engine's fixture (R1877001). No Starship Piece since #2521: its death fuses nothing.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // (1) Spell.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const FREEBIE: &str = "core-010"; // (0) Spell.
    const HIT_JOB: &str = "core-016"; // (3) Spell.
    const FLOOD: &str = "core-017"; // (4) Spell.

    fn offered(radiant: bool, seed: &str, mine: Value, theirs: Value) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(mine.as_array().cloned().unwrap_or_default());
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "field": [VANILLA], "library": filler(8), "mana": 10 },
            "p2": { "hand": theirs, "library": filler(8) },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s.play(ID, json!({ "zone": 2 }));
        s.end_turn();
        s
    }

    fn option_ids(s: &Scenario) -> Vec<String> {
        s.state()
            .pending
            .as_ref()
            .expect("a prompt is open")
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Instance { instance_id } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn held(s: &Scenario, player: PlayerId, def_id: &str) -> String {
        s.hand(player)
            .into_iter()
            .find(|card| card.def_id == def_id)
            .map(|card| card.id)
            .unwrap_or_else(|| panic!("{player:?} holds a {def_id}"))
    }

    fn pick(s: &mut Scenario, id: &str) {
        s.answer(json!(format!("instance:{id}")));
    }

    #[test]
    fn r1845002_its_cry_makes_a_trade_offer_you_give_then_they_discover_one_back() {
        let mut s = scenario(json!({
            "seed": "yuma-badge",
            "p1": { "hand": [ID, FILLER], "field": [VANILLA], "library": filler(8), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(8) },
        }));
        s.play(ID, json!({ "zone": 2 }));
        assert_eq!(s.state().delayed.len(), 1, "the offer waits");
        assert_eq!(s.state().delayed[0].resume.hook, TRADE_OFFER_HOOK);
        assert!(s.state().pending.is_none(), "nothing asked on p1's turn");

        let mut s = offered(false, "yuma", json!([MENACE, FILLER]), json!([VANILLA, HIT_JOB]));
        let pending = s.state().pending.clone().expect("p1 gives");
        assert_eq!((pending.player_id, pending.kind), (P1, PromptKind::Hand));
        let menace = held(&s, P1, MENACE);
        pick(&mut s, &menace);
        assert_eq!(s.card(menace.as_str()).zone, Zone::Hand { player: P2 });
        let pending = s.state().pending.clone().expect("p2 Discovers");
        assert_eq!((pending.player_id, pending.kind), (P2, PromptKind::Discover));
        assert!(!option_ids(&s).contains(&menace), "never the given card back");
        let hit_job = held(&s, P2, HIT_JOB);
        pick(&mut s, &hit_job);
        assert_eq!(s.card(hit_job.as_str()).zone, Zone::Hand { player: P1 });
        assert!(s.state().pending.is_none());
    }

    #[test]
    fn its_death_fuses_nothing_it_is_no_starship_piece() {
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB, FILLER], "field": [ID], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        let me = s.card(ID).clone();
        assert!(!has_keyword(&s.stats(&me).keywords, KeywordKind::StarshipPiece));
        // The testkit's `pick` named in full: this module's own `pick` answers prompts.
        s.play(HIT_JOB, json!({ "targets": jackioh_engine::testkit::pick::instance(&me) }));
        s.expect_in_zone(&me, "graveyard");
        assert!(dormant_starship_of(s.state(), P1).is_none(), "no Dormant Starship");
        assert!(!s.events().iter().any(|event| matches!(event, GameEvent::StarshipFused { .. })));
    }

    #[test]
    fn r1845002_either_hand_empty_nothing_is_asked() {
        let s = offered(false, "yuma-empty-mine", json!([]), json!([VANILLA]));
        assert!(s.state().pending.is_none());
        let s = offered(false, "yuma-empty-theirs", json!([FILLER]), json!([]));
        assert!(s.state().pending.is_none());
        assert_eq!(s.hand(P1).len(), 1);
    }

    #[test]
    fn r1877001_radiant_is_a_2_16_with_lucky_1_carried_into_the_offer() {
        let mut s = scenario(json!({
            "seed": "yuma-radiant",
            "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER], "field": [VANILLA], "library": filler(8), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(8) },
        }));
        s.play(ID, json!({ "zone": 2 }));
        let me = s.unit(P1, 2).expect("Yuma").id;
        let stats = s.stats(me.as_str());
        assert_eq!((stats.attack, stats.health), (2, 16));
        assert_eq!(lucky_on(s.state(), s.card(me.as_str())), 1);
        assert_eq!(
            s.state().delayed[0].resume.data.get(TRADE_OFFER_LUCKY_KEY),
            Some(&json!(1))
        );

        let s = offered(false, "yuma-base", json!([FILLER]), json!([VANILLA]));
        assert!(
            s.state().pending.is_some()
                && !s.state().delayed.iter().any(|offer| offer.resume.data.contains_key(TRADE_OFFER_LUCKY_KEY)),
            "the base face carries none"
        );
    }

    #[test]
    fn r1877001_radiant_offers_a_set_no_cheaper_than_the_base_face_on_the_same_seed() {
        let theirs = json!([FREEBIE, FILLER, VANILLA, HIT_JOB, MENACE, FLOOD, FREEBIE]);
        let cheapest = |radiant: bool, seed: &str| {
            let mut s = offered(radiant, seed, json!([FILLER]), theirs.clone());
            let filler = held(&s, P1, FILLER);
            pick(&mut s, &filler);
            assert_eq!(option_ids(&s).len(), DISCOVER_OPTIONS as usize);
            option_ids(&s)
                .iter()
                .map(|id| effective_cost(s.state(), s.card(id.as_str()), Default::default()))
                .min()
                .expect("options")
        };
        let mut dearer = 0;
        for n in 0..12 {
            let seed = format!("yuma-lucky-{n}");
            let (base, radiant) = (cheapest(false, &seed), cheapest(true, &seed));
            assert!(radiant >= base, "{seed}");
            if radiant > base {
                dearer += 1;
            }
        }
        assert!(dearer > 0);
    }
}
