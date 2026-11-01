//! Stinkyscape #143 Book Launcher (SPEC §8.10 row 143). Unit 0/6 → 0/12, Weapon, cost 3, Epic.
//!   Base:    "Starship Piece, Equip
//!            Activate: Choose a target. If it's an enemy, cast a random harmful Book, on it if it can
//!            be. If it's an ally, cast a random helpful Book, on it if it can be."
//!   Radiant: "… a random Radiant harmful Book … a random Radiant helpful Book …"
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! One Activate, once a turn (R384), since #2657: a declared Unit or hero on either side (R81). The side
//! of the target picks the list: an enemy (one the opponent controls, or their hero) a harmful Book, an
//! ally (one of yours, or your hero) a helpful one. R1862006: the harmful Books are those whose target
//! is the enemy's to suffer, Blood, Flame, Wildfire and Nerf; the helpful ones, Heal, Stats and Buff;
//! Vital Kill, which sets a hero to 13 and ends its caster's turn, is in neither, nor is a Book with no
//! target. R1862004: `cast_random_at` casts one of the list, free and counted as played by its
//! controller (R70, R452), aimed at the chosen target for its first target where it may take it
//! (R946), and with `targetEnemies` (R656) a pick the Book makes itself goes to the enemy when it harms
//! and to your side when it helps. Radiant: the Book is cast Radiant. Through v0.5.9f it had two
//! Activates, one for each side, counted apart (`script_v0_5_9f`, kept, R767000).

use jackioh_engine::effects::cast_random_at;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-143";

/// R1862006: Books of Blood, Flame and Wildfire (C #12, #16, #55) and of Nerf (C+ #72).
const HARMFUL_BOOKS: [&str; 4] = ["classic-012", "classic-016", "classic-055", "classicplus-072"];
/// R1862006: Books of Heal (C #3), Stats and Buff (C+ #57, #71).
const HELPFUL_BOOKS: [&str; 3] = ["classic-003", "classicplus-057", "classicplus-071"];

/// "Cast a random `books` Book on" the chosen target.
fn cast_at(ctx: &EffectContext<'_>, books: &'static [&'static str], radiant: bool) -> Vec<Effect> {
    let Some(aim) = ctx.targets.first().map(option_key) else {
        return Vec::new();
    };
    vec![cast_random_at(
        json_as(json!({ "query": { "defId": books }, "radiant": radiant, "targetEnemies": true })),
        aim,
    )]
}

/// Whether the chosen target is an enemy of the activator: the opponent's hero, or a card they
/// control now.
fn chose_an_enemy(ctx: &EffectContext<'_>) -> bool {
    match ctx.targets.first() {
        Some(Selection::Hero { player }) => *player != ctx.controller,
        Some(Selection::Instance { instance_id }) => {
            find_instance(&*ctx.state, instance_id).is_some_and(|card| card.controller != ctx.controller)
        }
        _ => false,
    }
}

/// "Choose a target. If it's an enemy, cast a random harmful Book …; if it's an ally, a helpful one."
fn launch(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: "launchBook".into(),
        label: "Cast a random Book on a target: a harmful one on an enemy, a helpful one on an ally".into(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "any", "of": ["unit", "hero"] }),
        )],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            let books: &'static [&'static str] = if chose_an_enemy(ctx) {
                &HARMFUL_BOOKS
            } else {
                &HELPFUL_BOOKS
            };
            cast_at(ctx, books, radiant)
        }),
    }
}

fn launcher(radiant: bool) -> Script {
    Script {
        activations: vec![launch(radiant)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: launcher(false),
        radiant: launcher(true),
    }
}

/// R767000: one of v0.5.9f's two Activates: "Cast a random `books` Book on" a target of `target`.
fn launch_v0_5_9f(
    id: &str,
    label: &str,
    target: TargetDecl,
    books: &'static [&'static str],
    radiant: bool,
) -> ActivationDecl {
    ActivationDecl {
        id: id.into(),
        label: label.into(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![target],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| cast_at(ctx, books, radiant)),
    }
}

fn launcher_v0_5_9f(radiant: bool) -> Script {
    let harmful = TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "hero"] }));
    let helpful = TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit", "hero"] }))
    };
    Script {
        activations: vec![
            launch_v0_5_9f(
                "harmfulBook",
                "Cast a random harmful Book on an enemy",
                harmful,
                &HARMFUL_BOOKS,
                radiant,
            ),
            launch_v0_5_9f(
                "helpfulBook",
                "Cast a random helpful Book on an ally",
                helpful,
                &HELPFUL_BOOKS,
                radiant,
            ),
        ],
        ..Script::default()
    }
}

/// R767000: through v0.5.9f it had two Activates, a harmful Book at an enemy and a helpful one at an
/// ally, each once a turn and counted apart; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_9f() -> CardScripts {
    CardScripts {
        base: launcher_v0_5_9f(false),
        radiant: launcher_v0_5_9f(true),
    }
}

// Stinkyscape #143 Book Launcher — SPEC §8.10 row 143, BUILD M12 row S 143: its one Activate, once a
// turn, casts a random harmful Book at a chosen enemy or a random helpful Book at a chosen ally, free
// (R70), by rng picks that replay; radiant 0/12 and the Books Radiant.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) 4/4.

    fn board(seed: &str, radiant: bool) -> Scenario {
        weapon_board(
            seed,
            json!({
                "field": [{ "def": ID, "radiant": radiant }, { "def": VANILLA, "damage": 2 }],
                "health": 20,
            }),
            json!({ "field": [MENACE], "health": 20 }),
        )
    }

    fn at(ability: &str, target: &str) -> Value {
        json!({ "ability": ability, "targets": pick::id(target) })
    }

    /// The one Activate at `target`.
    fn launch_at(target: &str) -> Value {
        at("launchBook", target)
    }

    fn cast(s: &Scenario) -> Vec<(PlayerId, String, i32)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed {
                    player,
                    def_id,
                    cost_paid,
                    ..
                } => Some((*player, def_id.clone(), *cost_paid)),
                _ => None,
            })
            .collect()
    }

    fn touched(s: &Scenario, id: &str) -> bool {
        let named = format!("\"{id}\"");
        s.last_events()
            .iter()
            .filter(|event| {
                !matches!(
                    event,
                    GameEvent::Activated { .. }
                        | GameEvent::CardPlayed { .. }
                        | GameEvent::CardAnnounced { .. }
                )
            })
            .any(|event| crate::js(event).to_string().contains(&named))
    }

    fn aims(ability: &str, side: PlayerId, lane: i32, books: &[&str]) {
        for seed in ["s143-a", "s143-b", "s143-c", "s143-d"] {
            let mut s = board(seed, false);
            let target = s.unit_at(side, lane).id;
            s.activate(ID, at(ability, &target));
            let played = cast(&s);
            assert_eq!(played.len(), 1, "{seed}: {played:?}");
            let (player, def_id, cost_paid) = &played[0];
            assert_eq!((*player, *cost_paid), (P1, 0), "{seed}");
            assert!(books.contains(&def_id.as_str()), "{seed}: {def_id}");
            assert!(touched(&s, &target), "{seed}: {def_id} missed {target}");
            assert!(s.state().pending.is_none(), "{seed}: a random cast asks nobody");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r1862006_an_enemy_target_takes_a_harmful_book() {
            aims("launchBook", P2, 1, &HARMFUL_BOOKS);
        }

        #[test]
        fn r1862006_an_ally_target_takes_a_helpful_book() {
            aims("launchBook", P1, 2, &HELPFUL_BOOKS);
        }

        #[test]
        fn r1862006_the_enemy_hero_takes_a_harmful_book_and_yours_a_helpful_one() {
            for (hero, books) in [("p2", &HARMFUL_BOOKS[..]), ("p1", &HELPFUL_BOOKS[..])] {
                let mut s = board("s143-heroes", false);
                let aim = json!({ "ability": "launchBook", "targets": [{ "pick": "hero", "player": hero }] });
                s.activate(ID, aim);
                let (_, def_id, _) = cast(&s).first().cloned().expect("a cast");
                assert!(books.contains(&def_id.as_str()), "{hero}: {def_id}");
            }
        }

        #[test]
        fn r384_once_a_turn_whichever_side_it_aimed_at() {
            let mut s = board("s143-once", false);
            let mine = s.unit_at(P1, 2).id;
            let theirs = s.unit_at(P2, 1).id;
            s.activate(ID, launch_at(&theirs));
            s.expect_refused_with(|s| s.activate(ID, launch_at(&mine)), "already been used");
        }

        #[test]
        fn r1751000_the_two_activates_kept_for_v0_5_9f_are_counted_apart() {
            crate::under_kept(ID, script_v0_5_9f(), || {
                let mut s = board("s143-apart", false);
                let mine = s.unit_at(P1, 2).id;
                let theirs = s.unit_at(P2, 1).id;
                s.expect_refused(|s| s.activate(ID, at("harmfulBook", &mine)));
                s.expect_refused(|s| s.activate(ID, at("helpfulBook", &theirs)));
                s.activate(ID, at("harmfulBook", &theirs));
                s.activate(ID, at("helpfulBook", &mine));
                s.expect_refused_with(
                    |s| s.activate(ID, at("harmfulBook", &theirs)),
                    "already been used",
                );
                s.expect_refused_with(|s| s.activate(ID, at("helpfulBook", &mine)), "already been used");
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862004_the_book_is_cast_radiant() {
            let mut s = board("s143-radiant", true);
            let theirs = s.unit_at(P2, 1).id;
            s.activate(ID, launch_at(&theirs));
            let (_, def_id, _) = cast(&s).first().cloned().expect("a cast");
            let book = s
                .pile(P1, "graveyard")
                .into_iter()
                .find(|card| card.def_id == def_id)
                .expect("the Book landed in its caster's graveyard");
            assert!(book.radiant, "{def_id}");
        }

        #[test]
        fn its_health_is_doubled() {
            let s = board("s143", true);
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, 12));
        }
    }
}
