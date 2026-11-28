//! SPEC §8.14 FvH #30 Big Book. (3) Spell, Epic. Built by S03 (#5253).
//!   Base:    "Cast {casts|random Book|random Books}, with random targets." — 5
//!   Radiant: "Cast {casts|random Radiant Book|random Radiant Books}. Each aims at enemies when it harms and
//!            at your side when it helps." — 5
//!
//! M #60 Eschews' `cast_random` of the declared `casts` (R386) from the Book tag. R5253003: the Books of
//! every set that ships (R380), FvH #23–#29 among them once their set ships (R1420); Big Book prints no
//! Book tag, and a card never generates itself anyway (R387). Each cast is a free play that resolves
//! before the next (R70, R113), X its caster's current mana (R453). The base face makes every choice at
//! random (R452); the Radiant face's are Radiant and aim each target pick by its declaration, enemies for
//! harm and its caster's side for help (R656). FvH #23–#29 declare no targets, so they cast whole on
//! either face.

use jackioh_engine::effects::{CastRandomArgs, CastRandomCount, CastRandomQuery, cast_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-030";

/// R5253003: every Book-tagged card of the sets that ship.
fn books() -> CatalogQueryArgs {
    json_as(json!({ "tags": ["Book"] }))
}

fn big_book(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![cast_random(CastRandomArgs {
                query: CastRandomQuery::Fixed(Box::new(books())),
                count: Some(CastRandomCount::Fixed(param(&*ctx, "casts"))),
                radiant: Some(radiant),
                // R656: only the Radiant face aims; the base face's picks are truly random (R452).
                target_enemies: radiant.then_some(true),
                afterward: None,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: big_book(false),
        radiant: big_book(true),
    }
}

// FvH #30 Big Book — SPEC §8.14 row 30 (R5253003, R452, R656, R387, R113).
#[cfg(test)]
mod tests {
    use super::{ID, books};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // Stockpile
    const ALLY: &str = "stand-in (1) 3/3";

    fn seeds() -> Vec<String> {
        (1..=8).map(|n| format!("big-book-{n}")).collect()
    }

    /// p1 holds the Big Book (`radiant`), with two Units on each side for the casts to aim at.
    fn holding(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [ALLY, ALLY],
                "library": filler(5),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [ALLY, ALLY], "library": filler(5) },
        }))
    }

    /// Plays the Big Book; returns its instance id. A cast may hand the opponent a choice of their own,
    /// theirs to make and never answered at random (R452 drives only the caster's): it is answered here
    /// so the casts go on.
    fn play_book(s: &mut Scenario) -> String {
        let book = s.card(ID).id.clone();
        s.play(ID, json!({}));
        let mut guard = 0;
        while guard < 20 {
            let Some(pending) = s.state().pending.clone() else {
                break;
            };
            if pending.player_id != P2 {
                break;
            }
            let take = std::cmp::max(1, pending.min) as usize;
            let keys: Vec<String> = pending.options.iter().take(take).map(|option| option.key.clone()).collect();
            s.answer(json!(keys));
            guard += 1;
        }
        book
    }

    /// Whether a cast ended the game, which stops the rest of the chain.
    fn game_over(s: &Scenario) -> bool {
        s.state().result.is_some()
    }

    /// One of p1's casts, as its announcement names it.
    struct Cast {
        instance_id: String,
        def_id: String,
        cost_paid: i32,
        targets: Vec<String>,
    }

    /// Everything p1 announced after the Big Book itself: its casts, since no Book casts a card.
    fn casts(s: &Scenario, book: &str) -> Vec<Cast> {
        s.events_of(GameEventType::CardAnnounced)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::CardAnnounced { player: P1, instance_id, def_id, cost_paid, targets, .. } => {
                    Some(Cast { instance_id, def_id, cost_paid, targets })
                }
                _ => None,
            })
            .filter(|cast| cast.instance_id != book)
            .collect()
    }

    fn is_book(def_id: &str) -> bool {
        crate::card_def(def_id).tags.contains(&Tag::Book)
    }

    fn pool() -> Vec<String> {
        crate::query::query(&books()).into_iter().map(|def| def.id.clone()).collect()
    }

    #[test]
    fn r5253003_its_books_are_every_shipped_sets_and_the_sets_own_once_it_ships() {
        crate::register_all();
        let own: Vec<String> = (23..=29).map(|n| format!("felinorsvshumans-0{n}")).collect();
        let held = pool();
        assert!(held.len() > 5, "{held:?}");
        assert!(held.iter().all(|def_id| is_book(def_id)), "{held:?}");
        assert!(!held.iter().any(|def_id| own.contains(def_id)), "{held:?}");
        assert!(!crate::card_def(ID).tags.contains(&Tag::Book));
        let _preview = preview_sets(&[SetName::FelinorsVsHumans]);
        let shipped = pool();
        assert!(!shipped.contains(&ID.to_string()));
        for def_id in nonempty(own, "the set's Books") {
            assert!(shipped.contains(&def_id), "{def_id}");
        }
    }

    #[test]
    fn r5253003_once_the_set_ships_its_books_cast_whole() {
        crate::register_all();
        let _preview = preview_sets(&[SetName::FelinorsVsHumans]);
        let mut own: Vec<String> = Vec::new();
        for seed in seeds() {
            let mut s = holding(false, &seed);
            let book = play_book(&mut s);
            if game_over(&s) {
                continue;
            }
            let cast = casts(&s, &book);
            assert_eq!(cast.len(), 5, "{seed}");
            assert!(cast.iter().all(|cast| is_book(&cast.def_id)), "{seed}");
            own.extend(cast.into_iter().map(|cast| cast.def_id).filter(|def_id| def_id.starts_with("felinorsvshumans-")));
        }
        assert!(!own.is_empty(), "no seed cast one of the set's own Books");
    }

    mod base {
        use super::*;

        #[test]
        fn r452_r387_casts_five_different_random_books_free_with_no_prompt_of_yours() {
            let mut checked = 0;
            for seed in seeds() {
                let mut s = holding(false, &seed);
                let book = play_book(&mut s);
                if game_over(&s) {
                    continue;
                }
                let cast = casts(&s, &book);
                assert_eq!(cast.len(), 5, "{seed}");
                let different: IndexSet<&String> = cast.iter().map(|cast| &cast.def_id).collect();
                assert_eq!(different.len(), 5, "{seed}: R3563000");
                for Cast { def_id, cost_paid, .. } in &cast {
                    assert!(is_book(def_id), "{seed}: {def_id} is a Book");
                    assert_ne!(def_id, ID, "{seed}");
                    assert_eq!(*cost_paid, 0, "{seed}: {def_id}");
                }
                assert!(s.state().pending.is_none(), "{seed}");
                checked += 1;
            }
            assert!(checked > 0);
        }

        #[test]
        fn r386_casts_reads_through_param() {
            let mut s = holding(false, &seeds()[0]);
            step_param(s.card_mut(ID), "casts", -1);
            let book = play_book(&mut s);
            assert!(!game_over(&s));
            assert_eq!(casts(&s, &book).len(), 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn casts_five_radiant_books() {
            let mut checked = 0;
            for seed in seeds() {
                let mut s = holding(true, &seed);
                let book = play_book(&mut s);
                if game_over(&s) {
                    continue;
                }
                let cast = casts(&s, &book);
                assert_eq!(cast.len(), 5, "{seed}");
                let resolved = s.events_of(GameEventType::CardResolved);
                for Cast { instance_id, def_id, .. } in &cast {
                    assert!(is_book(def_id), "{seed}: {def_id}");
                    let radiant = resolved.iter().any(|event| {
                        matches!(event, GameEvent::CardResolved { instance_id: id, radiant: Some(true), .. } if id == instance_id)
                    });
                    assert!(radiant, "{seed}: {def_id} resolved Radiant");
                }
                checked += 1;
            }
            assert!(checked > 0);
        }

        #[test]
        fn r656_each_pick_aims_by_its_declaration_harm_never_at_your_hero_help_never_at_an_enemy() {
            crate::register_all();
            let helpful: IndexSet<String> = crate::query::query(&json_as(json!({ "tags": ["Book"], "aim": "help" })))
                .into_iter()
                .map(|def| def.id.clone())
                .collect();
            let mut aimed: Vec<String> = Vec::new();
            for seed in seeds() {
                let mut s = holding(true, &seed);
                let mut foe: IndexSet<String> = IndexSet::new();
                foe.insert("hero-p2".to_string());
                foe.extend(s.state().players.p2.units.iter().flatten().flatten().map(|card| card.id.clone()));
                let book = play_book(&mut s);
                for Cast { def_id, targets, .. } in casts(&s, &book) {
                    for target in targets {
                        if helpful.contains(&def_id) {
                            assert!(!foe.contains(&target), "{seed}: {def_id} helps {target}");
                        } else {
                            assert_ne!(target, "hero-p1", "{seed}: {def_id} harms your hero");
                        }
                        aimed.push(format!("{def_id} at {target}"));
                    }
                }
            }
            assert!(!aimed.is_empty(), "no cast took a target");
        }
    }
}
