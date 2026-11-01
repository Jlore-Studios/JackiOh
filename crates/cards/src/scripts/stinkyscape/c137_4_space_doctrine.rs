//! S #137.4 Space Doctrine (SPEC §8.10 row 137.4; §2.4, R65, R824000, R1852005, R1859007, R1859011):
//! (4) Spell, War Doctrine, Token (printed Legendary), a token of the Stinkyscape set,
//! which #137 Starfleet Commander Discovers and casts.
//!
//! Base:    "For the rest of the game, your Units have Flying. Fill your hand with copies of the
//!          Starship Pieces fused into your Starship."
//! Radiant: "For the rest of the game, your Units have Flying. Fill your hand with copies of the
//!          Starship Pieces fused into your Starship. They cost ({setCost})."
//! Engine: `give_units_keyword { Flying }`, a `unitsHaveKeyword` player modifier for the rest of the
//! game: every Unit its caster controls has Flying, those that arrive later too, but a Vanilla one
//! (R1859007). Then, once, as it is cast: one copy of each piece its caster's launched Starship
//! remembers (`starship_pieces_of`, R1852005), each on the face it fused on, oldest first, added until
//! the hand is full (`hand_cap_of`), none burned and none repeated (§2.4, R1859011); no launched
//! Starship, nothing. Radiant: each copy's `costOverride` setCost (R65). Tunes: setCost 1 ↓, the
//! Radiant face only (R824000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-137-4";

fn space_doctrine(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![give_units_keyword(GiveUnitsKeywordArgs {
                keyword: Keyword::Flying,
            })];
            let state: &GameState = &*ctx.state;
            let Some(ship) = launched_starship_of(state, ctx.controller) else {
                return effects;
            };
            let room = (hand_cap_of(state, ctx.controller) - zone_count(state, ctx.controller, OffFieldZone::Hand)).max(0);
            let room = usize::try_from(room).unwrap_or(0);
            let set_cost = radiant.then(|| param(&*ctx, "setCost"));
            for piece in starship_pieces_of(ship).into_iter().take(room) {
                let mut copy = json!({ "defId": piece.def_id, "radiant": piece.radiant == Some(true) });
                if let Some(cost) = set_cost {
                    copy["costOverride"] = json!(cost);
                }
                effects.push(add_to_hand(json_as(copy)));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: space_doctrine(false),
        radiant: space_doctrine(true),
    }
}

// S #137.4 Space Doctrine — SPEC §8.10 row 137.4, BUILD M12 row S 137.4: "after it, every Unit its caster
// controls has Flying, later ones included; once, copies of the pieces its launched Starship holds go to
// the hand, oldest first, until it is full; no Starship, nothing; radiant each copy costs (1), setCost
// read through `param()` (R824000)".
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{built_starship, def_ids, launch_starship};
    use jackioh_engine::prelude::{HAND_CAP, play_cost};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const AP_ROUNDS: &str = "stinkyscape-100"; // (1) 3/1 Starship Piece, Equip, Pierce: keywords only.
    const CHASSIS: &str = "stinkyscape-048"; // (1) 4/1 Starship Piece, Equip, Rush: keywords only.
    const BURNER: &str = "stinkyscape-090"; // (2) 4/8 Starship Piece.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    /// p1's launched Starship holding an AP Rounds, a Radiant After Burner and an Ultralight Chassis, in
    /// that order; p1's hand the Doctrine (Radiant if `radiant`) and `fillers` fillers, not yet cast.
    fn board(seed: &str, radiant: bool, fillers: usize, launch: bool) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(vec![json!(FILLER); fillers]);
        let (mut s, ship) = built_starship(
            seed,
            json!([
                { "def": AP_ROUNDS, "lane": 1 },
                { "def": BURNER, "lane": 2, "radiant": true },
                { "def": CHASSIS, "lane": 3 },
            ]),
            json!({ "hand": hand, "field": [{ "def": VANILLA, "lane": 4 }] }),
            json!({}),
        );
        if launch {
            launch_starship(&mut s, &ship);
        }
        s.state_mut().players.p1.mana.current = 10;
        s
    }

    /// `board`, then the Doctrine cast; the hand after it.
    fn cast(seed: &str, radiant: bool, fillers: usize, launch: bool) -> (Scenario, Vec<CardInstance>) {
        let mut s = board(seed, radiant, fillers, launch);
        s.play(ID, json!({}));
        let hand = s.state().players.p1.hand.clone();
        (s, hand)
    }

    #[test]
    fn r1859011_one_copy_of_each_piece_oldest_first_on_the_face_it_fused_on() {
        let (s, hand) = cast("r1859011-space", false, 1, true);
        assert_eq!(def_ids(&hand), vec![FILLER, AP_ROUNDS, BURNER, CHASSIS]);
        assert_eq!(hand.iter().map(|card| card.radiant).collect::<Vec<_>>(), vec![false, false, true, false]);
        assert_eq!(play_cost(s.state(), &hand[1]), 1, "the printed cost on the base face");
        let flies = s
            .stats(s.unit_at(P1, 4).id.as_str())
            .keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Flying);
        assert!(flies);
    }

    #[test]
    fn r1859011_it_fills_the_hand_to_its_cap_and_burns_none() {
        let fillers = usize::try_from(HAND_CAP).expect("a small cap") - 2;
        let (s, hand) = cast("r1859011-space-full", false, fillers, true);
        assert_eq!(hand.len(), fillers + 2);
        assert_eq!(def_ids(&hand[fillers..]), vec![AP_ROUNDS, BURNER]);
        assert!(
            !s.last_events()
                .iter()
                .any(|event| event.event_type() == GameEventType::Burned)
        );
    }

    #[test]
    fn r1859011_radiant_each_copy_costs_1() {
        let (s, hand) = cast("r1859011-space-radiant", true, 0, true);
        assert_eq!(def_ids(&hand), vec![AP_ROUNDS, BURNER, CHASSIS]);
        assert!(hand.iter().all(|card| play_cost(s.state(), card) == 1));
        assert_eq!(hand[1].cost_override, Some(1), "Burner's (2) set to (1)");
    }

    #[test]
    fn set_cost_reads_through_param() {
        let mut s = board("space-param", true, 0, true);
        let set_cost = crate::upgrade_number(&mut s, ID, "setCost");
        s.play(ID, json!({}));
        let hand = s.state().players.p1.hand.clone();
        assert_eq!(def_ids(&hand), vec![AP_ROUNDS, BURNER, CHASSIS]);
        assert!(hand.iter().all(|card| card.cost_override == Some(set_cost)));
        assert!(hand.iter().all(|card| play_cost(s.state(), card) == set_cost));
    }

    #[test]
    fn r1859007_with_no_launched_starship_only_the_keyword() {
        let (_s, hand) = cast("r1859007-space-none", false, 1, false);
        assert_eq!(def_ids(&hand), vec![FILLER]);
    }
}
