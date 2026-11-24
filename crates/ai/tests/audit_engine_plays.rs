//! Audit of how a card is played (`play_choices.rs`, `play_steps.rs` and their neighbours): the tests
//! that prove the audit's findings. Each asserts the ruled behaviour and fails on the engine as it is.

use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

fn plays_of(state: &GameState, id: &str) -> Vec<PlayAction> {
    legal_actions(state, P1)
        .iter()
        .filter_map(PlayAction::from_body)
        .filter(|play| play.instance_id == id)
        .collect()
}

/// R4665000: Ticketed's play "lands on top of that permanent's pile, in its zone"; its graveyard is
/// only for "a permanent that has left the field or been covered by the time Ticketed is placed" or
/// "no permanent on the field to name". R3546000: "Nothing stacks, equips, carries onto or captures
/// with a GIGA, covered zones included". So a GIGA is no pick Ticketed can stack on, as a Stack play's
/// zones (`legal_zones_for`) and an Equip play's hosts (`equip_host_at`) leave it out through
/// `zones::accepts_stack_card`: every play of Ticketed `legal_actions` lists must put it on the field.
#[test]
fn finding_ticketed_is_offered_permanents_it_cannot_stack_on_and_is_lost_with_its_cost() {
    jackioh_cards::register_all();
    drop(std::panic::take_hook());
    let s = scenario(json!({
        "p1": {
            "hand": ["stinkyscape-009"],
            "field": [{ "def": "core-008", "lane": 5 }],
            "backrow": [{ "def": "stinkyscapeplus-053", "lane": 1, "faceUp": true }],
            "mana": 10,
        },
        "p2": {
            "hand": ["core-008"],
            "field": [{ "def": "stinkyscapeplus-t-ai-04", "lane": 1 }, { "def": "core-019", "lane": 5 }],
        },
    }));
    let state = s.state().clone();
    let ticketed = state.players[P1].hand[0].id.clone();
    let mut lost: Vec<String> = Vec::new();
    for play in nonempty(plays_of(&state, &ticketed), "plays of Ticketed") {
        let Some([Selection::Instance { instance_id: named }]) = play.targets.as_deref() else {
            panic!("Ticketed's play names one permanent: {play:?}");
        };
        let result = reduce(&state, &Action::new(play.clone().into_body(), P1, "ticketed"));
        assert_eq!(result.error, None, "a listed play is accepted");
        let card = must(find_instance(&result.state, &ticketed), "Ticketed after its play");
        if !matches!(card.zone, Zone::Field { .. }) {
            let target = must(find_instance(&state, named), "the named permanent");
            lost.push(format!(
                "onto {} ({}): Ticketed is in {:?}, mana {} -> {}",
                target.def_id,
                named,
                card.zone,
                state.players[P1].mana.current,
                result.state.players[P1].mana.current
            ));
        }
    }
    assert!(lost.is_empty(), "legal_actions offered Ticketed targets it cannot stack on:\n{}", lost.join("\n"));
}

/// R360: a play of #55 Lava Golem's base face "whose Tribute takes at least one unit the opponent
/// controls ... puts the Golem, at step 4, into the opponent's zone ... under the opponent's control".
/// R1086, R1869000: an Equip play fuses "the card on top" into its host, the Unit "standing on Rod";
/// "If either card left first, nothing is fused". Played as an Equip play onto Stinkyscape #16 Rod of
/// Ages with enemy Tributes, the Golem is summoned for the opponent and never stands on Rod, so it
/// must still be the opponent's once the play has resolved.
#[test]
fn finding_r360_a_lava_golem_handed_to_the_opponent_is_not_fused_back_into_its_players_rod_of_ages() {
    jackioh_cards::register_all();
    drop(std::panic::take_hook());
    let s = scenario(json!({
        "p1": {
            "hand": ["core-055"],
            "field": [{ "def": "core-008", "lane": 5 }],
            "backrow": [{ "def": "stinkyscape-016", "lane": 1, "faceUp": true }],
            "mana": 10,
        },
        "p2": { "hand": ["core-008"], "field": ["core-008", "core-008", "core-008"] },
    }));
    let state = s.state().clone();
    let golem = state.players[P1].hand[0].id.clone();
    let rod = must(state.players[P1].backrow[0].clone(), "Rod of Ages").id;
    let enemy: Vec<String> = active_units_of(&state, P2).into_iter().map(|unit| unit.id.clone()).collect();
    let play = must(
        plays_of(&state, &golem).into_iter().find(|play| {
            play.equip == Some(true) && play.tributes.as_ref().is_some_and(|paid| paid.iter().all(|id| enemy.contains(id)))
        }),
        "a listed Equip play of Lava Golem onto Rod of Ages that tributes three enemy Units",
    );
    let result = reduce(&state, &Action::new(play.into_body(), P1, "golem"));
    assert_eq!(result.error, None);
    assert!(result.state.pending.is_none());
    // R360's hand-over happened: step 4 summoned it for the opponent.
    assert!(result.events.iter().any(|event| matches!(
        event,
        GameEvent::Summoned { player: P2, instance_id, .. } if *instance_id == golem
    )));
    let after = find_instance(&result.state, &golem);
    let rod_after = must(find_instance(&result.state, &rod), "Rod of Ages after the play");
    assert!(
        after.is_some_and(|card| matches!(card.zone, Zone::Field { player: P2, .. }) && card.controller == P2),
        "R360: the Golem is the opponent's; it is {:?}, and Rod of Ages is now {} under {}",
        after.map(|card| card.zone.clone()),
        rod_after.def_id,
        rod_after.controller
    );
}

/// §9.3, §10.2: "A refused action comes back with the input state, no events and its message".
/// §2.3, R348: an X above the mana the player can spend is refused ("X is above your current mana").
/// While a Mayor acts (R1200) `validate_play` draws the targets before it checks X, so Stinkyscape #99
/// No-No Words' target check multiplies the unchecked X (`per * x`, c099_no_no_words.rs:29).
#[test]
fn finding_a_play_naming_a_huge_x_is_refused_not_a_panic_while_a_mayor_acts() {
    jackioh_cards::register_all();
    drop(std::panic::take_hook());
    let s = scenario(json!({
        "p1": { "hand": ["stinkyscape-099"], "mana": 4 },
        "p2": { "hand": ["core-008"], "field": ["meditative-086", "core-008"] },
    }));
    let state = s.state().clone();
    let play = PlayAction {
        instance_id: state.players[P1].hand[0].id.clone(),
        x: Some(i32::MAX),
        ..Default::default()
    };
    let action = Action::new(play.into_body(), P1, "huge-x");
    let outcome = catch_unwind(AssertUnwindSafe(|| reduce(&state, &action)));
    assert!(
        outcome.as_ref().is_ok_and(|result| result.error.is_some()),
        "reduce must refuse X = i32::MAX, not panic: {:?}",
        outcome.map(|result| result.error).map_err(|payload| payload.downcast_ref::<String>().cloned())
    );
}

/// R3546000: a GIGA "stands in its anchor ... and covers the other row's lane L and both rows' lane
/// L+1: four zones nothing else enters. Only fitting anchors are offered, zones its own Tribute
/// empties counting (R391)". R391: "a Tribute card may still be played, into a zone its own Tribute
/// empties ... a play reads its own side once the Tribute is paid". Tributing a GIGA empties all four
/// of its zones, so S+ #52 Airblast Drill ("Tribute 1, GIGA") fits where the GIGA it tributes stood.
#[test]
fn finding_r391_r3546000_a_tributed_giga_empties_the_zones_it_covers_for_the_play() {
    jackioh_cards::register_all();
    drop(std::panic::take_hook());
    let s = scenario(json!({
        "p1": {
            "hand": ["stinkyscapeplus-052"],
            // Spectre (GIGA) fills lanes 1 and 2 of both rows; the Unit in lane 4 leaves no other fit.
            "field": [{ "def": "stinkyscapeplus-106", "lane": 1 }, { "def": "core-008", "lane": 4 }],
            "mana": 4,
        },
        "p2": { "hand": ["core-008"], "field": [{ "def": "core-008", "lane": 5 }] },
    }));
    let state = s.state().clone();
    let drill = state.players[P1].hand[0].id.clone();
    let spectre = must(state.players[P1].units[0].as_ref().and_then(|pile| pile.first()), "Spectre").id.clone();
    let wanted = PlayAction {
        instance_id: drill.clone(),
        zone: Some(ZoneChoice { row: Row::Backrow, lane: 1 }),
        tributes: Some(vec![spectre.clone()]),
        ..Default::default()
    };
    let plays = plays_of(&state, &drill);
    let refusal = reduce(&state, &Action::new(wanted.clone().into_body(), P1, "drill")).error;
    let reason = match &view_for(&state, P1).you.hand {
        HandView::Cards(cards) => cards[0].play_reason.clone(),
        HandView::Count { .. } => None,
    };
    assert!(
        plays.contains(&wanted),
        "Airblast Drill tributing Spectre fits in lanes 1 and 2, yet legal_actions lists {} plays of it \
         ({:?}), the view says {reason:?}, and reduce answers {refusal:?}",
        plays.len(),
        plays.iter().map(|play| (play.zone, play.tributes.clone())).collect::<Vec<_>>()
    );
}
