//! SPEC §8.12 S+ #132 Great Paperclip Theory II (§6.3 Choose one; R81, R169, R275, R640, R1420, R1863000,
//! R3535000, R3565002): (2) Spell, Quickdraw, Mindustrial, Paperclip, Mythic, a card of the Stinkyscape+ set.
//!   Base:    "Choose one α or β;
//!            Add a Dagger/Flare to your hand. The first time you play each Automa-α/β cost card, add the
//!            subsequent cost Automa-α/β to your hand."
//!   Radiant: "Choose one α or β;
//!            Add a Radiant Dagger/Flare to your hand. The first time you play each Automa-α/β cost card,
//!            add the subsequent cost Radiant Automa-α/β to your hand."
//!
//! Quickdraw deals in opening hand (R640). The choice is one of two modes made with the play (R81): α adds
//! Dagger and climbs Dagger, Mace, Fortress, Scepter, Reign; β adds Flare and climbs Flare, Horizon, Zenith,
//! Antumbra, Eclipse (S+ #110 to #119, one for each cost (0) to (4); R3565002), and no mode reads as α. It
//! sets one `watch_first_plays` on its caster over the rungs (0) to (3) (R1863000): the first time they play
//! each of them, the next rung up is added to their hand, Radiant on the Radiant face. The watch's badge is
//! the face's text and the ladder, "(α)" or "(β)" (R169). The "Radiant … cost card" of the Radiant text does
//! not narrow which plays count (R3565002, rejecting a watch only a Radiant play answers: R275).

use jackioh_engine::effects::{WatchFirstPlaysArgs, watch_first_plays};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-132";

/// "Choose one α or β": the two options.
const ALPHA_MODE: &str = "α";
const BETA_MODE: &str = "β";

/// Dagger to Reign (S+ #110 to #114), by cost (0) to (4).
const ALPHA: [&str; 5] = [
    "stinkyscapeplus-110",
    "stinkyscapeplus-111",
    "stinkyscapeplus-112",
    "stinkyscapeplus-113",
    "stinkyscapeplus-114",
];
/// Flare to Eclipse (S+ #115 to #119), by cost (0) to (4).
const BETA: [&str; 5] = [
    "stinkyscapeplus-115",
    "stinkyscapeplus-116",
    "stinkyscapeplus-117",
    "stinkyscapeplus-118",
    "stinkyscapeplus-119",
];

/// The step the watcher re-enters for each rung's first resolved play.
const NEXT_RUNG: &str = "next-rung";

/// R3565002: the ladder a mode climbs; no mode reads as α.
fn ladder(mode: &str) -> &'static [&'static str; 5] {
    if mode == BETA_MODE { &BETA } else { &ALPHA }
}

/// The mode the play chose, α when it chose none.
fn mode_of(ctx: &EffectContext<'_>) -> &'static str {
    if chosen_options(ctx).first().map(String::as_str) == Some(BETA_MODE) {
        BETA_MODE
    } else {
        ALPHA_MODE
    }
}

/// R1863000: the rung after the one the watcher answered (`defId`), Radiant on `radiant`; none after (4).
fn next_rung(ctx: &EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let Some(played) = ctx.data.get("defId").and_then(Value::as_str) else {
        return vec![];
    };
    let Some(next) = [&ALPHA, &BETA].into_iter().find_map(|rungs| {
        let at = rungs.iter().position(|rung| *rung == played)?;
        rungs.get(at + 1)
    }) else {
        return vec![];
    };
    vec![add_to_hand(json_as(json!({ "defId": next, "radiant": radiant })))]
}

fn face(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![ALPHA_MODE.to_string(), BETA_MODE.to_string()],
        }],
        cry: Some(hook(move |ctx| {
            let mode = mode_of(ctx);
            let rungs = ladder(mode);
            let kind = if radiant { FaceKind::Radiant } else { FaceKind::Base };
            let text = fill_params(def_of(Some(&*ctx.state), ID), kind, None);
            vec![
                add_to_hand(json_as(json!({ "defId": rungs[0], "radiant": radiant }))),
                watch_first_plays(WatchFirstPlaysArgs {
                    // The last rung adds none, so the watch names the rungs before it.
                    def_ids: rungs[..rungs.len() - 1].iter().map(|rung| rung.to_string()).collect(),
                    step: NEXT_RUNG.to_string(),
                    label: format!("{text} ({mode})"),
                    hook: None,
                    data: None,
                }),
            ]
        })),
        resume: IndexMap::from([(NEXT_RUNG, hook(move |ctx| next_rung(ctx, radiant)))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S+ #132 Great Paperclip Theory II — SPEC §8.12 row S+ 132 (R3565002; R81, R169, R640, R1863000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const STAND_IN: &str = "stand-in (0) 1/1";

    fn setup(hand: Value, p2: Value) -> Scenario {
        let mut cards = hand.as_array().cloned().unwrap_or_default();
        cards.extend([json!(FILLER), json!(FILLER), json!(FILLER)]);
        let mut p2 = p2;
        if p2.get("hand").is_none() {
            p2["hand"] = json!([FILLER]);
        }
        p2["library"] = filler(6);
        scenario(json!({
            "seed": "s132",
            "p1": { "hand": cards, "library": filler(6), "mana": 40 },
            "p2": p2,
        }))
    }

    fn held(s: &Scenario, player: PlayerId, def_id: &str) -> Vec<bool> {
        s.hand(player)
            .into_iter()
            .filter(|card| card.def_id == def_id)
            .map(|card| card.radiant)
            .collect()
    }

    /// The Automa cards of both ladders in p1's hand.
    fn rungs_held(s: &Scenario) -> usize {
        s.hand(P1)
            .iter()
            .filter(|card| ALPHA.contains(&card.def_id.as_str()) || BETA.contains(&card.def_id.as_str()))
            .count()
    }

    fn watched(s: &Scenario) -> Vec<Vec<String>> {
        s.state().players.p1.mods
            .iter()
            .filter_map(|modifier| match &modifier.kind {
                ModifierKind::PlayWatch { def_ids, .. } => Some(def_ids.clone()),
                _ => None,
            })
            .collect()
    }

    fn rung_ids(rungs: &[&str]) -> Vec<String> {
        rungs.iter().map(|rung| rung.to_string()).collect()
    }

    fn cast(s: &mut Scenario, mode: &str) {
        s.play(ID, json!({ "modes": [mode] }));
    }

    #[test]
    fn r640_quickdraw_on_both_faces() {
        crate::register_all();
        let scripts = script();
        assert_eq!(scripts.base.flags().quickdraw, Some(true));
        assert_eq!(scripts.radiant.flags().quickdraw, Some(true));
        // Plain Core cards, then the card: one copy of each (§2.6 L3).
        let others = [
            "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015",
            "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043",
            "core-044", "core-053", "core-055", "core-001", "core-003", "core-004", "core-009", "core-010",
        ];
        let mut deck: Vec<String> = others.iter().map(|id| id.to_string()).collect();
        deck.push(ID.to_string());
        let created = create_game(&CreateGameOptions {
            seed: "s132-quickdraw".to_string(),
            decks: (deck.clone(), deck),
            ..CreateGameOptions::default()
        });
        let state = begin_game(&created).state;
        assert!(state.players.p1.hand.iter().any(|card| card.def_id == ID));
        assert!(state.players.p1.library.iter().all(|card| card.def_id != ID));
    }

    #[test]
    fn r3565002_the_ladders_are_the_cost_ordered_alpha_and_beta_cards() {
        crate::register_all();
        for (ladder, tag) in [(ALPHA, Tag::AutomaAlpha), (BETA, Tag::AutomaBeta)] {
            for (cost, id) in ladder.iter().enumerate() {
                let def = crate::card_def(id);
                assert!(def.tags.contains(&tag), "{id}");
                assert_eq!(def.cost, CardCost::Fixed(cost as i32), "{id}");
            }
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r3565002_alpha_adds_a_dagger_and_watches_dagger_to_scepter() {
            let mut s = setup(json!([ID]), json!({}));
            cast(&mut s, ALPHA_MODE);
            assert_eq!(held(&s, P1, ALPHA[0]), vec![false]);
            assert_eq!(rungs_held(&s), 1);
            assert_eq!(watched(&s), vec![rung_ids(&ALPHA[..4])]);
        }

        #[test]
        fn r3565002_beta_adds_a_flare_and_watches_flare_to_antumbra() {
            let mut s = setup(json!([ID]), json!({}));
            cast(&mut s, BETA_MODE);
            assert_eq!(held(&s, P1, BETA[0]), vec![false]);
            assert_eq!(rungs_held(&s), 1);
            assert_eq!(watched(&s), vec![rung_ids(&BETA[..4])]);
        }

        #[test]
        fn r3565002_no_mode_reads_as_alpha() {
            assert_eq!(ladder(""), &ALPHA);
            assert_eq!(ladder(ALPHA_MODE), &ALPHA);
            assert_eq!(ladder(BETA_MODE), &BETA);
        }

        #[test]
        fn r3565002_each_first_play_adds_the_next_rung() {
            let mut s = setup(json!([ID, STAND_IN]), json!({}));
            cast(&mut s, ALPHA_MODE);
            for (at, rung) in ALPHA[..3].iter().enumerate() {
                assert_eq!(held(&s, P1, rung), vec![false], "rung {at} is in hand");
                assert_eq!(rungs_held(&s), 1);
                if at == 2 {
                    // Fortress asks a Tribute: the stand-in pays it.
                    let unit = s.card(STAND_IN).id.clone();
                    s.play(STAND_IN, json!({ "zone": 1 }));
                    s.play(*rung, json!({ "zone": 2, "tributes": [unit.as_str()] }));
                } else {
                    s.play(*rung, json!({ "zone": at as i32 + 3 }));
                }
            }
            assert_eq!(held(&s, P1, ALPHA[3]), vec![false], "Fortress added Scepter");
            assert_eq!(watched(&s), vec![rung_ids(&ALPHA[3..4])], "only Scepter's rung is left");
        }

        #[test]
        fn r3565002_a_second_play_adds_nothing() {
            let mut s = setup(json!([ID]), json!({}));
            cast(&mut s, ALPHA_MODE);
            s.play(ALPHA[0], json!({ "zone": 1 }));
            assert_eq!(held(&s, P1, ALPHA[1]), vec![false]);
            // A second Dagger, created by hand: no rung is added for it.
            crate::run_as(
                &mut s,
                jackioh_engine::effects::add_to_hand(json_as(json!({ "defId": ALPHA[0] }))),
                P1,
            );
            s.play(ALPHA[0], json!({ "zone": 2 }));
            assert_eq!(held(&s, P1, ALPHA[1]), vec![false], "still the one");
        }

        #[test]
        fn r3565002_two_casts_keep_their_own_ladders() {
            let mut s = setup(json!([ID, ID]), json!({}));
            cast(&mut s, ALPHA_MODE);
            cast(&mut s, BETA_MODE);
            assert_eq!(watched(&s), vec![rung_ids(&ALPHA[..4]), rung_ids(&BETA[..4])]);
            s.play(ALPHA[0], json!({ "zone": 1 }));
            assert_eq!(held(&s, P1, ALPHA[1]), vec![false]);
            assert!(held(&s, P1, BETA[1]).is_empty());
            s.play(BETA[0], json!({ "zone": 2 }));
            assert_eq!(held(&s, P1, BETA[1]), vec![false]);
        }

        #[test]
        fn r3565002_the_opponents_play_adds_nothing() {
            let mut s = setup(json!([ID]), json!({ "hand": [ALPHA[0], FILLER] }));
            cast(&mut s, ALPHA_MODE);
            s.end_turn();
            s.play(ALPHA[0], json!({ "zone": 1 }));
            assert!(held(&s, P1, ALPHA[1]).is_empty());
            assert!(held(&s, P2, ALPHA[1]).is_empty());
            assert_eq!(watched(&s), vec![rung_ids(&ALPHA[..4])]);
        }

        #[test]
        fn r3565002_r169_the_badge_names_the_ladder() {
            let labels = |modifiers: &[ModifierView]| -> Vec<String> {
                modifiers.iter().map(|modifier| modifier.label.clone()).collect()
            };
            let text = crate::card_def(ID).base.text;
            for (mode, ladder) in [(ALPHA_MODE, "α"), (BETA_MODE, "β")] {
                let mut s = setup(json!([ID]), json!({}));
                cast(&mut s, mode);
                let badge = format!("{text} ({ladder})");
                assert_eq!(labels(&s.view(P1).you.modifiers), vec![badge.clone()]);
                assert_eq!(labels(&s.view(P2).opponent.modifiers), vec![badge]);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3565002_radiant_every_rung_is_radiant() {
            for (mode, ladder) in [(ALPHA_MODE, ALPHA), (BETA_MODE, BETA)] {
                let mut s = setup(json!([{ "def": ID, "radiant": true }, STAND_IN]), json!({}));
                cast(&mut s, mode);
                assert_eq!(held(&s, P1, ladder[0]), vec![true], "{mode}");
                s.play(ladder[0], json!({ "zone": 1 }));
                assert_eq!(held(&s, P1, ladder[1]), vec![true], "{mode}");
                s.play(ladder[1], json!({ "zone": 2 }));
                assert_eq!(held(&s, P1, ladder[2]), vec![true], "{mode}");
            }
        }

        #[test]
        fn r3565002_radiant_the_badge_is_the_radiant_text_and_the_ladder() {
            let mut s = setup(json!([{ "def": ID, "radiant": true }]), json!({}));
            cast(&mut s, BETA_MODE);
            let text = crate::card_def(ID).radiant.text;
            assert_eq!(s.view(P1).you.modifiers[0].label, format!("{text} (β)"));
        }
    }
}
