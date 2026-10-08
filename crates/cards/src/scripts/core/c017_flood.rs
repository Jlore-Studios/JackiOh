//! #17 Flood (SPEC §8.1): "Bounce all units on both sides", radiant "Choose one: bounce all units,
//! bounce all enemy units, destroy all enemy units; then draw 1". Engine cell: "Tokens vanish on
//! bounce; hand cap burns extras".
//!
//! The radiant cell restates the whole effect, so the base "bounce all units on both sides" survives
//! only as one of the three options (§8 Conventions).
//!
//! Neither the vanishing nor the burning is this card's business: §6.3 Bounce goes through
//! `effects/move_.rs`, where a unit token ceases to exist instead of reaching a hand (R11) and a full
//! hand burns the card to the graveyard (R4, §2.4).
//!
//! R81: "the targets and modes a card's script declares travel in the `play` action … so Glowy Jelly
//! Bean's hand card and Silly Silas's direction are chosen with the play and never pause resolution".
//! Flood's "choose one" is therefore a declared `modes` and NOT a `PendingChoice`: the pick arrives in
//! `ctx.modes`, which `chosen_options` reads (after any prompt mode pick, so one helper covers both).
//!
//! The sweeps are `bounce_all` (effects/move_.rs) and `destroy_all` (effects/destroy.rs), over a board
//! scope whose rows default to the units. `bounce_all` sends each card through the same body as
//! `bounce`, so R11 and R4 hold card by card; `destroy_all` marks every match and stops, exactly like
//! `destroy`, so Indestructible survives (R46) and everything dies in the one state check that
//! follows (R59).

use jackioh_engine::effects::{bounce_all, chosen_options, destroy_all, draw};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-017";

/// The three options, word for word from §8. They are the card's public interface: `legal_actions`
/// offers these strings, the client shows them and the hook below matches on them, so the
/// declaration and the hook must never drift apart — hence one constant for both.
const BOUNCE_ALL: &str = "bounce all units";
const BOUNCE_ENEMY: &str = "bounce all enemy units";
const DESTROY_ENEMY: &str = "destroy all enemy units";

fn modes() -> Vec<ModeDecl> {
    vec![ModeDecl {
        kind: PromptKind::Mode,
        options: vec![BOUNCE_ALL.to_string(), BOUNCE_ENEMY.to_string(), DESTROY_ENEMY.to_string()],
    }]
}

/// "Bounce all units on both sides" — the base text, and radiant's first option.
fn bounce_both_sides() -> Effect {
    bounce_all(json_as(json!({ "side": "any" })))
}

/// The option the play carried. `why_choices_refused` (play_choices.rs) already refuses a play that
/// names no mode for a declared one, so the fallback is unreachable in a legal game; it is the first
/// option rather than "do nothing" because "choose one" is a mandatory choice, not an optional rider.
fn picked_mode(ctx: &mut EffectContext<'_>) -> String {
    chosen_options(ctx)
        .into_iter()
        .next()
        .unwrap_or_else(|| BOUNCE_ALL.to_string())
}

fn chosen_effect(ctx: &mut EffectContext<'_>) -> Effect {
    let picked = picked_mode(ctx);
    if picked == DESTROY_ENEMY {
        return destroy_all(json_as(json!({ "side": "enemy", "rows": ["units"] })));
    }
    if picked == BOUNCE_ENEMY {
        return bounce_all(json_as(json!({ "side": "enemy" })));
    }
    bounce_both_sides()
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| vec![bounce_both_sides()])),
        ..Script::default()
    };
    let radiant = Script {
        modes: modes(),
        // "; then draw 1": the draw follows whichever mode resolved, on all three of them. Its count is
        // the declared number `draw` (R386).
        cry: Some(hook(|ctx| {
            let count = param(&*ctx, "draw");
            vec![chosen_effect(ctx), draw(json_as(json!({ "count": count })))]
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// #17 Flood — SPEC §8.1 row 17, BUILD M4-T4 row 17.
//
// Must-pass (M4-T4): "Bounces both sides, tokens vanish, hand cap burns; radiant three modes each
// tested plus draw 1".
//
// Engine cell: "Tokens vanish on bounce; hand cap burns extras" — R11 and R4.
//
// R81 is why no test here calls `answer()`: the radiant "choose one" is a DECLARED mode, so it
// travels in the play action (`play(..., { modes })`) and never opens a `PendingChoice`. The strings
// are the same constants the script declares; a drift between the two would fail `refuse_modes`
// (play_choices.rs) rather than silently pick the first option.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const BOUNCE_ALL: &str = "bounce all units";
    const BOUNCE_ENEMY: &str = "bounce all enemy units";
    const DESTROY_ENEMY: &str = "destroy all enemy units";

    /// HARNESS GAP: `SideSetup.hand` takes no `radiant` flag; see c016_hit_job.rs for the note.
    fn make_radiant<'a>(s: &'a mut Scenario, card: &str) -> &'a mut Scenario {
        let id = s.card(card).id.clone();
        match find_instance_mut(s.state_mut(), &id) {
            Some(instance) => instance.radiant = true,
            None => panic!("no instance {id}"),
        }
        s
    }

    fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        match s.unit(player, lane) {
            Some(found) => found,
            None => panic!("no unit in {player} unit lane {lane}"),
        }
    }

    /// Ten distinct cards, to sit a hand exactly on HAND_CAP once Flood has left it (R4).
    const TEN_CARDS: [&str; 10] = [
        "core-001", "core-002", "core-005", "core-006", "core-008", "core-010", "core-011", "core-012", "core-015",
        "core-020",
    ];

    mod n17_flood_base {
        use super::*;

        #[test]
        fn bounces_every_unit_on_both_sides_to_its_controller_s_hand_s6_3_bounce_r747() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-017", "core-010"], "field": ["core-012"], "mana": 4 },
                "p2": { "field": ["core-019", "core-020"] }
            }));
            let mine = unit_at(&s, PlayerId::P1, 1);
            let theirs1 = unit_at(&s, PlayerId::P2, 1);
            let theirs2 = unit_at(&s, PlayerId::P2, 2);

            s.play("core-017", json!({}));

            s.expect_in_zone(&mine, "hand");
            s.expect_in_zone(&theirs1, "hand");
            s.expect_in_zone(&theirs2, "hand");
            // Both unit rows are empty afterwards.
            for lane in 1..=5 {
                assert!(s.unit(PlayerId::P1, lane).is_none());
                assert!(s.unit(PlayerId::P2, lane).is_none());
            }
            // R747: each card goes to its CONTROLLER's hand, never the caster's.
            assert!(s.pile(PlayerId::P1, "hand").iter().any(|card| card.id == mine.id));
            assert!(
                s.pile(PlayerId::P2, "hand")
                    .iter()
                    .map(|card| card.id.clone())
                    .collect::<Vec<_>>()
                    .contains(&theirs1.id)
            );
        }

        #[test]
        fn r11_a_bounced_unit_token_ceases_to_exist_and_reaches_no_hand() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-017", "core-010"], "field": ["core-t-rush"], "mana": 4 },
                "p2": { "field": ["core-019"] }
            }));
            let token = unit_at(&s, PlayerId::P1, 1);
            let real = unit_at(&s, PlayerId::P2, 1);

            s.play("core-017", json!({}));

            s.expect_in_zone(&token, "gone");
            assert!(!s.pile(PlayerId::P1, "hand").iter().any(|card| card.id == token.id));
            assert!(!s.pile(PlayerId::P1, "graveyard").iter().any(|card| card.id == token.id));
            // The real card beside it still bounces normally.
            s.expect_in_zone(&real, "hand");
        }

        #[test]
        fn r4_a_bounced_unit_is_burned_to_the_graveyard_when_its_controller_s_hand_is_full() {
            crate::register_all();
            let mut hand = vec![json!("core-017")];
            hand.extend(TEN_CARDS.iter().map(|id| json!(id)));
            let mut s = scenario(json!({
                // Eleven cards: playing Flood leaves exactly HAND_CAP (10) behind, so the bounce has no
                // room.
                "p1": { "hand": hand, "field": ["core-012"], "mana": 4 },
                "p2": { "field": ["core-019"] }
            }));
            let mine = unit_at(&s, PlayerId::P1, 1);
            let theirs = unit_at(&s, PlayerId::P2, 1);

            s.play("core-017", json!({}));

            assert_eq!(s.pile(PlayerId::P1, "hand").len(), 10);
            s.expect_in_zone(&mine, "graveyard");
            s.expect_events(json!(["burned", "enteredGraveyard"]));
            // The opponent's hand is empty, so their unit is not burned: the cap is the entering hand's.
            s.expect_in_zone(&theirs, "hand");
        }

        #[test]
        fn declares_no_mode_the_base_face_bounces_both_sides_unconditionally() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-017", "core-010"], "mana": 4 }, "p2": { "field": ["core-019"] } }));

            s.play("core-017", json!({}));

            s.expect_in_zone("core-019", "hand");
            // No prompt was ever opened: R81's declared choices never pause resolution.
            assert!(s.state().pending.is_none());
        }
    }

    mod n17_flood_radiant {
        use super::*;

        #[test]
        fn r81_mode_bounce_all_units_bounces_both_sides_then_draws_1() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-017"], "field": ["core-012"], "library": ["core-010"], "mana": 4 },
                "p2": { "field": ["core-019", "core-020"] }
            }));
            make_radiant(&mut s, "core-017");
            let mine = unit_at(&s, PlayerId::P1, 1);
            let theirs = unit_at(&s, PlayerId::P2, 1);

            s.play("core-017", json!({ "modes": [BOUNCE_ALL] }));

            s.expect_in_zone(&mine, "hand");
            s.expect_in_zone(&theirs, "hand");
            s.expect_in_zone("core-010", "hand");
            assert_eq!(s.pile(PlayerId::P1, "library").len(), 0);
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn r81_mode_bounce_all_enemy_units_spares_your_own_units_then_draws_1() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-017"], "field": ["core-012", "core-025"], "library": ["core-010"], "mana": 4 },
                "p2": { "field": ["core-019", "core-020"] }
            }));
            make_radiant(&mut s, "core-017");
            let mine1 = unit_at(&s, PlayerId::P1, 1);
            let mine2 = unit_at(&s, PlayerId::P1, 2);
            let theirs1 = unit_at(&s, PlayerId::P2, 1);
            let theirs2 = unit_at(&s, PlayerId::P2, 2);

            s.play("core-017", json!({ "modes": [BOUNCE_ENEMY] }));

            s.expect_in_zone(&mine1, "field");
            s.expect_in_zone(&mine2, "field");
            s.expect_in_zone(&theirs1, "hand");
            s.expect_in_zone(&theirs2, "hand");
            s.expect_in_zone("core-010", "hand");
        }

        #[test]
        fn r81_mode_destroy_all_enemy_units_destroys_them_instead_of_bouncing_them_then_draws_1() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-017"], "field": ["core-012"], "library": ["core-010"], "mana": 4 },
                "p2": { "field": ["core-019", "core-020"] }
            }));
            make_radiant(&mut s, "core-017");
            let mine = unit_at(&s, PlayerId::P1, 1);
            let theirs1 = unit_at(&s, PlayerId::P2, 1);
            let theirs2 = unit_at(&s, PlayerId::P2, 2);

            s.play("core-017", json!({ "modes": [DESTROY_ENEMY] }));

            s.expect_in_zone(&theirs1, "graveyard");
            s.expect_in_zone(&theirs2, "graveyard");
            s.expect_in_zone(&mine, "field");
            s.expect_in_zone("core-010", "hand");
            // R59: one effect marks both, so both deaths land in the same state check.
            s.expect_events(json!(["destroyed", "destroyed"]));
        }

        #[test]
        fn r46_the_destroy_mode_leaves_an_indestructible_enemy_standing_and_still_draws_1() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-017"], "library": ["core-010"], "mana": 4 },
                "p2": { "field": [{ "def": "core-025", "radiant": true }, "core-019"] }
            }));
            make_radiant(&mut s, "core-017");
            let indestructible = unit_at(&s, PlayerId::P2, 1);
            let mortal = unit_at(&s, PlayerId::P2, 2);

            s.play("core-017", json!({ "modes": [DESTROY_ENEMY] }));

            s.expect_in_zone(&indestructible, "field");
            s.expect_in_zone(&mortal, "graveyard");
            s.expect_in_zone("core-010", "hand");
        }

        #[test]
        fn r11_the_bounce_modes_still_make_a_unit_token_cease_to_exist() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-017"], "library": ["core-010"], "mana": 4 },
                "p2": { "field": ["core-t-rush"] }
            }));
            make_radiant(&mut s, "core-017");
            let token = unit_at(&s, PlayerId::P2, 1);

            s.play("core-017", json!({ "modes": [BOUNCE_ENEMY] }));

            s.expect_in_zone(&token, "gone");
        }

        #[test]
        fn r386_an_upgrade_draws_2_and_a_degrade_finds_the_draw_at_its_floor_of_1() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": ["core-017"], "library": ["core-010", "core-011"], "mana": 4 },
                "p2": { "field": ["core-t-rush"] }
            }));
            make_radiant(&mut s, "core-017");
            assert!(!crate::can_degrade_number(&s, "core-017", "draw"));
            assert_eq!(crate::upgrade_number(&mut s, "core-017", "draw"), 2);

            s.play("core-017", json!({ "modes": [BOUNCE_ENEMY] }));

            s.expect_in_zone("core-010", "hand");
            s.expect_in_zone("core-011", "hand");
        }
    }
}
