//! Stinkyscape #111 Shockwave447 (SPEC §8.10 row 111). (4) Unit, Human, Acclaimed, Mythic, 4/7 → 8/14.
//!   Base:    "Aura: Has {keywords|random combat keyword|random combat keywords}.\nCry: Replace both
//!            players' hands with random complicated cards." — keywords 3
//!   Radiant: "… Cry: Replace both players' hands with random complicated cards. Make yours Radiant." — 3
//!
//! The Aura is M #65 Keymaster Keenus's (R781004): §10.4's layers read with no rng, so the keywords are
//! drawn as it arrives on the field (played, summoned, Flickered, back through Reborn), different ones
//! from the combat keyword pool, none it has, into one list kept on it until it leaves the field (R78).
//! The Aura gives the first `keywords` of it: a Buff of the number, a number set higher or the Radiant
//! face draws only the rest, a lower number takes the newest off and the number back returns them
//! (R3778000). The Aura reads them back (`conditionalKeywords`), so a Vanilla takes them.
//! Cry (R1): C+ #65.5's `replace_hand_with_random` for each player, its controller's first: each hand is
//! moved to its owner's graveyard (no discard) and as many random cards of `{ complicated: true }`
//! (R1846000) added, repeats allowed (R60), never this card (R387); an empty hand gets none (R129). The
//! pool is the sets that ship, as no `everySet` is asked (R1846001). The Radiant face makes its
//! controller's new cards Radiant, the opponent's not. The count is the declared `keywords` (R386).

use jackioh_engine::effects::{
    DrawCombatKeywordsArgs, aura_combat_keywords, draw_combat_keywords, drawn_combat_keywords,
    top_up_combat_keywords,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-111";

/// R781004: this card arrived on the field.
fn arrived(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = ctx.live_self() else {
        return false;
    };
    matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == me.id)
}

/// R3778000: `top_up` draws what the number asks beyond the list; the kept script's draw keeps any it
/// holds (R781004).
fn draw_on_arrival(top_up: bool) -> TriggerDef {
    TriggerDef::new(
        "shockwave447-combat-keywords",
        &[GameEventType::Summoned],
        move |ctx, event| {
            if !arrived(ctx, event) {
                return vec![];
            }
            let args = DrawCombatKeywordsArgs {
                count: param(&*ctx, "keywords"),
                key: None,
            };
            vec![if top_up {
                top_up_combat_keywords(args)
            } else {
                draw_combat_keywords(args)
            }]
        },
    )
    .with_when(|ctx, event| arrived(ctx, event))
}

fn shockwave(radiant: bool, top_up: bool) -> Script {
    Script {
        triggers: vec![draw_on_arrival(top_up)],
        // R3778000: a Buff, a number set or the Radiant face that raises its number draws the rest.
        combat_keywords_follow: top_up.then_some("keywords"),
        conditional_keywords: Some(if top_up {
            read_hook(|args| aura_combat_keywords(&args, "keywords", None))
        } else {
            read_hook(|args| drawn_combat_keywords(args.self_, None))
        }),
        cry: Some(hook(move |_ctx| {
            let mut yours = json!({ "query": { "complicated": true } });
            if radiant {
                yours["radiant"] = json!(true);
            }
            vec![
                replace_hand_with_random(json_as(yours)),
                replace_hand_with_random(json_as(json!({ "query": { "complicated": true }, "player": "enemy" }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: shockwave(false, true),
        radiant: shockwave(true, true),
    }
}

/// R767000: through v0.6.0b the Aura drew once as it arrived and kept its keywords whatever its
/// number did (R781004); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_0b() -> CardScripts {
    CardScripts {
        base: shockwave(false, false),
        radiant: shockwave(true, false),
    }
}

// Stinkyscape #111 Shockwave447 — SPEC §8.10 row 111, BUILD M12 row S 111 (R1846000, R781004).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::catalog::is_complicated;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // Stockpile: a Spell
    use crate::BOOK_OF_BUFF;
    const VANILLA: &str = "core-008"; // Mr. Vanilla

    fn played(radiant: bool, seed: &str, theirs: usize) -> (Scenario, Vec<String>, Vec<String>) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER, VANILLA], "library": filler(6), "mana": 10 },
            "p2": { "hand": vec![FILLER; theirs], "library": filler(6) },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        let mine: Vec<String> = s.hand(P1).into_iter().skip(1).map(|card| card.id).collect();
        let their: Vec<String> = s.hand(P2).into_iter().map(|card| card.id).collect();
        s.play(ID, json!({ "zone": 1 }));
        (s, mine, their)
    }

    fn me(s: &Scenario) -> String {
        s.unit(P1, 1).expect("Shockwave447").id
    }

    fn drawn(s: &Scenario, id: &str) -> Vec<KeywordKind> {
        drawn_combat_keywords(s.card(id), None)
            .iter()
            .map(Keyword::kind)
            .collect()
    }

    /// R3778000: the drawn keywords it acts with, in draw order: what its Aura gives.
    fn aura(s: &Scenario, id: &str) -> Vec<KeywordKind> {
        let acting: Vec<KeywordKind> = keywords_of(s.state(), s.card(id)).iter().map(Keyword::kind).collect();
        drawn(s, id).into_iter().filter(|kind| acting.contains(kind)).collect()
    }

    /// Shockwave447 on the field and, as its Cry replaces the hands, the Book of Buff drawn the next turn
    /// (a library's first card is drawn first).
    fn with_book(seed: &str) -> (Scenario, String) {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [ID], "library": [BOOK_OF_BUFF, FILLER, FILLER, FILLER], "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s.play(ID, json!({ "zone": 1 }));
        s.end_turn();
        s.end_turn();
        let id = me(&s);
        (s, id)
    }

    #[test]
    fn r781004_it_arrives_with_three_different_combat_keywords_the_same_seed_the_same() {
        let (s, _, _) = played(false, "shockwave", 2);
        let drawn: Vec<KeywordKind> = drawn_combat_keywords(s.card(me(&s).as_str()), None)
            .iter()
            .map(Keyword::kind)
            .collect();
        assert_eq!(drawn.len(), 3);
        let mut unique = drawn.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 3, "different ones");
        let acting: Vec<KeywordKind> = keywords_of(s.state(), s.card(me(&s).as_str()))
            .iter()
            .map(Keyword::kind)
            .collect();
        assert!(drawn.iter().all(|kind| acting.contains(kind)), "the Aura gives them");
        let (again, _, _) = played(false, "shockwave", 2);
        let replayed: Vec<KeywordKind> = drawn_combat_keywords(again.card(me(&again).as_str()), None)
            .iter()
            .map(Keyword::kind)
            .collect();
        assert_eq!(replayed, drawn);
    }

    #[test]
    fn r1846000_its_cry_replaces_both_hands_card_for_card_with_complicated_cards_no_discard() {
        let (s, mine, theirs) = played(false, "shockwave-hands", 3);
        for (player, old) in [(P1, &mine), (P2, &theirs)] {
            let hand = s.hand(player);
            assert_eq!(hand.len(), old.len(), "{player:?}: as many as it held");
            assert!(hand.iter().all(|card| is_complicated(&card.def_id) && card.def_id != ID));
            assert!(hand.iter().all(|card| !card.radiant), "the base face makes none Radiant");
            for id in old.iter() {
                assert_eq!(s.card(id.as_str()).zone, Zone::Graveyard { player });
            }
        }
        assert!(
            !s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Discarded { .. })),
            "moved, not discarded"
        );
    }

    #[test]
    fn keywords_reads_through_param() {
        let mut s = scenario(json!({
            "seed": "shockwave-param",
            "p1": { "hand": [ID], "library": filler(6), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        let keywords = crate::upgrade_number(&mut s, ID, "keywords");
        s.play(ID, json!({ "zone": 1 }));
        let id = me(&s);
        let list = drawn(&s, &id);
        assert_eq!(list.len(), usize::try_from(keywords).expect("a count"));
        // R3778000: and on the field it follows the number, a Nerf taking the newest off.
        assert_eq!(crate::degrade_number(&mut s, &id, "keywords"), keywords - 1);
        assert_eq!(aura(&s, &id), list[..list.len() - 1]);
    }

    #[test]
    fn r3778000_a_buff_of_its_number_keeps_its_3_and_draws_a_4th() {
        let (seed, s, id) = crate::buff_number_once("shockwave-3778", "keywords", with_book);
        let (before, first) = with_book(&seed);
        let three = drawn(&before, &first);
        assert_eq!(three.len(), 3);

        let four = drawn(&s, &id);
        assert_eq!(four.len(), 4, "{four:?}");
        assert_eq!(four[..3], three[..], "it keeps the 3 it drew, in their order");
        assert!(!three.contains(&four[3]), "the 4th is none it had: {four:?}");
        assert_eq!(aura(&s, &id), four, "it acts with all 4");
    }

    #[test]
    fn r3778000_a_nerf_takes_off_only_its_newest_and_a_buff_back_returns_it() {
        let (mut s, _, _) = played(false, "shockwave-3778-nerf", 2);
        let id = me(&s);
        let three = drawn(&s, &id);

        assert_eq!(crate::degrade_number(&mut s, &id, "keywords"), 2);
        assert_eq!(aura(&s, &id), three[..2], "only the newest, {:?}, comes off", three[2]);
        assert_eq!(drawn(&s, &id), three, "the list stays whole");

        assert_eq!(crate::upgrade_number(&mut s, &id, "keywords"), 3);
        assert_eq!(aura(&s, &id), three, "the same one comes back");
    }

    #[test]
    fn r767000_the_script_kept_for_v0_6_0b_keeps_its_3_whatever_its_number() {
        crate::under_kept(ID, script_v0_6_0b(), || {
            let (mut s, _, _) = played(false, "shockwave-kept", 2);
            let id = me(&s);
            let three = drawn(&s, &id);
            assert_eq!(three.len(), 3);
            assert_eq!(crate::degrade_number(&mut s, &id, "keywords"), 2);
            assert_eq!(aura(&s, &id), three, "it kept all 3 through v0.6.0b");
        });
    }

    #[test]
    fn r1846000_an_empty_hand_gets_none() {
        let (s, _, _) = played(false, "shockwave-empty", 0);
        assert!(s.hand(P2).is_empty());
        assert_eq!(s.hand(P1).len(), 2);
    }

    #[test]
    fn r1846000_radiant_is_an_8_14_and_only_your_new_cards_are_radiant() {
        let (s, _, _) = played(true, "shockwave-radiant", 2);
        assert!(s.hand(P1).iter().all(|card| card.radiant && is_complicated(&card.def_id)));
        assert!(s.hand(P2).iter().all(|card| !card.radiant));
        let stats = s.stats(me(&s).as_str());
        assert_eq!((stats.attack, stats.health), (8, 14));
        let drawn = drawn_combat_keywords(s.card(me(&s).as_str()), None);
        assert_eq!(drawn.len(), 3, "the Radiant face's 3");
    }
}
