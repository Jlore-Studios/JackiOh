//! Audit of the engine's work queue, prompts, traps and counters (`crates/engine/src/prompts.rs`,
//! `cry_trigger.rs`). Each test asserts the ruled or printed behaviour and fails on the code as it stands.

use jackioh_engine::testkit::*;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// Finding 1. C #9 Income Tax: "Reveals when the cards your opponent has drawn in a turn reach {draws}:
/// They keep one card of their choice and give you the rest of their hand." Its keep prompt is held by
/// the drawer and owned by the trap's controller (`OpenPromptArgs.owner`, B5 E18: "The answered step runs
/// as this player, the card's controller"). R452 answers a prompt opened for a random cast's caster at
/// once and at random; `prompts::answer_at_random` then runs the step as the player asked
/// (`controller: Some(args.player)`), not as the owner, so "give_from_hand { from: enemy, unchosen }"
/// reads the wrong side: the trap's controller hands their whole hand to the drawer.
///
/// P1 plays C+ #47 Jogg's Box (8 random Spells) on their own turn into P2's set Income Tax. Whenever
/// the trap fires inside the play, every card it moves must go from the drawer (P1) to its controller (P2).
#[test]
fn r452_income_tax_fired_inside_the_drawers_random_cast_takes_the_drawers_hand_not_its_own_controllers() {
    jackioh_cards::register_all();
    const TAX: &str = "classic-009";
    const JOGG: &str = "classicplus-047";
    let filler: Vec<&str> = vec!["core-008"; 30];
    let mut fired = 0;
    let mut wrong: Vec<String> = Vec::new();
    for n in 0..60 {
        let s = scenario(json!({
            "seed": format!("audit-tax:{n}"), "active": "p1", "turn": 9,
            "p1": { "hand": [JOGG, "core-008", "core-008", "core-008"], "mana": 10, "library": filler },
            "p2": { "hand": ["core-019", "core-012", "core-025"],
                    "backrow": [{ "def": TAX, "faceUp": false }], "library": filler },
        }));
        let play = must(s.plays_of(JOGG).into_iter().next(), "a play of Jogg's Box");
        let result = reduce(s.state(), &Action::new(play, P1, "jogg"));
        assert_eq!(result.error, None);
        let Some(from) = result
            .events
            .iter()
            .position(|event| matches!(event, GameEvent::TrapFired { def_id, .. } if def_id == TAX))
        else {
            continue;
        };
        fired += 1;
        // The firing's own events: from `trapFired` to the trap's consumption (§3.2), or to the end.
        let firing: Vec<&GameEvent> = result.events[from..]
            .iter()
            .take_while(
                |event| !matches!(event, GameEvent::EnteredGraveyard { def_id, .. } if def_id == TAX),
            )
            .collect();
        for event in firing {
            if let GameEvent::Stolen { from, to, def_id, .. } = event
                && (*from, *to) != (P1, P2)
            {
                wrong.push(format!("seed {n}: Income Tax moved {def_id} from {from} to {to}"));
            }
        }
    }
    assert!(fired > 0, "no seed fired Income Tax inside the play: widen the seed range");
    assert!(
        wrong.is_empty(),
        "Income Tax (P2's) took its own controller's hand for the drawer in {} of its moves:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// Finding 2. R450: "Targeting means a declared `target` pick or a `target` prompt answer. A targeting
/// cost (Classic #89 Paul Allen's Ghost) is paid in random discards at pay time (R682) ... and a prompt
/// answer naming such a card pays before it goes on." R467: a triggered Cry's "declared targets and modes
/// are asked as prompts". C #54 Rewind triggers Twisted Sorcerer's Cry ("deal 4 damage to a target"); its
/// `target` prompt offers the Ghost (the prompt's options already hold the chooser to the cost: see the
/// second half), yet `cry_trigger::answer_cry_prompt` files the pick without the targeting point, so
/// the Ghost is targeted for free.
#[test]
fn r450_a_triggered_crys_target_prompt_answer_naming_paul_allens_ghost_pays_its_discards() {
    jackioh_cards::register_all();
    const REWIND: &str = "classic-054";
    const SORCERER: &str = "core-068";
    const GHOST: &str = "classic-089";
    let board = |spares: usize| {
        let mut hand = vec![REWIND];
        hand.extend(vec!["core-008"; spares]);
        let mut s = scenario(json!({
            "seed": "audit-rewind", "active": "p1", "turn": 9,
            "p1": { "hand": hand, "mana": 10, "field": [SORCERER],
                    "library": ["core-008", "core-008", "core-008"] },
            "p2": { "field": [GHOST], "library": ["core-008", "core-008", "core-008"] },
        }));
        let sorcerer = s.card(SORCERER).id.clone();
        s.play(REWIND, json!({ "targets": [{ "pick": "instance", "instanceId": sorcerer }] }));
        s
    };

    // With one other card in hand the prompt does not offer the Ghost: it is a targeting prompt (R450).
    let poor = board(1);
    let ghost = poor.card(GHOST).id.clone();
    let pending = must(poor.state().pending.clone(), "the triggered Cry's target prompt");
    assert_eq!((pending.kind, pending.resume.hook.as_str()), (PromptKind::Target, "@triggerCry"));
    assert!(!pending.options.iter().any(|option| option.key == format!("instance:{ghost}")));

    // With three it does, and the answer naming it must pay 2 random discards before the Cry goes on.
    let mut s = board(3);
    let ghost = s.card(GHOST).id.clone();
    assert_eq!(s.hand(P1).len(), 3);
    s.answer(json!([{ "pick": "instance", "instanceId": ghost }]));
    assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::DivineShieldLost { .. })));
    let discards = s
        .last_events()
        .iter()
        .filter(|event| matches!(event, GameEvent::Discarded { .. }))
        .count();
    assert_eq!(
        (discards, s.hand(P1).len()),
        (2, 1),
        "R450: a prompt answer naming Paul Allen's Ghost pays its 2 discards (discards, cards left in hand)"
    );
}

/// Finding 2, second instance. A "Place N Plague Counters" placement is one `target` prompt (R689: "all
/// on the one permanent a single prompt names"; its kind is `target`), and R450 reads a `target` prompt
/// answer as targeting. C #70 Book of Plague's prompt holds the chooser to the Ghost's cost (not offered
/// with one other card), yet `effects::plague::answer_placement` places on it for free.
#[test]
fn r450_a_plague_placement_prompt_answer_naming_paul_allens_ghost_pays_its_discards() {
    jackioh_cards::register_all();
    const BOOK: &str = "classic-070";
    const GHOST: &str = "classic-089";
    let board = |spares: usize| {
        let mut hand = vec![BOOK];
        hand.extend(vec!["core-008"; spares]);
        let mut s = scenario(json!({
            "seed": "audit-plague", "active": "p1", "turn": 9,
            "p1": { "hand": hand, "mana": 10, "library": ["core-008", "core-008", "core-008"] },
            "p2": { "field": [GHOST, "core-019"], "library": ["core-008", "core-008", "core-008"] },
        }));
        s.play(BOOK, json!({ "modes": ["counters"] }));
        s
    };

    let poor = board(1);
    let ghost = poor.card(GHOST).id.clone();
    let pending = must(poor.state().pending.clone(), "the placement prompt");
    assert_eq!((pending.kind, pending.resume.hook.as_str()), (PromptKind::Target, "plague:placement"));
    assert!(!pending.options.iter().any(|option| option.key == format!("instance:{ghost}")));

    let mut s = board(3);
    let ghost = s.card(GHOST).id.clone();
    s.answer(json!([{ "pick": "instance", "instanceId": ghost }]));
    assert_eq!(s.card(GHOST).counters.plague, Some(5));
    let discards = s
        .last_events()
        .iter()
        .filter(|event| matches!(event, GameEvent::Discarded { .. }))
        .count();
    assert_eq!(
        (discards, s.hand(P1).len()),
        (2, 1),
        "R450: a prompt answer naming Paul Allen's Ghost pays its 2 discards (discards, cards left in hand)"
    );
}
