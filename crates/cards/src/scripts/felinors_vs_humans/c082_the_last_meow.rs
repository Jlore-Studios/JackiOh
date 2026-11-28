//! SPEC §8.14 FvH #82 The Last Meow ((4) Spell, Terminus, Mythic), a card of the held Felinors vs Humans
//! set. Built by S10 (#5260).
//!   Base:    "Exile your hand and board. Fill your board with Shrinking Felinors. When the last one dies,
//!            you lose the game."
//!   Radiant: "Exile your hand and board. Fill your board with Radiant Shrinking Felinors. When the last
//!            one dies, you lose the game."
//!
//! Terminus is a family tag (R1837000). In the text's order: `exile_hand` and `exile_board` of its caster
//! (R3539002's board exile, both rows); `fill_board` with M #94 Shrinking Felinor, every unit zone, a
//! Locked one included (R1136000), Radiant on the Radiant face, no Cry (R1); then `watch_deaths` on Felinor
//! (R5238009), a never-expiring watch run after each Felinor death's state check, so a dying Shrinking
//! Felinor's own Death has summoned its smaller self first. R5260000 (FV2 #82): when the dead card was a
//! Shrinking Felinor its caster controlled as it died and the caster then controls none, the caster loses
//! (`win_game` for the opponent): the last one is the 3/3 the chain ends on. A body lost another way
//! (bounce, steal, transform, exile) loses nothing by itself. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-082";

/// M #94 Shrinking Felinor, the board it fills.
const SHRINKING_FELINOR: &str = "meditative-094";

/// The step the death watch re-enters after each Felinor death (R5238009).
const ONE_DIED: &str = "oneDied";
/// R169: the watch's badge, in the card's own words.
const WATCH_LABEL: &str = "When the last Shrinking Felinor dies, you lose the game";

/// R5260000: after a Felinor's death, its own Death run, whether it was the holder's last Shrinking
/// Felinor: the dead card one, its controller as it died the watch's holder, and that player's side
/// holding none now.
fn the_last_one_died(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let holder = ctx.data.get(DEATH_WATCH_HOLDER_KEY).and_then(Value::as_str);
    if holder != Some(ctx.controller.as_str()) {
        return vec![];
    }
    let dead = ctx
        .data
        .get("instanceId")
        .and_then(Value::as_str)
        .and_then(|id| find_instance(ctx.state, id));
    if !dead.is_some_and(|card| card.def_id == SHRINKING_FELINOR) {
        return vec![];
    }
    if units_on_field_of(ctx.state, ctx.controller).iter().any(|unit| unit.def_id == SHRINKING_FELINOR) {
        return vec![];
    }
    vec![win_game(json_as(json!({ "player": "enemy" })))]
}

fn last_meow(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![
                exile_hand(json_as(json!({}))),
                exile_board(json_as(json!({}))),
                fill_board(json_as(json!({ "defId": SHRINKING_FELINOR, "radiant": radiant }))),
                watch_deaths(WatchDeathsArgs {
                    tag: Tag::Felinor,
                    step: ONE_DIED.to_string(),
                    label: WATCH_LABEL.to_string(),
                    hook: None,
                    data: None,
                }),
            ]
        })),
        resume: IndexMap::from([(ONE_DIED, hook(the_last_one_died))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: last_meow(false),
        radiant: last_meow(true),
    }
}

// FvH #82 The Last Meow — SPEC §8.14 row 82 (R1, R1136000, R3539002, R5238009, R5260000).
#[cfg(test)]
mod tests {
    use super::{ID, SHRINKING_FELINOR};
    use jackioh_engine::testkit::*;

    use crate::{def_ids, filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    const PLAGUE_NUKE: &str = "classic-043"; // Plague Nuke: destroys all Units
    /// A Unit, a Field Spell and a hand card for the Meow to exile.
    const UNIT: &str = "stand-in (1) 1/1";
    const FIELD: &str = "stand-in (1) Field Spell";
    const SPARE: &str = "stand-in (9) Spell";

    /// p1 casts the Meow over a Unit, a Field Spell and two hand cards; p2 holds `p2_hand`.
    fn meowed(radiant: bool, p2_hand: Value) -> Scenario {
        let mut s = weapon_board(
            "fvh082",
            json!({
                "hand": [{ "def": ID, "radiant": radiant }, SPARE, SPARE],
                "field": [UNIT],
                "backrow": [FIELD],
                "library": filler(6),
                "mana": 10,
            }),
            json!({ "hand": p2_hand, "library": filler(6), "mana": 10 }),
        );
        s.play(ID, json!({}));
        s
    }

    /// p1's unit zones, lane by lane: each card's def id and whether it is Radiant.
    fn board(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5).map(|lane| s.unit(P1, lane).map(|card| (card.def_id, card.radiant))).collect()
    }

    fn filled(radiant: bool) -> Vec<Option<(String, bool)>> {
        vec![Some((SHRINKING_FELINOR.to_string(), radiant)); 5]
    }

    /// p2 casts a Plague Nuke on its own turn, then hands the turn back round to p2.
    fn nuke_and_pass(s: &mut Scenario) {
        s.play(PLAGUE_NUKE, json!({}));
        s.end_turn().end_turn();
    }

    fn exiles_your_hand_and_board(radiant: bool) {
        let mut s = meowed(radiant, json!([]));
        assert!(s.hand(P1).is_empty());
        let mut exiled = def_ids(&s.pile(P1, "exile"));
        exiled.sort();
        let mut expected = vec![FIELD, SPARE, SPARE, UNIT];
        expected.sort_unstable();
        assert_eq!(exiled, expected);
        assert!(s.backrow(P1, 1).is_none());
        assert_eq!(board(&s), filled(radiant));
        s.expect_in_zone(ID, "graveyard");
    }

    mod base {
        use super::*;

        #[test]
        fn r3539002_exiles_your_hand_and_board_and_fills_your_board_with_shrinking_felinors() {
            exiles_your_hand_and_board(false);
        }

        #[test]
        fn r5260000_a_shrinking_felinor_whose_death_summons_its_smaller_self_loses_nothing() {
            let mut s = meowed(false, json!([PLAGUE_NUKE]));
            s.end_turn();
            s.play(PLAGUE_NUKE, json!({}));
            assert!(s.state().result.is_none());
            assert_eq!(board(&s), filled(false));
            for lane in 1..=5 {
                s.expect_stats(s.unit_at(P1, lane), json!({ "attack": 6, "health": 6 }));
            }
        }

        #[test]
        fn r5260000_you_lose_once_the_last_shrinking_felinor_dies() {
            let mut s = meowed(false, json!([PLAGUE_NUKE, PLAGUE_NUKE, PLAGUE_NUKE]));
            s.end_turn();
            // 9/9s to 6/6s, then 6/6s to 3/3s: each death summons the next, so the game goes on.
            nuke_and_pass(&mut s);
            nuke_and_pass(&mut s);
            assert!(s.state().result.is_none());
            assert_eq!(board(&s), filled(false));
            // The 3/3s summon nothing: the last one's death loses p1 the game.
            s.play(PLAGUE_NUKE, json!({}));
            let result = must(s.state().result, "the game is over");
            assert_eq!(result.winner, Winner::P2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3539002_fills_your_board_with_radiant_shrinking_felinors() {
            exiles_your_hand_and_board(true);
        }

        #[test]
        fn r5260000_the_radiant_felinors_shrink_and_the_game_goes_on() {
            let mut s = meowed(true, json!([PLAGUE_NUKE]));
            s.end_turn();
            s.play(PLAGUE_NUKE, json!({}));
            assert!(s.state().result.is_none());
            for lane in 1..=5 {
                assert_eq!(s.unit_at(P1, lane).def_id, SHRINKING_FELINOR);
            }
        }
    }
}
