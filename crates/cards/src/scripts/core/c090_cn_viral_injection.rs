//! #90 CN-Viral Injection (SPEC §8.5, §7, R11, R12, R57, R58, R80).
//!
//! Base: "Shuffle a CN-Virus into the opponent's library". Radiant: "A Radiant CN-Virus".
//! Engine cell: "The opponent owns the token, so its cast-on-draw chain runs on their draws; a full
//! library refuses the shuffle (R80)."
//!
//! §8's Conventions: the radiant cell restates only WHAT is shuffled, so the count (one), the
//! destination (the opponent's library) and everything else are kept.
//!
//! OWNERSHIP IS THE WHOLE POINT. `shuffleInto` creates the token with `newInstance(state, defId,
//! player, …)`, and `newInstance` sets both `owner` and `controller` to that player, so `player:
//! "enemy"` makes the opponent the OWNER of the virus, not merely its controller. Off the field
//! ownership is what decides everything (R12): the card sits in their library, feeds their draws,
//! and its cast-on-draw chain therefore runs on their turn and damages their hero. A steal-style
//! control change would have done none of that, which is why this is `player`, not a target.
//!
//! `shuffleIntoLibrary` (engine/src/draw.ts) puts it at `rng.int(library.length + 1)` — a uniformly
//! random position in the whole pile, drawn from the match rng so a replay reproduces it (§9.2) —
//! and emits `shuffledIn` carrying that position. R80: a library holds at most `LIBRARY_CAP` cards
//! and a card that would be shuffled into a full one "is not created", so a full library simply
//! refuses the shuffle and this spell fizzles; the spell still counts as played (§8 Conventions).
//!
//! THE RADIANT FLAG TRAVELS, NOTHING ELSE DOES. `shuffleInto`'s `radiant` sets the flag on the fresh
//! instance, which is what R57 says a copy shuffled into a library carries. The flag then decides
//! which face runs on the draw (§5.2): `flagsOf` reads `scriptOf(instance)`, so a Radiant CN-Virus
//! casts its radiant text and shuffles 3 copies instead of 2 — and `shuffleCopiesOfSelf` copies the
//! flag onto those, so the whole chain stays Radiant.
//!
//! The token's id comes from the catalog through `cardDef`, never a string literal: #90.1 is a real
//! catalog entry and `cardDef` throws if it ever stops being one.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-090";

/// #90.1 CN-Virus, the token this card shuffles (§7). `build.rs` proves the id is a catalog entry
/// (SURFACE §7.4), as `cardDef` did.
const VIRUS: &str = "core-090-1";

/// `radiantVirus` is the whole of the radiant text.
fn injection(radiant_virus: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![shuffle_into(json_as(json!({
                "defId": VIRUS,
                "count": 1,
                "player": "enemy",
                "radiant": radiant_virus,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: injection(false),
        radiant: injection(true),
    }
}

// #90 CN-Viral Injection and #90.1 CN-Virus (SPEC §8 rows 90 / 90.1, §7, §2.4, §4.4, §9.2;
// R11, R12, R57, R58, R63, R70, R80, R316, R350).
//
// BUILD M4-T4 row 90:   "Virus shuffled into the opponent's library at a random position; radiant
//                        virus is radiant". Patch v0.1.1 (issue #27) made it cost 2.
// BUILD M4-T4 row 90.1: "On draw: 1 damage through the pipeline (Going Long reduces it), draw
//                        again; 2 copies shuffled in at the end of the turn (R350); a chain stops at
//                        20 casts (R58); radiant 3 copies". R275 scales the radiant face's damage
//                        too: "take 2 damage; 3 copies".
//
// The two cards are tested in one file because #90's whole effect is to hand #90.1 to the OTHER
// player: ownership (R12) is what makes the token's cast-on-draw chain run on the opponent's draws
// and damage the opponent's hero, so "who owns the virus" is asserted on #90 and "what the virus
// does to its owner" on #90.1.
//
// R11 is the difference from every unit token in the game: #90.1 is a SPELL token, so it lives in
// a hand and a library like a real card and reaches the graveyard after resolving. A card that
// ceased to exist is tagged `gone`, never `exile` (R86), and nothing here is ever `gone`.
//
// Two routes reach the virus's script and both are tested: playing it from hand (its `cry` alone,
// with no draw around it) and drawing it (`staticFlags.castOnDraw`, which is where R58's chain cap
// lives). R350 holds a cast's copies back to the end of the turn it was cast on, so a chain casts
// only the viruses the library already held: CAST_ON_DRAW_CHAIN_CAP is 20, so a library of 21
// viruses costs its owner exactly 20 health and leaves the 21st in hand uncast, and the 40 copies
// go in as the turn ends.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const INJECTION: &str = "core-090";
    const VIRUS: &str = "core-090-1";

    /// R58's cap, restated from `engine/src/config.ts` so a change to it fails here by name.
    const CHAIN_CAP: usize = 20;
    /// R80's cap, likewise.
    const LIBRARY_CAP: usize = 60;

    /// One `shuffledIn` event, as TS's `shuffledIn` helper reduced it.
    #[derive(Clone, Debug, PartialEq)]
    struct Shuffled {
        def_id: String,
        position: i32,
        player: PlayerId,
    }

    fn shuffled_in(s: &Scenario, player: Option<PlayerId>) -> Vec<Shuffled> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::ShuffledIn {
                    player: who,
                    def_id,
                    position,
                    ..
                } if player.is_none() || player == Some(*who) => Some(Shuffled {
                    def_id: def_id.clone(),
                    position: *position,
                    player: *who,
                }),
                _ => None,
            })
            .collect()
    }

    fn damage_to(s: &Scenario, target: &str) -> Vec<i32> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if target_id == target => Some(*amount),
                _ => None,
            })
            .collect()
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    /// A pile of harmless ordinary cards, for a library whose length is the point.
    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// `type` of an event, as TS names it.
    fn type_of(event: &GameEvent) -> String {
        event.event_type().to_string()
    }

    use crate::js;

    use crate::matches_object;

    fn library_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.state().players[player].library.clone()
    }

    // =========================================================================================
    // #90 CN-Viral Injection — base
    // =========================================================================================

    mod n90_cn_viral_injection_base {
        use super::*;

        #[test]
        fn shuffles_one_cn_virus_into_the_opponent_s_library_and_nothing_into_yours() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-enemy-library",
                "p1": { "hand": [INJECTION, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"], "library": filler(4) },
            }));

            s.play(INJECTION, json!({}));

            assert_eq!(library_of(&s, P2).len(), 5);
            assert!(def_ids(&library_of(&s, P2)).contains(&VIRUS.to_string()));
            // Nothing was added to the caster's own library.
            assert_eq!(library_of(&s, P1).len(), 3);
            assert!(!def_ids(&library_of(&s, P1)).contains(&VIRUS.to_string()));
            s.expect_events(json!(["cardPlayed", "shuffledIn"]));
        }

        #[test]
        fn r12_the_opponent_owns_the_virus_so_it_feeds_their_draws_and_not_yours() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-ownership",
                "p1": { "hand": [INJECTION, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"], "library": filler(4) },
            }));

            s.play(INJECTION, json!({}));

            let virus = library_of(&s, P2).into_iter().find(|card| card.def_id == VIRUS);
            assert!(virus.is_some());
            let virus = virus.unwrap();
            assert_eq!(virus.owner, P2);
            assert_eq!(virus.controller, P2);
            let shuffled = shuffled_in(&s, None);
            assert_eq!(shuffled.len(), 1);
            assert_eq!(shuffled[0].def_id, VIRUS);
            assert_eq!(shuffled[0].player, P2);
        }

        #[test]
        fn the_base_face_shuffles_a_non_radiant_virus_s8_only_the_radiant_cell_adds_the_flag() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-not-radiant",
                "p1": { "hand": [INJECTION, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"], "library": filler(4) },
            }));

            s.play(INJECTION, json!({}));

            let virus = library_of(&s, P2).into_iter().find(|card| card.def_id == VIRUS);
            assert_eq!(virus.map(|card| card.radiant), Some(false));
        }

        #[test]
        fn s3_2_the_spell_itself_reaches_the_graveyard_and_counts_as_played() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-graveyard",
                "p1": { "hand": [INJECTION, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"], "library": filler(4) },
            }));

            s.play(INJECTION, json!({}));

            s.expect_in_zone(INJECTION, "graveyard");
            assert_eq!(s.state().players[P1].turn_log.cards_played, 1);
            // Patch v0.1.1: it costs 2.
            s.expect_mana(P1, 2);
        }

        #[test]
        fn costs_2_patch_v0_1_1() {
            crate::register_all();
            let s = scenario(json!({ "seed": "core-090-cost", "p1": { "hand": [INJECTION] } }));
            let cost = match s.view(Some(P1)).you.hand {
                HandView::Cards(cards) => cards.first().map(|card| card.cost),
                HandView::Count { .. } => None,
            };
            assert_eq!(cost, Some(2));
        }

        #[test]
        fn s9_2_the_random_position_is_inside_the_whole_pile_and_replays_identically_from_the_seed() {
            crate::register_all();
            let build = || {
                let mut s = scenario(json!({
                    "seed": "core-090-replay",
                    "p1": { "hand": [INJECTION, "core-005"], "library": filler(3) },
                    "p2": { "hand": ["core-005"], "library": filler(4) },
                }));
                s.play(INJECTION, json!({}));
                s
            };

            let first = shuffled_in(&build(), None);
            let second = shuffled_in(&build(), None);

            assert_eq!(first.len(), 1);
            // A uniformly random spot in a pile of 4 is one of the 5 gaps 0..4 (`shuffleIntoLibrary`).
            assert!(first[0].position >= 0);
            assert!(first[0].position <= 4);
            // Same seed, same steps, same position — which is what makes replay and fuzz meaningful.
            assert_eq!(second, first);
        }

        #[test]
        fn r80_a_full_library_refuses_the_shuffle_no_virus_is_created_and_the_spell_still_resolves() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-library-cap",
                "p1": { "hand": [INJECTION, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"], "library": filler(LIBRARY_CAP) },
            }));

            s.play(INJECTION, json!({}));

            assert_eq!(library_of(&s, P2).len(), LIBRARY_CAP);
            assert!(!def_ids(&library_of(&s, P2)).contains(&VIRUS.to_string()));
            assert!(shuffled_in(&s, None).is_empty());
            // §8 Conventions: a fizzled clause does not un-play the card.
            s.expect_in_zone(INJECTION, "graveyard");
            assert_eq!(s.state().players[P1].turn_log.cards_played, 1);
        }
    }

    // =========================================================================================
    // #90 CN-Viral Injection — radiant
    // =========================================================================================

    mod n90_cn_viral_injection_radiant {
        use super::*;

        #[test]
        fn a_radiant_cn_virus_the_flag_is_set_and_everything_else_is_kept_s8_conventions() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-radiant",
                "p1": { "hand": [{ "def": INJECTION, "radiant": true }, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"], "library": filler(4) },
            }));

            s.play(INJECTION, json!({}));

            let virus = library_of(&s, P2).into_iter().find(|card| card.def_id == VIRUS);
            assert_eq!(virus.as_ref().map(|card| card.radiant), Some(true));
            // Still ONE virus, still the opponent's library, still their card.
            assert_eq!(shuffled_in(&s, None).len(), 1);
            assert_eq!(virus.map(|card| card.owner), Some(P2));
            assert_eq!(library_of(&s, P2).len(), 5);
        }

        #[test]
        fn the_radiant_virus_runs_its_radiant_face_when_drawn_2_damage_then_3_radiant_copies_as_the_turn_ends() {
            crate::register_all();
            // The virus sits on top of its owner's library, so the very next draw casts it.
            let mut s = scenario(json!({
                "seed": "core-090-radiant-face",
                "p1": { "library": [{ "def": VIRUS, "radiant": true }, "core-005"], "hand": [] },
                "p2": { "hand": ["core-005"] },
            }));

            s.start_turn();

            // One cast of the radiant face, then the draw repeats and finds the Stockpile.
            assert_eq!(damage_to(&s, "hero-p1"), vec![2]);
            assert_eq!(def_ids(&s.hand(Some(P1))), vec!["core-005".to_string()]);
            assert!(shuffled_in(&s, Some(P1)).is_empty());

            s.end_turn();

            // R350, R57: three copies at the end of the turn, all Radiant.
            let copies: Vec<Shuffled> =
                shuffled_in(&s, Some(P1)).into_iter().filter(|event| event.def_id == VIRUS).collect();
            assert_eq!(copies.len(), 3);
            assert_eq!(library_of(&s, P1).iter().filter(|card| card.def_id == VIRUS).count(), 3);
            assert!(library_of(&s, P1).iter().all(|card| card.radiant));
        }
    }

    // =========================================================================================
    // R311: what the library's owner is shown of a virus going in (SPEC §10.8)
    // =========================================================================================

    mod n90_and_n90_1_r311_the_owner_s_library_list {
        use super::*;

        #[test]
        fn r311_the_victim_s_list_names_the_virus_the_opponent_s_injection_shuffled_in_radiant_on_the_radiant_face() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-r311",
                "p1": { "hand": [{ "def": INJECTION, "radiant": true }, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"], "library": filler(4) },
            }));

            s.play(INJECTION, json!({}));

            // The play was public and its text names the card, so p2 knows what went in; never where.
            // Both cost 1, so the list goes by name (R310): CN-Virus before Stockpile.
            assert_eq!(
                js(&s.view(Some(P2)).you.own_library),
                json!({
                    "cards": [
                        { "defId": VIRUS, "radiant": true, "count": 1 },
                        { "defId": "core-005", "radiant": false, "count": 4 },
                    ],
                    "unknown": 0,
                })
            );
            // The caster reads p2's library as a count and nothing else.
            assert!(s.view(Some(P1)).opponent.own_library.is_none());
            assert_eq!(s.view(Some(P1)).opponent.library_count, 5);
        }

        #[test]
        fn r311_a_virus_s_own_copies_are_listed_as_their_owner_saw_them_go_in() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-r311",
                "p1": { "hand": [VIRUS, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(VIRUS, json!({})).end_turn();

            let list = s.view(Some(P1)).you.own_library;
            assert_eq!(list.as_ref().map(|list| list.unknown), Some(0));
            let cards: Vec<Value> = list.map(|list| list.cards.iter().map(js).collect()).unwrap_or_default();
            assert!(cards.contains(&json!({ "defId": VIRUS, "radiant": false, "count": 2 })));
            assert!(cards.contains(&json!({ "defId": "core-005", "radiant": false, "count": 3 })));
        }
    }

    // =========================================================================================
    // #90.1 CN-Virus — base, played from hand (the `cry` with no draw around it)
    // =========================================================================================

    /// The delayed effects a CN-Virus has armed and not yet run (R350).
    fn armed_copies(s: &Scenario) -> usize {
        s.state().delayed.iter().filter(|entry| entry.resume.def_id == VIRUS).count()
    }

    /// The `shuffledIn` events of `player` that put a CN-Virus in.
    fn viruses_in(s: &Scenario, player: PlayerId) -> usize {
        shuffled_in(s, Some(player)).iter().filter(|event| event.def_id == VIRUS).count()
    }

    mod n90_1_cn_virus_base {
        use super::*;

        #[test]
        fn r350_1_damage_to_its_own_hero_at_once_and_the_2_copies_wait_for_the_end_of_the_turn() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-cry",
                "p1": { "hand": [VIRUS, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(VIRUS, json!({}));

            s.expect_health(P1, 29);
            assert_eq!(damage_to(&s, "hero-p1"), vec![1]);
            s.expect_health(P2, 30);
            // Nothing is shuffled yet: the copies are an end-of-turn delayed effect of the caster's.
            assert!(shuffled_in(&s, Some(P1)).is_empty());
            assert_eq!(library_of(&s, P1).len(), 3);
            assert_eq!(armed_copies(&s), 1);

            s.end_turn();

            assert_eq!(viruses_in(&s, P1), 2);
            assert_eq!(armed_copies(&s), 0);
            // p2 has started its turn and drawn nothing from p1's library: 3 + 2.
            assert_eq!(library_of(&s, P1).len(), 5);
        }

        #[test]
        fn r350_the_copies_go_in_at_s2_2_s_end_of_turn_delayed_effect_point_after_the_turn_ends_before_the_next_begins() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-order",
                "p1": { "hand": [VIRUS, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(VIRUS, json!({})).end_turn();

            s.expect_events(json!(["turnEnded", "shuffledIn", "shuffledIn", "turnStarted"]));
        }

        #[test]
        fn r350_each_cast_arms_its_own_shuffle_two_viruses_played_in_one_turn_shuffle_in_4_at_its_end() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-two",
                "p1": { "hand": [VIRUS, VIRUS, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"] },
            }));
            let hand = s.hand(Some(P1));
            let first = hand[0].clone();
            let second = hand[1].clone();

            s.play(&first, json!({})).play(&second, json!({}));
            s.expect_health(P1, 28);
            assert_eq!(armed_copies(&s), 2);

            s.end_turn();

            assert_eq!(viruses_in(&s, P1), 4);
        }

        #[test]
        fn r11_a_spell_token_reaches_the_graveyard_it_never_ceases_to_exist() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-graveyard",
                "p1": { "hand": [VIRUS, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"] },
            }));
            let virus = s.hand(Some(P1)).first().cloned();
            assert_eq!(virus.as_ref().map(|card| card.def_id.as_str()), Some(VIRUS));

            s.play(VIRUS, json!({}));

            // R86: "gone" is what a card that ceased to exist is tagged; a spell token is not one.
            if let Some(virus) = &virus {
                s.expect_in_zone(virus, "graveyard");
            }
            assert!(def_ids(&s.pile(P1, "graveyard")).contains(&VIRUS.to_string()));
        }

        #[test]
        fn r57_the_copies_of_a_non_radiant_virus_are_non_radiant() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-copies-plain",
                "p1": { "hand": [VIRUS, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(VIRUS, json!({})).end_turn();

            let copies: Vec<CardInstance> =
                library_of(&s, P1).into_iter().filter(|card| card.def_id == VIRUS).collect();
            assert_eq!(copies.len(), 2);
            assert!(copies.iter().all(|card| !card.radiant));
        }

        #[test]
        fn s4_4_step_2_r63_armor_absorbs_the_1_damage_no_damage_event_fires_and_the_copies_still_land() {
            crate::register_all();
            // What "Going Long (#84) reduces it" means for the pipeline: the two clauses are independent.
            let mut s = scenario(json!({
                "seed": "core-090-1-armor",
                "p1": { "hand": [VIRUS, "core-005"], "library": filler(3), "armor": 2 },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(VIRUS, json!({}));

            s.expect_health(P1, 30);
            assert_eq!(damage_to(&s, "hero-p1"), Vec::<i32>::new());

            s.end_turn();

            assert_eq!(viruses_in(&s, P1), 2);
        }

        #[test]
        fn r80_r316_the_copies_stop_at_the_library_cap_the_second_is_never_created_and_the_refusal_is_reported() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-library-cap",
                "p1": { "hand": [VIRUS, "core-005"], "library": filler(LIBRARY_CAP - 1) },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(VIRUS, json!({}));
            // The damage clause is unaffected by the library being full.
            s.expect_health(P1, 29);

            s.end_turn();

            // p2's turn has begun and p1 has drawn nothing since, so p1's library is exactly at the cap.
            assert_eq!(library_of(&s, P1).len(), LIBRARY_CAP);
            assert_eq!(viruses_in(&s, P1), 1);
            let refused: Vec<Value> = s
                .events()
                .iter()
                .filter(|event| type_of(event) == "libraryOverflow")
                .map(js)
                .collect();
            assert_eq!(refused.len(), 1);
            assert!(matches_object(
                &refused[0],
                &json!({ "player": "p1", "defId": VIRUS, "outcome": "notCreated" })
            ));
        }

        #[test]
        fn r350_a_virus_cast_on_the_opponent_s_turn_shuffles_its_copies_at_the_end_of_that_turn() {
            crate::register_all();
            // At p2's start of turn p2's #9 Moths to the Flame (worn to 4 health) makes p1's #32 Prem Panther
            // (5/4) attack it; the Panther destroys it and survives, so p1 draws 2 on p2's turn (R426): the
            // CN-Virus on top of p1's library is cast there, and "the end of the turn" is p2's.
            let mut s = scenario(json!({
                "seed": "core-090-1-their-turn",
                "p1": { "field": ["core-032"], "library": [VIRUS, "core-005", "core-005"], "hand": ["core-005"] },
                "p2": { "field": [{ "def": "core-009", "damage": 10 }], "hand": ["core-005"] },
            }));

            s.end_turn();
            assert_eq!(s.state().active, P2);

            assert_eq!(damage_to(&s, "hero-p1"), vec![1]);
            assert!(def_ids(&s.pile(P1, "graveyard")).contains(&VIRUS.to_string()));
            assert_eq!(armed_copies(&s), 1);
            let at = s
                .state()
                .delayed
                .iter()
                .find(|entry| entry.resume.def_id == VIRUS)
                .map(|entry| js(&entry.at));
            assert_eq!(at, Some(json!({ "phase": "end", "player": "p2" })));

            s.end_turn();

            // At the end of p2's turn, into p1's own library (its owner's) — before p1's turn begins, whose
            // draw then casts them in turn.
            let last: Vec<GameEvent> = s.last_events().to_vec();
            let types: Vec<String> = last.iter().map(type_of).collect();
            let ended = types.iter().position(|kind| kind == "turnEnded").expect("a turnEnded event");
            assert!(matches_object(&js(&last[ended]), &json!({ "type": "turnEnded", "player": "p2" })));
            assert_eq!(types[ended + 1..ended + 3].to_vec(), vec!["shuffledIn".to_string(), "shuffledIn".to_string()]);
            assert!(last[ended + 1..ended + 3]
                .iter()
                .all(|event| matches!(event, GameEvent::ShuffledIn { player, .. } if *player == P1)));
            assert_eq!(types[ended + 3], "turnStarted");
        }
    }

    // =========================================================================================
    // #90.1 CN-Virus — cast on draw and R58's chain
    // =========================================================================================

    mod n90_1_cn_virus_cast_on_draw_r58_r70 {
        use super::*;

        #[test]
        fn r350_a_drawn_virus_casts_at_once_and_the_draw_repeats_through_the_library_never_its_own_copies() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-chain",
                "p1": { "library": [VIRUS, VIRUS, "core-005"], "hand": [] },
                "p2": { "hand": ["core-005"] },
            }));

            s.start_turn();

            // Two casts, then the Stockpile to hand; not one copy went in to be drawn.
            assert_eq!(damage_to(&s, "hero-p1"), vec![1, 1]);
            assert_eq!(def_ids(&s.hand(Some(P1))), vec!["core-005".to_string()]);
            assert!(shuffled_in(&s, Some(P1)).is_empty());
            assert_eq!(library_of(&s, P1).len(), 0);

            s.end_turn();

            assert_eq!(viruses_in(&s, P1), 4);
        }

        #[test]
        fn r350_a_lone_virus_no_longer_feeds_its_own_chain_the_repeated_draw_finds_the_library_empty() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-lone",
                "p1": { "library": [VIRUS], "hand": [] },
                "p2": { "hand": ["core-005"] },
            }));

            s.start_turn();

            // 1 from the virus, then §2.4's first fatigue draw for 1.
            s.expect_events(json!(["drawn", "damage", "fatigue", "damage"]));
            s.expect_health(P1, 28);
        }

        #[test]
        fn r58_the_chain_stops_at_the_cap_and_the_next_virus_sits_in_hand_uncast() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-chain-cap",
                "p1": { "library": vec![VIRUS; CHAIN_CAP + 2], "hand": [] },
                "p2": { "hand": ["core-005"] },
            }));

            s.start_turn();

            assert_eq!(damage_to(&s, "hero-p1").len(), CHAIN_CAP);
            s.expect_health(P1, 30 - CHAIN_CAP as i32);
            // 22 in the library: 20 cast, the 21st drawn uncast, one left.
            assert_eq!(def_ids(&s.hand(Some(P1))), vec![VIRUS.to_string()]);
            assert_eq!(library_of(&s, P1).len(), 1);
            assert_eq!(armed_copies(&s), CHAIN_CAP);
        }

        #[test]
        fn r70_every_cast_counts_as_a_card_played_so_the_turn_log_sees_the_whole_chain() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-chain-played",
                "p1": { "library": vec![VIRUS; CHAIN_CAP + 1], "hand": [] },
                "p2": { "hand": ["core-005"] },
            }));

            s.start_turn();

            assert_eq!(s.state().players[P1].turn_log.cards_played, CHAIN_CAP as i32);
            assert_eq!(s.state().players[P1].turn_log.played_ids.len(), CHAIN_CAP);
            // A cast is free (R70): the mana the start of turn refreshed is untouched.
            s.expect_mana(P1, 4);
        }

        #[test]
        fn every_cast_virus_ends_in_the_graveyard_r11_none_of_them_gone_and_each_one_s_copies_go_in_at_the_end() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-chain-graveyard",
                "p1": { "library": vec![VIRUS; CHAIN_CAP + 1], "hand": [] },
                "p2": { "hand": ["core-005"] },
            }));

            s.start_turn();

            assert_eq!(s.pile(P1, "graveyard").iter().filter(|card| card.def_id == VIRUS).count(), CHAIN_CAP);
            assert_eq!(s.pile(P1, "exile").len(), 0);

            s.end_turn();

            // 20 casts × 2 copies, into a library that held nothing after the 21st draw.
            assert_eq!(viruses_in(&s, P1), CHAIN_CAP * 2);
            assert_eq!(library_of(&s, P1).len(), CHAIN_CAP * 2);
        }
    }

    mod n90_1_cn_virus_radiant {
        use super::*;

        #[test]
        fn r275_take_2_damage_3_copies_both_numbers_scale_on_the_radiant_face() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-radiant-cry",
                "p1": { "hand": [{ "def": VIRUS, "radiant": true }, "core-005"], "library": filler(3) },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(VIRUS, json!({}));

            s.expect_health(P1, 28);
            assert_eq!(damage_to(&s, "hero-p1"), vec![2]);
            s.expect_health(P2, 30);

            s.end_turn();

            assert_eq!(viruses_in(&s, P1), 3);
        }

        #[test]
        fn s4_4_the_radiant_2_is_one_damage_instance_1_armor_leaves_1_and_the_copies_still_land() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-radiant-armor",
                "p1": { "hand": [{ "def": VIRUS, "radiant": true }, "core-005"], "library": filler(3), "armor": 1 },
                "p2": { "hand": ["core-005"] },
            }));

            s.play(VIRUS, json!({}));

            s.expect_health(P1, 29);
            assert_eq!(damage_to(&s, "hero-p1"), vec![1]);

            s.end_turn();

            assert_eq!(viruses_in(&s, P1), 3);
        }

        #[test]
        fn r57_a_radiant_virus_breeds_radiant_viruses_and_they_cast_the_radiant_face_when_drawn() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-radiant-chain",
                "p1": { "hand": [{ "def": VIRUS, "radiant": true }, "core-005"], "library": [], "health": 50 },
                "p2": { "hand": ["core-005"], "library": filler(4) },
            }));

            s.play(VIRUS, json!({})).end_turn();

            // Three Radiant copies went in; p2's turn is running.
            assert_eq!(library_of(&s, P1).len(), 3);
            assert!(library_of(&s, P1).iter().all(|card| card.radiant));

            // Back to p1: the draw casts all three, 2 each, arming 3 × 3 Radiant copies, and the repeat
            // after the third finds the library empty (§2.4's first fatigue, 1).
            s.end_turn();
            assert_eq!(damage_to(&s, "hero-p1"), vec![2, 2, 2, 2, 1]);
            assert_eq!(armed_copies(&s), 3);
            assert!(s.state().delayed.iter().all(|entry| entry.resume.radiant));
        }

        #[test]
        fn s2_5_r59_a_radiant_chain_on_a_30_health_hero_is_lethal_before_r58_s_cap_15_casts_of_2() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-090-1-radiant-lethal",
                "p1": { "library": vec![json!({ "def": VIRUS, "radiant": true }); 16], "hand": [] },
                "p2": { "hand": ["core-005"] },
            }));

            s.start_turn();

            assert_eq!(damage_to(&s, "hero-p1"), vec![2; 30 / 2]);
            assert_eq!(s.state().result.as_ref().map(|result| result.winner), Some(Winner::P2));
        }
    }

    // =========================================================================================
    // §5.1: the token is in no random pool
    // =========================================================================================

    /// TS `query(args).map((card) => card.id)`: the ids `catalog.query` answers (§5.1).
    fn query_ids(args: Value) -> Vec<String> {
        crate::query::query(&json_as::<CatalogQueryArgs>(args))
            .iter()
            .map(|card| card.id.clone())
            .collect()
    }

    mod n90_1_cn_virus_s5_1_pools {
        use super::*;

        #[test]
        fn s5_1_no_random_pool_offers_the_token_and_the_1_cost_spell_pool_does_not_either() {
            crate::register_all();
            assert!(!query_ids(json!({})).contains(&VIRUS.to_string()));
            assert!(!query_ids(json!({ "type": "Spell", "cost": 1 })).contains(&VIRUS.to_string()));
            // Reachable only by a query that asks for tokens or names it.
            assert!(query_ids(json!({ "tags": ["Token"] })).contains(&VIRUS.to_string()));
            assert_eq!(query_ids(json!({ "defId": VIRUS })), vec![VIRUS.to_string()]);
        }

        #[test]
        fn s5_1_n90_itself_is_an_ordinary_catalog_card_and_stays_in_the_pool() {
            crate::register_all();
            assert!(query_ids(json!({})).contains(&INJECTION.to_string()));
            // Both carry the CN tag (§8), which is how a CN-tagged pool finds #90 but never the token.
            assert!(query_ids(json!({ "tags": ["CN"] })).contains(&INJECTION.to_string()));
            assert!(!query_ids(json!({ "tags": ["CN"] })).contains(&VIRUS.to_string()));
        }
    }
}
