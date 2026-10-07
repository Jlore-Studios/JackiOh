//! Port of `packages/cards/test/control-change-carry.test.ts` (part 27.1).
//!
//! What a card's "you" follows once it changes sides (SPEC §6.2, §8 Conventions: "'Your' means the
//! controller", R30, R171). Found by the polish-4 edge-case hunt (docs/polish/4-edge-cases.md, lens
//! L1); every case here failed before its fix.
//!
//!  - #79 Twinspell's "the next Spell you play gains Echo +1" is a modifier the permanent installed,
//!    and it moves with the permanent to its new controller.
//!  - §6.2: a start-of-turn hook fires on its controller's own turn start. One queued for that
//!    controller does not fire for the player who took the card while the queue was running.

use jackioh_engine::testkit::*;

const STOCKPILE: &str = "core-005";
const VANILLA: &str = "core-008";
const MOTHS: &str = "core-009";
const POINTMASTER: &str = "core-020";
const SEVEN_SEVEN: &str = "core-025";
const GRAVEDIGGER: &str = "core-037";
const KPOP: &str = "core-050";
const TWINSPELL: &str = "core-079";
const MROW: &str = "core-086";
const LIBRARY: [&str; 8] = [
    VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA, VANILLA,
];

/// The harness with the real catalog and every card script registered (TS's `_harness.ts` import
/// ran `registerAll()`; the engine's testkit cannot name the cards crate, so the cards test does).
fn setup(opts: Value) -> Scenario {
    jackioh_cards::register_all();
    scenario(opts)
}

fn at(card: &CardInstance) -> Value {
    json!([{ "pick": "instance", "instanceId": card.id }])
}

fn unit_at(g: &Scenario, player: &'static str, lane: i32) -> CardInstance {
    g.unit(player, lane)
        .unwrap_or_else(|| panic!("setup: {player} should hold a unit in lane {lane}"))
}

fn backrow_at(g: &Scenario, player: &'static str, lane: i32) -> CardInstance {
    g.backrow(player, lane)
        .unwrap_or_else(|| panic!("setup: {player} should hold a backrow card in lane {lane}"))
}

fn echo_next_spell_mods(g: &Scenario, player: PlayerId) -> usize {
    g.state().players[player]
        .mods
        .iter()
        .filter(|m| matches!(m.kind, ModifierKind::EchoNextSpell { .. }))
        .count()
}

fn def_ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

/// R30: a stolen Twinspell's grant is its new controller's
mod r30_a_stolen_twinspells_grant_is_its_new_controllers {
    use super::*;

    #[test]
    fn r30_r12_r171_a_stolen_twinspell_grants_its_echo_to_the_thiefs_next_spell_and_goes_to_its_owners_graveyard_s8_c79()
     {
        let mut g = setup(json!({
            "p1": { "hand": [TWINSPELL, VANILLA], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
            "p2": { "hand": [KPOP, STOCKPILE, VANILLA], "field": [{ "def": VANILLA, "lane": 1 }], "library": LIBRARY },
        }));
        g.play(TWINSPELL, json!({ "zone": 3 }));
        let twinspell = backrow_at(&g, "p1", 3);
        g.end_turn();

        // p2 takes it with K-Pop Fanatic, so no Spell of p2's is in flight when control changes.
        g.play(KPOP, json!({ "targets": at(&twinspell) }));
        g.end_turn();
        assert_eq!(g.state().active, PlayerId::P1);
        g.end_turn();
        // The steal fired at the start of p2's turn, and the modifier went with the card; both seats
        // were told, since `modifierChanged` names the seat (R169).
        assert_eq!(g.state().active, PlayerId::P2);
        assert_eq!(g.card(&twinspell).controller, PlayerId::P2);
        assert_eq!(echo_next_spell_mods(&g, PlayerId::P1), 0);
        assert_eq!(echo_next_spell_mods(&g, PlayerId::P2), 1);
        let changes: Vec<(PlayerId, bool)> = g
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::ModifierChanged { player, added, .. } => Some((*player, *added)),
                _ => None,
            })
            .collect();
        assert_eq!(changes, [(PlayerId::P1, false), (PlayerId::P2, true)]);

        // p2's Stockpile resolves twice (draw 2, twice), and Twinspell goes to its owner's graveyard.
        let before = g.hand("p2").len();
        g.play(STOCKPILE, json!({}));
        assert_eq!(g.hand("p2").len(), before - 1 + 4);
        g.expect_in_zone(&twinspell, "graveyard");
        assert_eq!(echo_next_spell_mods(&g, PlayerId::P2), 0);
    }
}

/// §6.2: a start-of-turn hook belongs to its controller's turn
mod s6_2_a_start_of_turn_hook_belongs_to_its_controllers_turn {
    use super::*;

    #[test]
    fn r62_r153_s6_2_a_start_of_turn_queue_belongs_to_its_controller_the_moths_mrows_death_takes_mid_queue_changes_sides_and_the_gravedigger_behind_it_still_fires_for_p1()
     {
        let mut g = setup(json!({
            "p1": {
                "hand": [VANILLA],
                "field": [{ "def": MOTHS, "lane": 1 }, { "def": GRAVEDIGGER, "lane": 2 }],
                "graveyard": [POINTMASTER],
                "library": LIBRARY,
            },
            "p2": { "hand": [VANILLA], "field": [{ "def": MROW, "lane": 1 }], "graveyard": [SEVEN_SEVEN], "library": LIBRARY },
        }));
        let moths = unit_at(&g, "p1", 1);
        let digger = unit_at(&g, "p1", 2);
        let mrow = unit_at(&g, "p2", 1);
        let p2_hand = g.hand("p2").len();

        // p1's start of turn queues Moths (lane 1), then Gravedigger (lane 2). Moths compels Mrow, which
        // dies to the strike back; its Death takes control of the unit that destroyed it (R361): Moths.
        g.start_turn();
        g.expect_in_zone(&mrow, "graveyard");
        assert_eq!(g.card(&moths).controller, PlayerId::P2);
        assert_eq!(g.card(&digger).controller, PlayerId::P1);

        // Gravedigger is still p1's, and this is p1's turn start: its queued hook fires for p1, and
        // nothing of it reaches the player who took the Moths ahead of it.
        assert!(def_ids(&g.hand("p1")).contains(&POINTMASTER.to_string()));
        assert_eq!(g.hand("p2").len(), p2_hand);
        assert!(def_ids(&g.pile("p2", "graveyard")).contains(&SEVEN_SEVEN.to_string()));
    }
}
