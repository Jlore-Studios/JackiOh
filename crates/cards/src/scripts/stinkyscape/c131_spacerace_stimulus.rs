//! S #131 Spacerace Stimulus (SPEC §8.10 row 131; R17, R64, R383, R831007, R1852000, R1852003,
//! R1859003): (1) Field Trap, Common, 6/6 → 12/12 (its unit face), Animated, Starship Piece, a card of
//! the Stinkyscape set.
//!
//! Base:    "Animated, Starship Piece\nReveals when either player launches a Starship."
//! Radiant: "Animated, Starship Piece\nReveals when either player launches a Starship or plays a
//!          Starship Piece."
//! Engine: set face-down in its backrow zone, it answers `starshipLaunched` of either player's Starship
//! (R1852003) and, on the Radiant face, `cardPlayed` of a card that is a Starship Piece as it is played
//! (`is_starship_piece`), either player's, its own play left out; a card set face-down has no text in
//! force, so it is no piece played (R1859003) and its play reveals nothing (CLAUDE.md rule 7). It fires

use jackioh_engine::effects::{animate, remember};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-131";

/// What it keeps once it has revealed: it reveals once (R831007).
const REVEALED: &str = "revealed";

/// Whether `event` reveals it: a Starship's launch, either player's, or on the Radiant face a card
/// played that is a Starship Piece as it is played, not this one — while it stands in its backrow zone
/// and has not revealed yet.
fn answers(ctx: &EffectContext<'_>, event: &GameEvent, radiant: bool) -> bool {
    let Some(this) = ctx.live_self() else {
        return false;
    };
    if !matches!(this.zone, Zone::Field { row: Row::Backrow, .. })
        || recalled(ctx, REVEALED) == Some(Value::Bool(true))
    {
        return false;
    }
    match event {
        GameEvent::StarshipLaunched { .. } => true,
        GameEvent::CardPlayed { instance_id, .. } => {
            let state: &GameState = &*ctx.state;
            radiant
                && *instance_id != this.id
                && find_instance(state, instance_id).is_some_and(|card| is_starship_piece(state, card))
        }
        _ => false,
    }
}

fn stimulus(radiant: bool) -> Script {
    let on: &[GameEventType] = if radiant {
        &[GameEventType::StarshipLaunched, GameEventType::CardPlayed]
    } else {
        &[GameEventType::StarshipLaunched]
    };
    let reveal = TriggerDef::new("spacerace-stimulus", on, move |ctx, event| {
        if !answers(ctx, event, radiant) {
            return vec![];
        }
        vec![
            remember(json_as(json!({ "key": REVEALED, "value": true }))),
            animate(json_as(json!({}))),
        ]
    })
    .with_when(move |ctx, event| answers(ctx, event, radiant));
    Script {
        triggers: vec![reveal],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: stimulus(false),
        radiant: stimulus(true),
    }
}

// S #131 Spacerace Stimulus — SPEC §8.10 row 131, BUILD M12 row S 131.
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{built_starship, launch_starship};
    use jackioh_engine::prelude::{dormant_starship_of, starship_pieces_of};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const AP_ROUNDS: &str = "stinkyscape-100"; // (1) 3/1 Starship Piece, Equip, Pierce: keywords only.
    const VANILLA: &str = "core-008"; // (1) 4/4, no piece.
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy a Unit.

    /// p1's Starship (one AP Rounds) and, in p2's backrow lane 2, a set Stimulus (Radiant if `radiant`);
    /// p1 holds `hand` beside it. Returns the scenario, the Starship's id and the Stimulus's.
    fn setup(seed: &str, radiant: bool, hand: Value) -> (Scenario, String, String) {
        let (s, ship) = built_starship(
            seed,
            json!([{ "def": AP_ROUNDS, "lane": 1 }]),
            json!({ "hand": hand }),
            json!({ "backrow": [{ "def": ID, "lane": 2, "faceUp": false, "radiant": radiant }] }),
        );
        let stimulus = s.backrow(P2, 2).expect("the set Stimulus").id;
        (s, ship, stimulus)
    }

    fn is_set(s: &Scenario, id: &str) -> bool {
        let card = s.card(id);
        matches!(card.zone, Zone::Field { row: Row::Backrow, lane: 2, .. }) && card.face_up != Some(true)
    }

    #[test]
    fn r1859003_set_it_animates_as_a_6_6_starship_piece_when_either_player_launches() {
        let (mut s, ship, id) = setup("r1859003-launch", false, json!([]));
        assert!(is_set(&s, &id));
        launch_starship(&mut s, &ship);
        let card = s.card(id.as_str()).clone();
        assert!(matches!(card.zone, Zone::Field { player: P2, row: Row::Units, lane: 2 }), "{:?}", card.zone);
        let stats = s.stats(card.id.as_str());
        assert_eq!((stats.attack, stats.health), (6, 6));
        assert!(stats.keywords.iter().any(|keyword| keyword.kind() == KeywordKind::StarshipPiece));
        assert!(s.last_events().iter().any(|event| event.event_type() == GameEventType::Animated));
    }

    #[test]
    fn r1859003_its_death_as_a_unit_fuses_into_its_owners_starship() {
        let (mut s, ship, id) = setup("r1859003-fuses", false, json!([HIT_JOB]));
        launch_starship(&mut s, &ship);
        let card = s.card(id.as_str()).clone();
        s.state_mut().players.p1.mana.current = 10;
        s.play(HIT_JOB, json!({ "targets": pick::id(&card.id) }));
        let theirs = dormant_starship_of(s.state(), P2).expect("p2's Starship");
        assert_eq!(starship_pieces_of(theirs)[0].def_id, ID);
    }

    #[test]
    fn r1859003_with_no_open_unit_zone_it_stays_face_up_in_the_backrow() {
        let (mut s, ship) = built_starship(
            "r1859003-full",
            json!([{ "def": AP_ROUNDS, "lane": 1 }]),
            json!({}),
            json!({
                "field": (1..=5).map(|lane| json!({ "def": VANILLA, "lane": lane })).collect::<Vec<_>>(),
                "backrow": [{ "def": ID, "lane": 2, "faceUp": false }],
            }),
        );
        let id = s.backrow(P2, 2).expect("the set Stimulus").id;
        launch_starship(&mut s, &ship);
        let card = s.card(id.as_str()).clone();
        assert!(matches!(card.zone, Zone::Field { row: Row::Backrow, lane: 2, .. }));
        assert!(card.face_up == Some(true) || card.revealed == Some(true));
    }

    #[test]
    fn r1859003_a_piece_played_leaves_the_base_face_set() {
        let (mut s, _ship, id) = setup("r1859003-played-base", false, json!([AP_ROUNDS]));
        s.play(AP_ROUNDS, json!({ "zone": 3 }));
        assert!(is_set(&s, &id));
    }

    #[test]
    fn r1859003_radiant_it_also_reveals_on_a_piece_played_as_a_12_12() {
        let (mut s, _ship, id) = setup("r1859003-played-radiant", true, json!([VANILLA, AP_ROUNDS]));
        s.play(VANILLA, json!({ "zone": 3 }));
        assert!(is_set(&s, &id), "a Unit that is no piece leaves it set");
        s.play(AP_ROUNDS, json!({ "zone": 4 }));
        let card = s.card(id.as_str()).clone();
        assert!(matches!(card.zone, Zone::Field { player: P2, row: Row::Units, .. }), "{:?}", card.zone);
        let stats = s.stats(card.id.as_str());
        assert_eq!((stats.attack, stats.health), (12, 12));
    }

    #[test]
    fn r1859003_radiant_a_piece_set_face_down_or_its_own_play_reveals_nothing() {
        let (mut s, _ship, id) = setup("r1859003-face-down", true, json!([{ "def": ID, "radiant": true }]));
        let mine = s
            .state()
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.def_id == ID)
            .expect("p1's Stimulus")
            .id
            .clone();
        // Backrow lane 3 holds the Dormant Starship (R2249000); lane 4 is open.
        s.play(mine.as_str(), json!({ "zone": 4 }));
        assert!(is_set(&s, &id));
        let set = s.backrow(P1, 4).expect("p1's Stimulus, set");
        assert_eq!(set.def_id, ID);
        assert_ne!(set.face_up, Some(true));
    }
}
