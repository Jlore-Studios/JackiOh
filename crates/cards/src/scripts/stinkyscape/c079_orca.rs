//! Stinkyscape #79 Orca (SPEC §8.10 row 79). Unit 8/8 → 16/16, cost 4, Embiggened (5), Epic.
//!   Base:    "Starship Piece, Flying
//!            Cry: Choose one of 2 Miners and Equip it into this.
//!            Embiggened (5): The Miners offered are Radiant."
//!   Radiant: "… Cry: Choose one of 2 Miners and Equip it into this, twice. …"
//!
//! Miner v1 to S #66 Miner v5, named by id so that the pool reaches them whether or not the set ships (R1420):
//! the pick is a fresh copy fused straight into this (R1086, R77), its Activate with it (R1751000). The
//! Radiant face asks twice, the second offer once the first pick is in and without it. Embiggened
//! (R1175000), the offer is Radiant (R469), the Cry reading the play's stamp (R81); its (5) is past
//! `MAX_MANA`, so it is played so only with mana an effect gave (R1857001). Starship Piece and Flying are
//! catalog keywords the engine plays; it stays a piece, the kept card being one (R1852001).

use jackioh_engine::effects::equip_picks;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-079";

/// S #62 Miner v1 to S #66 Miner v5.
const MINERS: [&str; 5] = [
    "stinkyscape-062",
    "stinkyscape-063",
    "stinkyscape-064",
    "stinkyscape-065",
    "stinkyscape-066",
];

/// "Choose one of 2 Miners".
const OFFER: i32 = 2;

/// "twice", on the Radiant face.
const RADIANT_PICKS: i32 = 2;

/// R1872002: "Choose one of 2 Miners and Equip it into this[, twice]", Radiant ones Embiggened.
fn trawl(picks: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({ "query": { "defId": MINERS }, "options": OFFER, "times": picks });
            if ctx.embiggened {
                args["radiant"] = json!(true);
            }
            vec![equip_picks(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: trawl(1),
        radiant: trawl(RADIANT_PICKS),
    }
}

// Stinkyscape #79 Orca — SPEC §8.10 row 79, BUILD M12 row S 79 (R1841002).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::effects::EQUIP_PICK_STEP;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    fn played(radiant: bool, embiggened: bool) -> (Scenario, String) {
        let mut s = weapon_board(
            "s079",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 }),
            json!({}),
        );
        s.play(ID, json!({ "zone": 1, "embiggen": embiggened }));
        let host = s.unit_at(P1, 1).id;
        (s, host)
    }

    fn offered(s: &Scenario) -> Vec<String> {
        let pending = s.state().pending.clone().expect("a Discover is open");
        assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
        assert_eq!(pending.resume.step, EQUIP_PICK_STEP);
        pending
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Mode { option } => Some(option.clone()),
                _ => None,
            })
            .collect()
    }

    fn answers(s: &mut Scenario, host: &str, picks: usize, radiant_offer: bool) -> Vec<String> {
        let mut held: Vec<String> = Vec::new();
        for _ in 0..picks {
            let offer = offered(s);
            assert_eq!(offer.len(), OFFER as usize, "{offer:?}");
            assert!(offer.iter().all(|id| MINERS.contains(&id.as_str()) && !held.contains(id)), "{offer:?}");
            let before = s.stats(host).health;
            s.answer(json!(offer[0]));
            let def = crate::card_def(&offer[0]);
            let gained = if radiant_offer { def.radiant.health } else { def.base.health }.unwrap_or_default();
            assert_eq!(s.stats(host).health, before + gained, "{}", offer[0]);
            held.push(offer[0].clone());
        }
        assert!(s.state().pending.is_none());
        held
    }

    #[test]
    fn r1872002_its_cry_offers_two_miners_and_equips_the_one_chosen() {
        let (mut s, host) = played(false, false);
        answers(&mut s, &host, 1, false);
        assert!(unit_has(s.state(), s.card(host.as_str()), KeywordKind::StarshipPiece));
        s.activate(host.as_str(), json!({ "ability": "mine" }));
        assert_eq!(resources_in_hand_of(s.state(), P1).len(), 1);
    }

    #[test]
    fn r1872002_r1175000_embiggened_at_5_the_miners_offered_are_radiant() {
        let (mut s, host) = played(false, true);
        assert_eq!(s.state().players[P1].mana.current, 10 - 5);
        answers(&mut s, &host, 1, true);
    }

    #[test]
    fn radiant_16_16_and_two_offers_in_turn_the_second_without_the_first_pick() {
        let (mut s, host) = played(true, false);
        // The Radiant host wears its Miners' Radiant faces (R77).
        answers(&mut s, &host, 2, true);
        assert!(unit_has(s.state(), s.card(host.as_str()), KeywordKind::StarshipPiece));
    }
}
