//! S+ #80 The Last Voyage (SPEC §8.12 row S+ #80; §6.1 Dormant, Starship Piece and Starship, §6.3
//! Launch; R3539000–R3539002): (4) Spell, Terminus, Mythic.
//!   Base:    "Exile your hand, deck and board, Dormant cards included. Summon {starships|random
//!            Terminus Starship|random Terminus Starships}. They can't be launched by hand. Launch a
//!            random one at the start of each of your turns."
//!   Radiant: "Exile your hand, deck and board and your opponent's board, Dormant cards included.
//!            Summon … They can't be launched by hand. Launch a random one now and at the start of each
//!            of your turns."
//!
//! Engine: the board exile reaches Dormant cards (`exile_board`, R3539002) and spares only a card no
//! effect reaches; the ten Terminus Starships are summoned already holding their fixed loadouts
//! (`summon_starships`, R3539001; five worth 5 mana of Starship Pieces and five worth 6 since issue #3951,
//! 5 to 12 through v0.6.4c, which `script_v0_6_4c` keeps for those replays, R767000), sealed against a
//! launch by hand (R3539000); one launches at the start of each of its caster's turns from the next on
//! (`launch_random_starship`, kept by `for_rest_of_game`, R3539002), a free launch (R1852006), and on
//! the Radiant face one at once as well. The number summoned is the declared `starships`. Issue #4154
//! took the base face's launch at once out; `script_v0_6_6h` keeps it for the replays of v0.6.6h and
//! before (R767000).

use jackioh_engine::prelude::*;

use crate::scripts::stinkyscape_plus::t_terminus_starship_ark_of_ashes::{
    ID as ARK_ID, PIECES as ARK_PIECES,
};
use crate::scripts::stinkyscape_plus::t_terminus_starship_cold_exodus::{
    ID as EXODUS_ID, PIECES as EXODUS_PIECES,
};
use crate::scripts::stinkyscape_plus::t_terminus_starship_dying_ember::{
    ID as EMBER_ID, PIECES as EMBER_PIECES,
};
use crate::scripts::stinkyscape_plus::t_terminus_starship_entropys_wake::{
    ID as ENTROPY_ID, PIECES as ENTROPY_PIECES,
};
use crate::scripts::stinkyscape_plus::t_terminus_starship_farthest_shore::{
    ID as SHORE_ID, PIECES as SHORE_PIECES,
};
use crate::scripts::stinkyscape_plus::t_terminus_starship_heat_death_pilgrim::{
    ID as PILGRIM_ID, PIECES as PILGRIM_PIECES,
};
use crate::scripts::stinkyscape_plus::t_terminus_starship_nightfall_covenant::{
    ID as COVENANT_ID, PIECES as COVENANT_PIECES,
};
use crate::scripts::stinkyscape_plus::t_terminus_starship_omega_drift::{
    ID as DRIFT_ID, PIECES as DRIFT_PIECES,
};
use crate::scripts::stinkyscape_plus::t_terminus_starship_red_giant_requiem::{
    ID as REQUIEM_ID, PIECES as REQUIEM_PIECES,
};
use crate::scripts::stinkyscape_plus::t_terminus_starship_vacuum_psalm::{
    ID as PSALM_ID, PIECES as PSALM_PIECES,
};

pub const ID: &str = "stinkyscapeplus-080";

/// R3539001: the ten Terminus Starships, each hull and the fixed loadout it comes holding, five worth 5
/// mana of Starship Pieces and five worth 6 (issue #3951).
pub const TERMINUS_STARSHIPS: [(&str, &[&str]); 10] = [
    (ARK_ID, ARK_PIECES),
    (EMBER_ID, EMBER_PIECES),
    (PILGRIM_ID, PILGRIM_PIECES),
    (ENTROPY_ID, ENTROPY_PIECES),
    (REQUIEM_ID, REQUIEM_PIECES),
    (EXODUS_ID, EXODUS_PIECES),
    (DRIFT_ID, DRIFT_PIECES),
    (SHORE_ID, SHORE_PIECES),
    (COVENANT_ID, COVENANT_PIECES),
    (PSALM_ID, PSALM_PIECES),
];

/// R767000: the ten Terminus Starships through v0.6.4c, each with the loadout it came holding then, worth
/// 5 to 12 mana of Starship Pieces (issue #3951 cut eight of them to 5 or 6). `script_v0_6_4c` here and
/// in `c155_the_far_vagabond` summon from it.
pub const TERMINUS_STARSHIPS_V0_6_4C: [(&str, &[&str]); 10] = [
    (ARK_ID, ARK_PIECES),
    (EMBER_ID, EMBER_PIECES),
    (PILGRIM_ID, &["stinkyscape-108", "stinkyscape-107", "stinkyscape-043", "stinkyscape-122"]),
    (ENTROPY_ID, &["stinkyscape-105", "stinkyscape-057", "stinkyscape-038", "stinkyscape-098"]),
    (REQUIEM_ID, &["stinkyscape-060", "stinkyscape-108", "stinkyscape-133"]),
    (EXODUS_ID, &["stinkyscape-175", "stinkyscape-104", "stinkyscape-132", "stinkyscape-054"]),
    (DRIFT_ID, &["stinkyscape-058", "stinkyscape-106", "stinkyscape-136", "stinkyscape-107"]),
    (SHORE_ID, &["stinkyscape-059", "stinkyscape-090", "stinkyscape-093", "stinkyscape-133", "stinkyscape-043"]),
    (COVENANT_ID, &["stinkyscape-061", "stinkyscape-056", "stinkyscape-122", "stinkyscape-038", "stinkyscape-049"]),
    (PSALM_ID, &["stinkyscape-060", "stinkyscape-058", "stinkyscape-061"]),
];

/// The blueprints `summon_starships` picks from.
pub fn blueprints() -> Vec<StarshipBlueprint> {
    blueprints_of(&TERMINUS_STARSHIPS)
}

/// The blueprints of a table of hulls and loadouts.
pub fn blueprints_of(table: &[(&str, &[&str])]) -> Vec<StarshipBlueprint> {
    table
        .iter()
        .map(|(id, pieces)| StarshipBlueprint {
            def_id: (*id).to_string(),
            pieces: pieces.iter().map(|piece| (*piece).to_string()).collect(),
        })
        .collect()
}

/// The hulls `launch_random_starship` picks from.
fn hull_ids() -> Vec<String> {
    TERMINUS_STARSHIPS
        .iter()
        .map(|(id, _)| (*id).to_string())
        .collect()
}

/// R3539000: both faces of a Terminus Starship print Dormant, the hull and the seal: it can't be
/// launched by hand.
pub fn terminus_hull() -> CardScripts {
    let face = Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText::default()),
            starship_hull: Some(true),
            no_hand_launch: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    CardScripts {
        base: face.clone(),
        radiant: face,
    }
}

/// The badge its caster carries for the rest of the game.
const BADGE: &str = "Start of turn: Launch a random Terminus Starship.";

/// One face: `radiant` also exiles the opponent's board, and `launch_now` launches one at once (the
/// Radiant face's, and the base face's through v0.6.6h).
fn voyage(radiant: bool, launch_now: bool, blueprints: fn() -> Vec<StarshipBlueprint>) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let starships = param(ctx, "starships");
            let mut effects = vec![
                exile_hand(json_as(json!({}))),
                exile_bottom_of_library(json_as(json!({
                    "count": zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Library).len(),
                }))),
                exile_board(json_as(json!({}))),
            ];
            if radiant {
                effects.push(exile_board(json_as(json!({ "player": "enemy" }))));
            }
            effects.push(summon_starships(json_as(json!({
                "blueprints": blueprints(),
                "count": starships,
            }))));
            if launch_now {
                effects.push(launch_random_starship(json_as(json!({
                    "hulls": hull_ids(),
                }))));
            }
            effects.push(for_rest_of_game(json_as(json!({
                "step": "launch",
                "label": BADGE,
            }))));
            effects
        })),
        delayed: Some(hook(|_ctx| {
            vec![launch_random_starship(json_as(json!({
                "hulls": hull_ids(),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: voyage(false, false, blueprints),
        radiant: voyage(true, true, blueprints),
    }
}

/// R767000: after v0.6.4c through v0.6.6h the base face launched one at once too (issue #4154).
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_6h() -> CardScripts {
    CardScripts {
        base: voyage(false, true, blueprints),
        radiant: voyage(true, true, blueprints),
    }
}

/// R767000: the loadouts of v0.6.4c and before (issue #3951).
pub fn blueprints_v0_6_4c() -> Vec<StarshipBlueprint> {
    blueprints_of(&TERMINUS_STARSHIPS_V0_6_4C)
}

/// R767000: the scripts through v0.6.4c, summoning the loadouts of then (issue #3951). `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_4c() -> CardScripts {
    CardScripts {
        base: voyage(false, true, blueprints_v0_6_4c),
        radiant: voyage(true, true, blueprints_v0_6_4c),
    }
}

// S+ #80 The Last Voyage — SPEC §8.12 row S+ #80 (R3539000–R3539002).
/// R3539001: the shared proof of the ten Terminus Starships, run from each token's file: a ship
/// summoned holding `pieces` is Dormant, sealed against a launch by hand, and Radiant holding
/// Radiant pieces on its Radiant face.
#[cfg(test)]
pub(crate) fn assert_terminus(id: &str, pieces: &[&str], radiant: bool) {
    use jackioh_engine::effects::{StarshipBlueprint, SummonStarshipsArgs, summon_starships};
    use jackioh_engine::testkit::*;

    let mut s = crate::scenario(json!({ "seed": "terminus", "p1": {}, "p2": {} }));
    crate::run_as(
        &mut s,
        summon_starships(SummonStarshipsArgs {
            blueprints: vec![StarshipBlueprint {
                def_id: id.to_string(),
                pieces: pieces.iter().map(|piece| (*piece).to_string()).collect(),
            }],
            count: 1,
            player: None,
            radiant: if radiant { Some(true) } else { None },
            price: None,
        }),
        PlayerId::P1,
    );
    let ship = dormant_starship_of(s.state(), PlayerId::P1)
        .cloned()
        .expect("a Terminus Starship");
    assert!(is_dormant_starship(&ship));
    assert_eq!(hull_def_of(s.state(), &ship.def_id), id);
    let records = starship_pieces_of(&ship);
    let held: Vec<String> = records.iter().map(|record| record.def_id.clone()).collect();
    assert_eq!(held, pieces);
    for record in &records {
        assert_eq!(record.radiant, if radiant { Some(true) } else { None });
    }
    assert!(
        !legal_actions(s.state(), PlayerId::P1).iter().any(|action| {
            matches!(action, ActionBody::Activate { instance_id, .. } if instance_id == &ship.id)
        }),
        "no launch by hand"
    );
}

#[cfg(test)]
mod tests {
    use super::{ID, TERMINUS_STARSHIPS, blueprints, script_v0_6_6h};
    use crate::built_starship;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VOYAGE: &str = ID;
    const FILLER: &str = "core-005"; // Stockpile: a Spell
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const AP_ROUNDS: &str = "stinkyscape-100"; // AP Rounds: a Starship Piece

    /// Every ship on p1's side of the field.
    fn ships(s: &Scenario) -> Vec<CardInstance> {
        let side = &s.state().players.p1;
        side.units
            .iter()
            .flatten()
            .flatten()
            .chain(side.backrow.iter().flatten())
            .cloned()
            .collect()
    }

    fn launched_count(s: &Scenario) -> usize {
        ships(s).iter().filter(|card| is_launched_starship(card)).count()
    }

    /// p1's Dormant Starship built from one AP Rounds, holding Voyage and a filler, against p2's
    /// vanilla and a Dormant vanilla; p1 to play, turns never ending by themselves.
    fn voyaging(radiant: bool) -> (Scenario, String, String, Vec<String>, String, String, String) {
        let hand = if radiant {
            json!([{ "def": VOYAGE, "radiant": true }, FILLER])
        } else {
            json!([VOYAGE, FILLER])
        };
        let (s, ship) = built_starship(
            "voyage",
            json!([{ "def": AP_ROUNDS, "lane": 2 }]),
            json!({ "hand": hand, "mana": 10 }),
            json!({ "field": [VANILLA, { "def": VANILLA, "lane": 2, "slumber": { "turns": 2 } }] }),
        );
        let voyage_id = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == VOYAGE)
            .expect("Voyage in hand")
            .id;
        let filler_id = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == FILLER)
            .expect("a filler in hand")
            .id;
        let library_ids: Vec<String> = s
            .pile(P1, "library")
            .into_iter()
            .map(|card| card.id)
            .collect();
        let vanilla_id = s.unit(P2, 1).expect("p2's vanilla fills lane 1").id;
        let dormant_id = s.unit(P2, 2).expect("p2's Dormant vanilla fills lane 2").id;
        (s, voyage_id, filler_id, library_ids, ship, vanilla_id, dormant_id)
    }

    #[test]
    fn r3539001_each_loadout_is_5_to_12_mana_of_shipped_starship_pieces() {
        const TERMINUS_MIN_MANA: i32 = 5;
        const TERMINUS_MAX_MANA: i32 = 12;
        for (id, pieces) in TERMINUS_STARSHIPS {
            let total: i32 = pieces
                .iter()
                .map(|piece| query_cost(&crate::card_def(piece)))
                .sum();
            assert!(
                (TERMINUS_MIN_MANA..=TERMINUS_MAX_MANA).contains(&total),
                "{id} holds {total} mana of pieces"
            );
            for piece in pieces {
                let def = crate::card_def(piece);
                assert_eq!(def.set, SetName::Stinkyscape, "{piece} is Stinkyscape's");
                assert!(!def.token, "{piece} is no token");
                assert!(
                    has_keyword(&def.base.keywords, KeywordKind::StarshipPiece),
                    "{piece} prints Starship Piece"
                );
            }
        }
    }

    #[test]
    fn r3539001_the_ten_are_ten_different_starships() {
        let (mut s, _, _, _, _, _, _) = voyaging(false);
        s.play(VOYAGE, json!({}));
        let mut defs: Vec<String> = ships(&s)
            .into_iter()
            .map(|card| hull_def_of(s.state(), &card.def_id))
            .collect();
        defs.sort();
        let mut hulls: Vec<String> = blueprints()
            .into_iter()
            .map(|blueprint| blueprint.def_id)
            .collect();
        hulls.sort();
        assert_eq!(defs, hulls);
    }

    /// Issue #4154: the base face launches none at once.
    #[test]
    fn r3539002_exiles_hand_deck_and_board_dormant_included_summons_ten_and_launches_none_now() {
        let (mut s, voyage_id, filler_id, library_ids, ship, vanilla_id, _) = voyaging(false);
        s.play(VOYAGE, json!({}));
        assert!(s.hand(P1).is_empty(), "the hand is exiled");
        assert!(s.pile(P1, "library").is_empty(), "the deck is exiled");
        let exile: Vec<String> = s
            .pile(P1, "exile")
            .into_iter()
            .map(|card| card.id)
            .collect();
        assert!(exile.contains(&filler_id), "the old hand reaches exile");
        for id in &library_ids {
            assert!(exile.contains(id), "the old deck reaches exile");
        }
        assert!(exile.contains(&ship), "the old Dormant board reaches exile");
        let graves: Vec<String> = s
            .pile(P1, "graveyard")
            .into_iter()
            .map(|card| card.id)
            .collect();
        assert!(graves.contains(&voyage_id), "the Spell itself resolves to its graveyard");
        assert_eq!(ships(&s).len(), 10, "ten Starships fill 5 backrow and 5 unit zones");
        assert_eq!(launched_count(&s), 0, "none launches now");
        assert_eq!(dormant_starships_of(s.state(), P1).len(), 10);
        assert_eq!(
            s.unit(P2, 1).map(|unit| unit.id),
            Some(vanilla_id),
            "p2's board is untouched"
        );
    }

    #[test]
    fn r3539002_launches_one_at_each_start_of_its_casters_turn_only() {
        let (mut s, _, _, _, _, _, _) = voyaging(false);
        s.play(VOYAGE, json!({}));
        assert_eq!(launched_count(&s), 0);
        s.end_turn();
        assert_eq!(launched_count(&s), 0, "no launch at the opponent's start");
        s.end_turn();
        assert_eq!(launched_count(&s), 1, "one at its caster's next start");
        s.end_turn();
        s.end_turn();
        assert_eq!(launched_count(&s), 2, "and another at the one after");
    }

    /// R767000: through v0.6.6h the base face launched one at once too (issue #4154).
    #[test]
    fn r767000_the_script_kept_for_v0_6_6h_launches_one_now_on_the_base_face() {
        crate::under_kept(ID, script_v0_6_6h(), || {
            let (mut s, _, _, _, _, _, _) = voyaging(false);
            s.play(VOYAGE, json!({}));
            assert_eq!(launched_count(&s), 1, "one launches now");
            assert_eq!(dormant_starships_of(s.state(), P1).len(), 9);
            s.end_turn().end_turn();
            assert_eq!(launched_count(&s), 2, "another at its caster's next start");
        });
    }

    /// The Radiant face still launches one at once.
    #[test]
    fn radiant_also_exiles_the_opponents_board_dormant_included() {
        let (mut s, _, _, _, _, vanilla_id, dormant_id) = voyaging(true);
        s.play(VOYAGE, json!({}));
        assert!(s.unit(P2, 1).is_none(), "p2's board is exiled too");
        assert!(s.unit(P2, 2).is_none(), "p2's Dormant card with it");
        let exile: Vec<String> = s
            .pile(P2, "exile")
            .into_iter()
            .map(|card| card.id)
            .collect();
        assert!(exile.contains(&vanilla_id));
        assert!(exile.contains(&dormant_id));
        assert_eq!(ships(&s).len(), 10);
        assert_eq!(launched_count(&s), 1, "one launches now");
        s.end_turn().end_turn();
        assert_eq!(launched_count(&s), 2, "another at its caster's next start");
    }
}
