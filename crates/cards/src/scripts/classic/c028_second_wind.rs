//! C #28 Second Wind (SPEC §8.6 row 28, §6.1 Indestructible and Flashback, §6.2 Aura; R1, R16, R1869003,
//! R3951001). Field Spell, cost 0, Legendary.
//!   Base:    "Indestructible\nCry: Discard your hand and exile your deck.\nAura: Every card in your deck
//!            and graveyard has Flashback."
//!   Radiant: "Indestructible\nCry: Discard your hand and your deck.\nAura: Every card in your deck and
//!            graveyard has Flashback."
//!   Engine:  Indestructible is a catalog keyword (§6.1). The Cry (R1) discards the whole hand (R16: no
//!            choice), then the base face exiles the deck, each card its own exile (R135), and the Radiant
//!            face sends it to the graveyard (`discard_library`, R3951001), where its cards can be played
//!            again. The Aura is S #152 Sanctum's (R1869003): while this acts on the field, every card in
//!            its controller's deck and graveyard has Flashback (R783012), so each graveyard card is
//!            offered once as a play from there at its cost, and a played one's `flashbackSpent` keeps the
//!            aura off it for good (R783013). With no deck left, every draw is fatigue (§2.4).
//!   Issue #3951 gave it this text. Through v0.6.2b its Cry exiled the deck and then discarded the
//!   hand, and its Aura let its controller play (1)+ Cost cards from their graveyard (`minCost`) any
//!   number of times, the base face exiling every card of theirs that would go there; `script_v0_6_2b`
//!   keeps that for those replays (R767000, R393, R65).

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-028";

/// R3951001: the Cry: the hand discarded, then the deck exiled (base) or discarded (Radiant).
fn cry(radiant: bool) -> Hook {
    hook(move |_ctx| {
        let deck = if radiant {
            discard_library(DiscardLibraryArgs::default())
        } else {
            exile_matching(json_as(json!({ "zones": ["library"] })))
        };
        vec![discard_hand(Default::default()), deck]
    })
}

pub fn script() -> CardScripts {
    let face = |radiant: bool| Script {
        cry: Some(cry(radiant)),
        aura: Some(crate::scripts::stinkyscape::c152_sanctum::flashback_aura()),
        ..Script::default()
    };
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

/// The instance-memory mark the Cry holds while its own discard lands (R393).
const CRY_RUNNING: &str = "secondWindCry";

fn recalled_on<'a>(card: &'a CardInstance, key: &str) -> Option<&'a Value> {
    card.memory.get(key)
}

/// Both faces' Cry: exile the deck, each card its own exile (R135), then discard the hand (R16: a whole
/// hand needs no choice). Its discard must land in the graveyard before the Aura starts exiling (R393),
/// so the Cry marks itself as running (`remember`) around it.
fn cry_v0_6_2b() -> Hook {
    hook(|_ctx| {
        vec![
            remember(json_as(json!({ "key": CRY_RUNNING, "value": true }))),
            exile_matching(json_as(json!({ "zones": ["library"] }))),
            discard_hand(Default::default()),
            remember(json_as(json!({ "key": CRY_RUNNING, "value": false }))),
        ]
    })
}

/// Both faces' Aura (`Script.graveyardPlay`, B5 E11): while this is on the field, any card of your own
/// graveyard is played as from a hand (§10.5), at a price of at least the declared `minCost` as paid.
fn graveyard_play() -> GraveyardPlayHook {
    Arc::new(|args| {
        vec![GraveyardPlayPermission {
            min_price: Some(param(&args, "minCost")),
            ..GraveyardPlayPermission::default()
        }]
    })
}

/// R767000: the scripts through v0.6.2b (issue #3951): the Cry exiled the deck, then discarded the hand,
/// and the Aura let its controller play (`minCost`)+ Cost cards from their graveyard, the base face
/// exiling their cards that would go there. `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_2b() -> CardScripts {
    let base = Script {
        cry: Some(cry_v0_6_2b()),
        graveyard_play: Some(graveyard_play()),
        // B5 E5: your cards that would go to a graveyard are exiled, but not while the Cry's mark is on
        // (R393); a Second Wind that arrives without its Cry (a Recruit, a summon) has no mark.
        replacements: vec![ReplacementDef {
            id: "second-wind-exile".into(),
            on: ReplacementMoment::ToGraveyard,
            where_: None,
            when: Some(replacement_when(|ctx| {
                let owned = matches!(ctx.event, ReplacedEvent::ToGraveyard { owner, .. } if *owner == ctx.controller);
                owned && recalled_on(ctx.self_, CRY_RUNNING) != Some(&Value::Bool(true))
            })),
            instead: ReplacementInstead {
                to: Some(InsteadTo::Exile),
                ..ReplacementInstead::default()
            },
            then: None,
            by: None,
        }],
        ..Script::default()
    };

    let radiant = Script {
        cry: Some(cry_v0_6_2b()),
        graveyard_play: Some(graveyard_play()),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #28 Second Wind — SPEC §8.6 row 28, BUILD M9 Classic row C 28 (#64, R393, R65, §2.4, R386).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::def_ids;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const WIND: &str = "classic-028";
    const STOCKPILE: &str = "core-005"; // Stockpile: a Spell
    const ECLIPSE: &str = "core-035"; // Lunar Eclipse: a Spell with a target
    const MR_TOKEN: &str = "core-015"; // Me and Mr Token: a Unit with a Cry
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const MENACE: &str = "core-019"; // Midrange Menace: a Unit

    /// The defs of p1's graveyard cards `legalActions` offers a play of.
    fn graveyard_defs_offered(s: &Scenario) -> Vec<String> {
        let offered: IndexSet<String> = legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, .. } => Some(instance_id),
                _ => None,
            })
            .collect();
        s.pile(P1, "graveyard")
            .into_iter()
            .filter(|card| offered.contains(&card.id))
            .map(|card| card.def_id)
            .collect()
    }

    fn game(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": WIND, "radiant": radiant }, STOCKPILE, MR_TOKEN],
                "library": [VANILLA, MENACE, ECLIPSE],
                "mana": 10,
            },
            "p2": { "hand": [STOCKPILE] },
        }))
    }

    fn count(s: &Scenario, kind: &str) -> usize {
        s.events().iter().filter(|event| event.event_type().as_str() == kind).count()
    }

    /// Issue #3951: no number, Indestructible, the Aura Sanctum's, and no play permission or replacement of
    /// its own any more.
    #[test]
    fn it_declares_no_number_is_indestructible_and_lends_flashback() {
        crate::register_all();
        let def = crate::card_def(WIND);
        assert!(def.params.is_none());
        for face in [&def.base, &def.radiant] {
            assert!(face.keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Indestructible));
        }
        let scripts = script();
        for face in [&scripts.base, &scripts.radiant] {
            assert!(face.aura.is_some());
            assert!(face.graveyard_play.is_none());
            assert!(face.replacements.is_empty());
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r3951001_the_cry_discards_your_hand_then_exiles_your_deck() {
            let mut s = game(false);
            let library = s.pile(P1, "library");
            s.play(WIND, json!({ "zone": 1 }));
            assert!(s.hand(P1).is_empty());
            assert!(s.pile(P1, "library").is_empty());
            for card in &library {
                s.expect_in_zone(card, "exile");
            }
            assert_eq!(def_ids(&s.pile(P1, "graveyard")), vec![STOCKPILE, MR_TOKEN]);
            s.expect_events(json!(["discarded", "exiled"]));
        }

        #[test]
        fn r1869003_your_graveyard_cards_have_flashback_each_played_once() {
            let mut s = game(false);
            s.play(WIND, json!({ "zone": 1 }));
            assert_eq!(graveyard_defs_offered(&s), vec![STOCKPILE, MR_TOKEN]);
            let stockpile = s.pile(P1, "graveyard").into_iter().find(|card| card.def_id == STOCKPILE).expect("Stockpile");
            let action = legal_actions(s.state(), P1)
                .into_iter()
                .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == stockpile.id))
                .expect("its Flashback play is offered");
            s.act(action, P1, "play Stockpile from the graveyard");
            s.expect_in_zone(&stockpile, "graveyard");
            assert_eq!(graveyard_defs_offered(&s), vec![MR_TOKEN], "its Flashback is spent");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3951001_the_cry_discards_your_hand_and_your_deck_into_your_graveyard() {
            let mut s = game(true);
            s.play(WIND, json!({ "zone": 1 }));
            assert!(s.hand(P1).is_empty());
            assert!(s.pile(P1, "library").is_empty());
            assert!(s.pile(P1, "exile").is_empty());
            assert_eq!(def_ids(&s.pile(P1, "graveyard")), vec![STOCKPILE, MR_TOKEN, VANILLA, MENACE, ECLIPSE]);
            assert_eq!(count(&s, "discarded"), 2, "only the hand is discarded");
            assert_eq!(count(&s, "enteredGraveyard"), 5);
            assert_eq!(count(&s, "fatigue"), 0, "no draw");
        }

        #[test]
        fn r1869003_every_card_of_the_deck_it_discarded_has_flashback() {
            let mut s = game(true);
            s.play(WIND, json!({ "zone": 1 }));
            assert_eq!(graveyard_defs_offered(&s), vec![STOCKPILE, MR_TOKEN, VANILLA, MENACE, ECLIPSE]);
        }
    }

    /// R767000: the scripts kept for v0.6.2b, with the `minCost` they declared then.
    mod kept_v0_6_2b {
        use super::*;

        fn kept<T>(run: impl FnOnce() -> T) -> T {
            crate::under_kept_as(
                WIND,
                script_v0_6_2b(),
                |def| {
                    def.params = serde_json::from_value(json!([
                        { "key": "minCost", "base": 1, "radiant": 1, "better": "down", "step": 1, "min": 1 }
                    ]))
                    .expect("the v0.6.2b params");
                },
                run,
            )
        }

        #[test]
        fn r393_the_discards_land_in_your_graveyard_before_the_aura_starts_so_they_are_playable() {
            kept(|| {
                let mut s = scenario(json!({
                    "p1": { "hand": [WIND, ECLIPSE, MR_TOKEN], "library": [VANILLA] },
                    "p2": { "hand": [STOCKPILE] },
                }));
                s.play(WIND, json!({ "zone": 1 }));
                s.expect_events(json!(["exiled", "discarded"]));
                assert_eq!(def_ids(&s.pile(P1, "graveyard")), vec![ECLIPSE, MR_TOKEN]);
                assert_eq!(graveyard_defs_offered(&s), vec![ECLIPSE, MR_TOKEN]);
            });
        }
    }
}
