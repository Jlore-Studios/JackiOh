//! SPEC §8.10 Stinkyscape 116 Megabase, a card of the Stinkyscape set.
//!   Base:    "Can't attack, Immune to Spells, Immutable, Untributable, Armor 15\nCan't be in
//!            Defense Position.\nActivate: Choose one: Both players gain 5, 10 or 15 Credits.\nDeath:
//!            Both players gain 100 Credits." (0/20)
//!   Radiant: "… Armor 30 … Activate: Choose one: You gain 5, 10 or 15 Credits, and your opponent gains
//!            5 fewer.\nDeath: Both players gain 200 Credits." (0/40)
//!
//! A Unit, played from hand into an open unit zone. Quickdraw (R640, `StaticFlags.quickdraw` on both
//! faces): it starts in the opening hand. Its printed keywords are the engine's: Can't attack;
//! Immune to Spells; Immutable, so nothing tunes it and its numbers are
//! printed constants here rather than declared (R386); Untributable, so no Tribute takes it (R1220);
//! Armor 15. Can't Defend (R2657000) keeps it out of Defense Position (§4.1). Activate (§6.2, R384),
//! once a turn, three declared modes (R81): each player `gain_market_credits` the chosen 5, 10 or 15
//! (R1840005), the first gain opening the Marketplace (R1853000). Death (§6.2): each player gains 100.
//! Radiant: its Activate gives its controller N and the opponent N − 5; its Death 200 each.

use jackioh_engine::effects::gain_market_credits;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-116";

/// The Activate's id.
const CREDITS: &str = "credits";
/// "Choose one: … gain 5, 10 or 15 Credits": each mode and the Credits it gives.
const CREDIT_MODES: [(&str, i32); 3] = [("5 Credits", 5), ("10 Credits", 10), ("15 Credits", 15)];
/// Radiant: "and your opponent gains 5 fewer".
const OPPONENT_SHORTFALL: i32 = 5;
/// "Death: Both players gain 100 Credits."
const DEATH_CREDITS: i32 = 100;
/// The Radiant face's Death: 200 each.
const RADIANT_DEATH_CREDITS: i32 = 200;

/// Both players' Credits: `own` to its controller, `theirs` to the opponent.
fn both_gain(own: i32, theirs: i32) -> Vec<Effect> {
    vec![
        gain_market_credits(json_as(json!({ "amount": own }))),
        gain_market_credits(json_as(json!({ "amount": theirs, "player": "enemy" }))),
    ]
}

fn credits(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: CREDITS.to_string(),
        label: "Both players gain Credits".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: CREDIT_MODES.iter().map(|(mode, _)| mode.to_string()).collect(),
        }],
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            let Some(amount) = ctx
                .modes
                .first()
                .and_then(|chosen| CREDIT_MODES.iter().find(|(mode, _)| mode == chosen))
                .map(|(_, amount)| *amount)
            else {
                return vec![];
            };
            let theirs = if radiant {
                amount - OPPONENT_SHORTFALL
            } else {
                amount
            };
            both_gain(amount, theirs)
        }),
    }
}

fn face(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        activations: vec![credits(radiant)],
        death: Some(hook(move |_ctx| {
            let each = if radiant {
                RADIANT_DEATH_CREDITS
            } else {
                DEATH_CREDITS
            };
            both_gain(each, each)
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

/// R767000: through v0.5.9e "Can't be in Defense Position" was the static flag `never_defense` on both
/// faces, before R2657000 made it the keyword Can't Defend; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_9e() -> CardScripts {
    crate::never_defense_kept(script(), true, true)
}

// S 116 Megabase — SPEC §8.10 row 116, BUILD M12 row S 116: "played as a Unit from hand into an open
// unit zone, never attacking or in Defense Position, no Spell targeting it; its Activate gives each
// player the chosen 5, 10 or 15 Credits, once a turn; destroyed, each player gains 100; a Tribute
// cannot take it; it starts in the opening hand (R640); radiant 0/40, Armor 30, the opponent gaining 5
// fewer, and 200 each on its Death".
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::{destroy, summon};
    use jackioh_engine::testkit::*;

    use super::{CREDITS, ID, script};
    use crate::{filler, run_as, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const TURTINATOR: &str = "classic-021"; // Activate: Tribute another Unit; deal damage equal to its attack.
    const TRUE_STRIKE: &str = "core-044"; // (1) Spell: deal 4 damage to a target.

    fn landed(radiant: bool, p1: Value) -> Scenario {
        let mut side = json!({ "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4) });
        if let (Some(into), Value::Object(extra)) = (side.as_object_mut(), p1) {
            into.extend(extra);
        }
        let mut s = scenario(json!({ "p1": side, "p2": { "hand": [FILLER], "library": filler(4) } }));
        s.play(ID, json!({ "zone": 2 }));
        s
    }

    fn credits(s: &Scenario) -> (i32, i32) {
        (market_credits_of(s.state(), P1), market_credits_of(s.state(), P2))
    }

    fn activate(s: &mut Scenario, mode: &str) {
        s.activate(ID, json!({ "ability": CREDITS, "modes": [mode] }));
    }

    #[test]
    fn enters_as_a_unit_in_an_open_unit_zone() {
        let s = landed(false, json!({}));
        let base = s.unit_at(P1, 2);
        assert_eq!(base.def_id, ID, "in the unit zone of its lane");
        assert!(s.backrow(P1, 2).is_none());
        let stats = s.stats(&base);
        assert_eq!((stats.attack, stats.health), (0, 20));
        assert_eq!(stats.armor, 15);
        assert!(
            !has_keyword(&stats.keywords, KeywordKind::Animated),
            "no Animated on the base face"
        );
        let def = crate::card_def(ID);
        assert!(
            !def.base.text.contains("Animated") && !def.radiant.text.contains("Animated"),
            "neither face prints Animated"
        );
    }

    #[test]
    fn never_attacks_or_defends() {
        let mut s = landed(false, json!({}));
        s.end_turn().end_turn();
        s.expect_refused(|s| s.attack(ID, "hero"));
        s.expect_refused(|s| s.switch_position(ID));
    }

    #[test]
    fn r1220_turtinator_cannot_tribute_it() {
        let mut s = landed(false, json!({ "field": [TURTINATOR] }));
        let base = s.unit_at(P1, 2);
        s.expect_refused(|s| {
            s.activate(
                TURTINATOR,
                json!({ "tributes": [base.id.clone()], "targets": [{ "pick": "hero", "player": "p2" }] }),
            )
        });
        s.expect_in_zone(&base, "field");
    }

    #[test]
    fn true_strike_cannot_target_it() {
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "library": filler(4) },
            "p2": { "hand": [TRUE_STRIKE, FILLER], "library": filler(4) },
        }));
        // Summoned into p1's open unit zone.
        run_as(&mut s, summon(json_as(json!({ "defId": ID }))), P1);
        let base = s.unit_at(P1, 1);
        assert_eq!(base.def_id, ID);
        s.expect_refused(|s| s.play(TRUE_STRIKE, json!({ "targets": pick::instance(&base) })));
        s.expect_stats(&base, json!({ "health": 20 }));
    }

    #[test]
    fn activate_gives_both_the_chosen_credits_once_a_turn() {
        let mut s = landed(false, json!({}));
        activate(&mut s, "10 Credits");
        assert_eq!(credits(&s), (10, 10));
        assert!(
            s.view(P1).marketplace.is_some(),
            "the first gain opens the Marketplace"
        );
        s.expect_refused(|s| s.activate(ID, json!({ "ability": CREDITS, "modes": ["5 Credits"] })));
        s.end_turn().end_turn();
        activate(&mut s, "15 Credits");
        assert_eq!(credits(&s), (25, 25));
        s.end_turn().end_turn();
        activate(&mut s, "5 Credits");
        assert_eq!(credits(&s), (30, 30));
    }

    #[test]
    fn death_gives_both_100() {
        let mut s = landed(false, json!({}));
        let base = s.unit_at(P1, 2);
        run_as(
            &mut s,
            destroy(json_as(
                json!({ "target": { "of": "instance", "instanceId": base.id } }),
            )),
            P2,
        );
        s.play(FILLER, json!({}));
        s.expect_in_zone(&base, "graveyard");
        assert_eq!(credits(&s), (100, 100));
    }

    #[test]
    fn r640_quickdraw_on_both_faces() {
        crate::register_all();
        let scripts = script();
        for face in [&scripts.base, &scripts.radiant] {
            let flags = face.static_flags.as_ref();
            assert_eq!(flags.and_then(|flags| flags.quickdraw), Some(true));
        }
        // R2657000: Can't Defend, the keyword that took over from the flag.
        for face in [&crate::card_def(ID).base, &crate::card_def(ID).radiant] {
            assert!(has_keyword(&face.keywords, KeywordKind::CantDefend));
        }
        assert!(crate::card_def(ID).tags.contains(&Tag::Quickdraw));

        // Dealt: every opening hand holds it, wherever the shuffle put it.
        let rest = [
            "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-007", "core-008",
            "core-009", "core-010", "core-011", "core-012", "core-013", "core-014", "core-016", "core-017",
            "core-019", "core-020", "core-025", "core-032", "core-036", "core-043", "core-044", "core-053",
        ];
        let mut mine = vec![ID.to_string()];
        mine.extend(rest.iter().map(|id| id.to_string()));
        let mut theirs = vec!["core-026".to_string()];
        theirs.extend(rest.iter().map(|id| id.to_string()));
        for n in 0..20 {
            let options: CreateGameOptions = json_as(json!({
                "seed": format!("megabase-deal-{n}"),
                "decks": [mine, theirs],
            }));
            let begun = begin_game(&create_game(&options));
            assert!(
                begun.state.players.p1.hand.iter().any(|card| card.def_id == ID),
                "{n}: in the opening hand"
            );
        }
    }

    #[test]
    fn radiant_opponent_gains_5_fewer() {
        let mut s = landed(true, json!({}));
        activate(&mut s, "5 Credits");
        assert_eq!(credits(&s), (5, 0));
        s.end_turn().end_turn();
        activate(&mut s, "15 Credits");
        assert_eq!(credits(&s), (20, 10));
    }

    #[test]
    fn radiant_armor_30_and_death_gives_200() {
        let mut s = landed(true, json!({}));
        let base = s.unit_at(P1, 2);
        let stats = s.stats(&base);
        assert_eq!((stats.attack, stats.health, stats.armor), (0, 40, 30));
        run_as(
            &mut s,
            destroy(json_as(
                json!({ "target": { "of": "instance", "instanceId": base.id } }),
            )),
            P2,
        );
        s.play(FILLER, json!({}));
        assert_eq!(credits(&s), (200, 200));
    }
}
