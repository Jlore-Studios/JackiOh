//! M #85 Playtester (SPEC §8.8 row 85): (2) Unit, Rare, 4/5 → 8/10.
//!
//! Base:    "Start of turn: At random, add {books|Book of Buff|Books of Buff} or {books|Book of
//!          Nerf|Books of Nerf} to your hand."
//! Radiant: "Start of turn: At random, add {books|Radiant Book of Buff|Radiant Books of Buff} or
//!          {books|Radiant Book of Nerf|Radiant Books of Nerf} to your hand."
//! Engine: at its controller's start of turn (R62), one uniform pick of C+ #71 `classicplus-071` or
//! C+ #72 `classicplus-072` by the match rng (R2179004, issue #2179), then `add_to_hand` of {books}
//! copies of the one picked (Radiant: `radiant: true`). Both are real cards, named, so no pool and no
//! R387. No prompt opens; the opponent sees a hidden card arrive.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-085";

/// C+ #71 Book of Buff, the first mode.
const BUFF_BOOK: &str = "classicplus-071";
/// C+ #72 Book of Nerf, the second mode.
const NERF_BOOK: &str = "classicplus-072";

/// R2179004: the two Books one pick is made among, in the order the text names them.
const BOOKS: [&str; 2] = [BUFF_BOOK, NERF_BOOK];

/// The two modes of v0.5.2's prompt, worded as the faces printed them. The same labels on both faces.
const BUFF: &str = "Book of Buff";
const NERF: &str = "Book of Nerf";

/// The resume step v0.5.2's mode prompt re-entered (§10.6).
const STEP_BOOK: &str = "book";

/// {books} copies of the Book `id`, Radiant on the Radiant face.
fn add_books(ctx: &EffectContext<'_>, id: &str, radiant: bool) -> Vec<Effect> {
    let n = param(ctx, "books").max(0) as usize;
    (0..n)
        .map(|_| add_to_hand(json_as(json!({ "defId": id, "radiant": radiant }))))
        .collect()
}

fn playtester(radiant: bool) -> Script {
    Script {
        // R2179004: one pick of the two by the match rng, then {books} copies of it.
        start_of_turn: Some(hook(move |ctx| {
            let Some(id) = ctx.rng.pick(&BOOKS).copied() else {
                return vec![];
            };
            add_books(ctx, id, radiant)
        })),
        ..Script::default()
    }
}

/// v0.5.2's face: a mode prompt for its controller (R1161), then the Book chosen.
fn playtester_v0_5_2(radiant: bool) -> Script {
    Script {
        start_of_turn: Some(hook(|_ctx| {
            vec![choose_mode(json_as(json!({
                "options": [BUFF, NERF],
                "step": STEP_BOOK,
                "prompt": "Playtester: choose one",
            })))]
        })),
        resume: IndexMap::from([(
            STEP_BOOK,
            hook(move |ctx| {
                let picked = chosen_options(ctx).into_iter().next();
                let id = match picked.as_deref() {
                    Some(NERF) => NERF_BOOK,
                    _ => BUFF_BOOK,
                };
                add_books(ctx, id, radiant)
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: playtester(false), radiant: playtester(true) }
}

/// R767000: through v0.5.2 its controller chose the Book; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_2() -> CardScripts {
    CardScripts { base: playtester_v0_5_2(false), radiant: playtester_v0_5_2(true) }
}

// M #85 Playtester — SPEC §8.8 row 85, BUILD M10 row M 85.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::def_ids;
    use crate::filler;
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PLAYTESTER: &str = "meditative-085";
    const FILLER: &str = "core-005";

    /// p1's Playtester stands on the field (base unless `radiant_face`); both sides hold and draw
    /// fillers, so turns pass normally.
    fn standing(seed: &str, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [FILLER],
                "field": [{ "def": PLAYTESTER, "radiant": radiant_face, "lane": 1 }],
                "library": filler(4),
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// The Books p1's start of turn added: the hand cards that are neither `before` nor the turn's draw.
    fn books_added(s: &Scenario, before: &[String]) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| !before.contains(&card.id) && BOOKS.contains(&card.def_id.as_str()))
            .collect()
    }

    fn ids(s: &Scenario) -> Vec<String> {
        s.hand(P1).iter().map(|card| card.id.clone()).collect()
    }

    fn option_labels(s: &Scenario) -> Vec<String> {
        s.state()
            .pending
            .as_ref()
            .map(|pending| pending.options.iter().map(|option| option.label.clone()).collect())
            .unwrap_or_default()
    }

    mod m85_playtester {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r2179004_your_start_of_turn_adds_one_book_at_random_and_opens_no_prompt() {
                let mut s = standing("playtester", false);
                let before = ids(&s);
                s.start_turn();
                assert!(s.state().pending.is_none());
                let made = books_added(&s, &before);
                assert_eq!(made.len(), 1);
                assert!(!made[0].radiant);
                // The turn draw plus the Book.
                assert_eq!(s.hand(P1).len(), before.len() + 2);
                // Hidden from the opponent: a count, never the card.
                let view: Value = js(&s.view(P2));
                assert_eq!(view["opponent"]["hand"], json!({ "count": s.hand(P1).len() }));
                assert!(!serde_json::to_string(&view["events"]).expect("events").contains(&made[0].def_id));
            }

            #[test]
            fn r2179004_across_seeds_both_books_come_and_the_same_seed_adds_the_same_one() {
                let mut seen: IndexSet<String> = IndexSet::new();
                for n in 0..16 {
                    let mut s = standing(&format!("playtester-{n}"), false);
                    let before = ids(&s);
                    s.start_turn();
                    let made = books_added(&s, &before);
                    let mut again = standing(&format!("playtester-{n}"), false);
                    let again_before = ids(&again);
                    again.start_turn();
                    assert_eq!(def_ids(&made), def_ids(&books_added(&again, &again_before)));
                    seen.extend(def_ids(&made));
                }
                assert_eq!(seen.len(), 2, "{seen:?}");
            }

            #[test]
            fn nothing_at_the_opponents_start_of_turn() {
                let mut s = standing("playtester-foe", false);
                let before = ids(&s);
                s.end_turn();
                assert!(s.state().pending.is_none());
                assert!(books_added(&s, &before).is_empty());
            }

            #[test]
            fn a_full_hand_burns_the_book() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "playtester-full",
                    "p1": {
                        "hand": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                        "field": [{ "def": PLAYTESTER, "lane": 1 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.start_turn();
                assert!(s.events().iter().any(|event| matches!(event, GameEvent::Burned { .. })));
                assert!(!s.hand(P1).iter().any(|card| BOOKS.contains(&card.def_id.as_str())));
            }

            #[test]
            fn r2179004_r386_books_reads_through_param_and_adds_that_many_of_the_one_picked() {
                crate::register_all();
                let mut s = standing("playtester-param", false);
                crate::upgrade_number(&mut s, PLAYTESTER, "books");
                let before = ids(&s);
                s.start_turn();
                // The turn draw plus two Books, both the same one.
                assert_eq!(s.hand(P1).len(), before.len() + 3);
                let made = books_added(&s, &before);
                assert_eq!(made.len(), 2);
                assert_eq!(made[0].def_id, made[1].def_id);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r2179004_radiant_the_book_is_radiant() {
                let mut s = standing("playtester-radiant", true);
                let before = ids(&s);
                s.start_turn();
                assert!(s.state().pending.is_none());
                let made = books_added(&s, &before);
                assert_eq!(made.len(), 1);
                assert!(made[0].radiant);
            }
        }

        /// R767000: the script kept for the replays of v0.5.2 and before, whose controller chose the Book.
        mod v0_5_2 {
            use super::*;

            #[test]
            fn r767000_r1161_a_game_on_v0_5_2_asks_its_controller_and_adds_the_book_chosen() {
                crate::under_kept(PLAYTESTER, script_v0_5_2(), || {
                    let mut s = standing("playtester-kept", false);
                    let before = ids(&s);
                    s.start_turn();
                    let pending = s.state().pending.clone().expect("a prompt is open");
                    assert_eq!(pending.kind, PromptKind::Mode);
                    assert_eq!(option_labels(&s), [BUFF.to_string(), NERF.to_string()]);
                    s.answer(json!(NERF));
                    assert_eq!(def_ids(&books_added(&s, &before)), vec![NERF_BOOK.to_string()]);
                });
            }
        }
    }
}
