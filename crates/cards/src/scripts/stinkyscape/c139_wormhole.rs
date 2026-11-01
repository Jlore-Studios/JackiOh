//! S #139 Wormhole (SPEC §8.10 row 139; R448, R1850004, R1875007): (2) Trap, Rare, of the
//! Stinkyscape set.
//!   Base:    "Reveals when your opponent plays a Spell aimed only at their hero or permanents they
//!            control: Counter it and cast it for yourself, aimed only at your hero or permanents you
//!            control."
//!   Radiant: the same, "Counter it, make it Radiant and cast it for yourself, …".
//! Engine: set face-down, it answers in §10.5 step 3a's announce window an opponent's Spell whose chosen
//! targets, one at least, are all that player's own hero or cards they control on the field
//! (`query::beneficial_on_own_side`, R1850004; R1875007: a Spell that chose no target is not one, nor one
//! aimed at Wormhole's own side). The Spell is countered into Wormhole's controller's hand, its owner
//! becoming them (`counter_play` to the thief, R448), and cast by them (R70) with `friendsOnly`: every
//! target pick of the cast, and every one its text asks for while it is cast, is its caster's own hero or
//! a card they control on the field, a declaration with none passed (R90). Radiant: `set_radiant` on the
//! countered card before the cast, so it resolves Radiant and lands Radiant in its new owner's graveyard.
//! Fused into a card that stands in a unit zone and is no trap there, its text never fires (R2226000).
//! Declares no numbers.

use jackioh_engine::effects::{cast, counter_play, set_radiant};
use jackioh_engine::prelude::*;
use jackioh_engine::query::beneficial_on_own_side;

pub const ID: &str = "stinkyscape-139";

/// R1875007: an opponent's Spell aimed only at their own side.
fn answers(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    matches!(event, GameEvent::CardAnnounced { player, .. } if *player != ctx.controller)
        && beneficial_on_own_side(&*ctx.state, event).is_some()
}

fn wormhole(radiant: bool) -> Script {
    let steal = TriggerDef::new("wormhole", &[GameEventType::CardAnnounced], move |ctx, event| {
        let Some(spell) = beneficial_on_own_side(&*ctx.state, event) else {
            return Vec::new();
        };
        let card = json!({ "of": "instance", "instanceId": spell });
        let mut effects = vec![counter_play(json_as(json!({ "to": "thief", "target": card.clone() })))];
        if radiant {
            effects.push(set_radiant(json_as(json!({ "instanceId": spell }))));
        }
        effects.push(cast(json_as(json!({ "target": card, "friendsOnly": true }))));
        effects
    })
    .with_when(|ctx, event| answers(ctx, event));
    Script {
        triggers: vec![steal],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: wormhole(false),
        radiant: wormhole(true),
    }
}

// S #139 Wormhole — SPEC §8.10 row 139, BUILD M12 row S 139.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // (1) Spell Stockpile: draws, aims at nothing.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const GREASER: &str = "meditative-100"; // (2) 7/7.
    const BOOK: &str = "classicplus-057"; // (1) Spell Book of Stats: "Give a Unit +5/+5" (+10/+10 Radiant).
    const BUFF: i32 = 5;
    const RADIANT_BUFF: i32 = 10;

    fn trapped(radiant: bool) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": "wormhole",
            "p1": {
                "hand": [BOOK, FILLER],
                "field": [{ "def": VANILLA, "lane": 1 }],
                "library": crate::filler(6),
                "mana": 10,
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": VANILLA, "lane": 1 }, { "def": GREASER, "lane": 2 }],
                "backrow": [{ "def": ID, "radiant": radiant, "lane": 1 }],
                "library": crate::filler(6),
            },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    fn fired(s: &Scenario) -> bool {
        s.last_events().iter().any(|event| event.event_type() == GameEventType::TrapFired)
    }

    fn book_in_graveyard_of(s: &Scenario, player: PlayerId) -> Option<CardInstance> {
        s.pile(player, "graveyard").into_iter().find(|card| card.def_id == BOOK)
    }

    mod base {
        use super::*;

        #[test]
        fn r1875007_an_enemy_spell_on_its_own_unit_is_countered_and_cast_by_its_controller_who_picks() {
            let mut s = trapped(false);
            let mine = s.unit_at(P1, 1);
            s.play(BOOK, json!({ "targets": pick::instance(&mine) }));
            assert!(fired(&s));
            let pending = s.state().pending.clone().expect("the cast's target prompt");
            assert_eq!(pending.player_id, P2);
            let greaser = s.unit_at(P2, 2);
            let offered: Vec<Selection> = pending.options.iter().map(|option| option.selection.clone()).collect();
            assert_eq!(
                offered,
                [s.unit_at(P2, 1).id, greaser.id.clone()]
                    .map(|instance_id| Selection::Instance { instance_id })
                    .to_vec()
            );
            s.answer(pick::instance(&greaser));
            s.expect_stats(&greaser, json!({ "attack": 7 + BUFF, "health": 7 + BUFF }));
            s.expect_stats(&mine, json!({ "attack": 4, "health": 4 }));
            let book = book_in_graveyard_of(&s, P2).expect("the Book lands in p2's graveyard");
            assert_eq!((book.owner, book.radiant), (P2, false));
            assert!(book_in_graveyard_of(&s, P1).is_none());
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r1875007_a_spell_aimed_at_its_controllers_unit_or_at_nothing_leaves_it_set() {
            let mut s = trapped(false);
            let theirs = s.unit_at(P2, 2);
            s.play(BOOK, json!({ "targets": pick::instance(&theirs) }));
            assert!(!fired(&s));
            s.expect_stats(&theirs, json!({ "attack": 7 + BUFF }));
            s.play(FILLER, json!({}));
            assert!(!fired(&s));
            assert_eq!(s.backrow(P2, 1).map(|card| card.def_id), Some(ID.to_string()));
        }
    }

    /// R102, R1884003, R2226000: Wormhole's text fused onto a Unit stands in a unit zone on a card that is
    /// no trap, so it never fires there: its controller's own Spell aimed at their own Unit resolves, and
    /// so does the opponent's, which R1884003's reading of its condition countered before.
    #[test]
    fn r1884003_r2226000_fused_onto_a_unit_it_answers_no_spell_from_its_unit_zone() {
        use jackioh_engine::prelude::*;
        crate::register_all();
        let mut s = crate::scenario(json!({
            "seed": "wormhole-fused",
            "active": "p2",
            "p1": {
                "hand": [BOOK],
                "field": [{ "def": VANILLA, "lane": 1 }],
                "library": crate::filler(6),
                "mana": 10,
            },
            "p2": { "hand": [BOOK], "library": crate::filler(6), "mana": 10 },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        // A Vanilla and a Wormhole fused into p2's hand, as Craft a Card fuses (R77).
        let mut events: Vec<GameEvent> = vec![];
        let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
        let fused = {
            let state = s.state_mut();
            let ingredients: Vec<CardInstance> = [VANILLA, ID]
                .iter()
                .map(|def_id| new_instance(&mut *state, def_id, P2, Zone::Gone { player: P2 }))
                .collect();
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            subsystems::fuse(
                &mut sink,
                subsystems::FuseArgs {
                    ingredients,
                    to_hand: Some(P2),
                    ..subsystems::FuseArgs::default()
                },
            )
        }
        .expect("the fused card");
        s.state_mut().rng_cursor = rng.cursor();
        s.play(&fused.id, json!({ "zone": 1 }));
        let fused = s.unit_at(P2, 1);

        let countered = |s: &Scenario| {
            s.last_events()
                .iter()
                .any(|event| event.event_type() == GameEventType::Countered)
        };
        s.play(BOOK, json!({ "targets": pick::instance(&fused) }));
        assert!(!countered(&s), "its controller's own Spell resolves");
        s.expect_stats(&fused, json!({ "attack": 4 + BUFF, "health": 4 + BUFF }));

        s.end_turn();
        let mine = s.unit_at(P1, 1);
        s.play(BOOK, json!({ "targets": pick::instance(&mine) }));
        assert!(!countered(&s), "the opponent's resolves too");
        assert!(s.state().pending.is_none());
        s.expect_stats(&mine, json!({ "attack": 4 + BUFF, "health": 4 + BUFF }));
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1875007_it_resolves_radiant_and_lands_radiant() {
            let mut s = trapped(true);
            let mine = s.unit_at(P1, 1);
            s.play(BOOK, json!({ "targets": pick::instance(&mine) }));
            assert!(fired(&s));
            let greaser = s.unit_at(P2, 2);
            s.answer(pick::instance(&greaser));
            s.expect_stats(&greaser, json!({ "attack": 7 + RADIANT_BUFF, "health": 7 + RADIANT_BUFF }));
            let book = book_in_graveyard_of(&s, P2).expect("the Book lands in p2's graveyard");
            assert!(book.radiant);
        }
    }
}
