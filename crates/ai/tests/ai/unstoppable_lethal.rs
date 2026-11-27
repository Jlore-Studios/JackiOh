//! R2660000: the ladder bots' concession check (`unstoppable_lethal`, SPEC §9.12): whether a lethal
//! shows on the opponent's board that nothing the bot does this turn stops. Both turns are played
//! through the engine, and every doubt plays on.
//!
//! The bot sits in p1 on its own turn; p2's board is Tempo Timmy 3/3 and Mr. Vanilla 4/4, 7 to the
//! face. The scenarios' libraries are empty unless a test fills the bot's, so p2's next draw is a
//! fatigue point of its own.

use std::cell::Cell;

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

/// A scenario on `seed` with `active` to act, p1 as `p1` and p2 as `p2`.
fn board(seed: &str, active: PlayerId, p1: Value, p2: Value) -> GameState {
    register_cards();
    scenario(json!({ "seed": seed, "active": active, "p1": p1, "p2": p2 }))
        .state()
        .clone()
}

/// p2 with the 3/3 and the 4/4 on its field and whatever `extra` adds.
fn seven(extra: Value) -> Value {
    let mut p2 = json!({ "field": ["core-011", "core-008"] });
    if let (Some(side), Some(extra)) = (p2.as_object_mut(), extra.as_object()) {
        side.extend(extra.clone());
    }
    p2
}

/// A 1/1 that can't switch to Defense Position, so it never walls p2's lethal (a stand-in, #3486).
const SMALL: &str = "stand-in (1) 1/1 Can't Defend";

/// The bot, p1 on its own turn, as `p1`, facing `seven(p2)`.
fn facing(p1: Value, p2: Value) -> GameState {
    board("unstoppable", AI, p1, seven(p2))
}

/// R2660000: 7 to the face of a bot on 6 or 7 with nothing to play, or with a card that changes no
/// board (Core #10 Rapid Replenish, its Combo unmet), kills it whatever it does
#[test]
fn r2660000_a_board_that_kills_the_bot_whatever_it_does_is_unstoppable() {
    for p1 in [
        json!({ "health": 6 }),
        json!({ "health": 7 }),
        json!({ "health": 6, "hand": ["core-010"] }),
    ] {
        let state = facing(p1.clone(), json!({}));
        assert!(unstoppable_lethal(&state, AI, None), "{p1}");
    }
}

/// R2660000: 7 to the face of a bot on 8 leaves it standing
#[test]
fn r2660000_a_board_short_of_lethal_is_not_unstoppable() {
    let state = facing(json!({ "health": 8 }), json!({}));
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000: p2's two burn spells would finish a bot on 8, but a hand is no board
#[test]
fn r2660000_lethal_only_with_the_opponents_hand_is_not_unstoppable() {
    let state = facing(
        json!({ "health": 8 }),
        json!({ "hand": ["core-044", "core-035"] }),
    );
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000, §4.1: a unit played in Defense Position has Taunt, which walls both attackers' way to
/// the face
#[test]
fn r2660000_a_unit_played_into_defense_walls_the_lethal() {
    let state = facing(json!({ "health": 6, "hand": ["core-008"] }), json!({}));
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000: Core #16 Hit Job destroys the 4/4, and 3 to the face leaves a bot on 6 standing
#[test]
fn r2660000_removal_in_hand_stops_the_lethal() {
    let state = facing(json!({ "health": 6, "hand": ["core-016"] }), json!({}));
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000: Core #19 Midrange Menace's Taunt, already on the bot's board, walls the lethal
#[test]
fn r2660000_a_taunt_on_the_bots_board_walls_the_lethal() {
    let state = facing(json!({ "health": 6, "field": ["core-019"] }), json!({}));
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// S+ #100 Plesiosize, a (2) Spell: a Unit in its caster's hand gains +8/+8 and GIGA.
const PLESIOSIZE: &str = "stinkyscapeplus-100";
/// A 1/1 Taunt, which one attacker kills (a stand-in, #3486).
const WALL: &str = "stand-in (1) 1/1 Taunt";

/// R4793000: a bot on 3 with 3 mana holds Plesiosize, a 1/1 Taunt and a 1/1 without it. The Taunt alone
/// dies to one attacker and the other's hit kills; Plesiosize on it first makes it a 9/9 Taunt that
/// takes both attacks and lives, the one escape. The walk tries Plesiosize on the other 1/1 first, and
/// the two used to sign as one position, so it never tried the escape and conceded
#[test]
fn r4793000_plesiosize_on_the_second_hand_unit_is_an_escape_the_walk_tries() {
    let state = facing(
        json!({ "health": 3, "mana": 3, "hand": [PLESIOSIZE, WALL, SMALL] }),
        json!({}),
    );
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000: a bot whose 4/4 swings p2 from 4 to 0 wins before p2's board ever attacks
#[test]
fn r2660000_a_bot_that_wins_this_turn_is_not_doomed() {
    let state = facing(
        json!({ "health": 6, "field": ["core-008"] }),
        json!({ "health": 4 }),
    );
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000: a clock that has run out (the bot's decision deadline) stops the walk before it proves
/// anything, and a walk the clock stopped concedes nothing
#[test]
fn r2660000_a_search_the_clock_stops_proves_nothing() {
    let state = facing(json!({ "health": 6 }), json!({}));
    let stopped = || true;
    assert!(!unstoppable_lethal(&state, AI, Some(&stopped)));
    let running = || false;
    assert!(unstoppable_lethal(&state, AI, Some(&running)));
}

/// R2660000: the check walks every line where a decision's search walks a beam, so its budget is
/// twice a decision's: a doomed board whose walk spends more than `AI_BUDGET.nodes` is still proven
#[test]
fn r2660000_a_doomed_board_wider_than_one_decisions_budget_is_still_proven() {
    let state = board(
        "unstoppable",
        AI,
        json!({ "health": 6, "field": [SMALL, SMALL, SMALL, SMALL] }),
        json!({ "field": ["stand-in (1) 7/30"] }),
    );
    let polls = Cell::new(0_usize);
    let count = || {
        polls.set(polls.get() + 1);
        false
    };
    assert!(unstoppable_lethal(&state, AI, Some(&count)));
    assert!(polls.get() > AI_BUDGET.nodes, "{} nodes", polls.get());
}

/// R2660000: on p2's turn the bot has no turn to play, whatever p2's board would deal
#[test]
fn r2660000_off_the_bots_own_turn_nothing_is_unstoppable() {
    let state = board("unstoppable", HUMAN, json!({ "health": 6 }), seven(json!({})));
    assert!(!unstoppable_lethal(&state, AI, None));
}

/// R2660000, R185: states that differ only in p2's hand, its face-down trap and the seed redact alike
/// and give one answer, the one p2's board alone gives: two Traps set face-down against the bot
/// change nothing, neither stopping its answer nor making the lethal
#[test]
fn r2660000_the_opponents_hidden_cards_change_nothing() {
    for health in [6, 8] {
        let a = board(
            "unstoppable-a",
            AI,
            json!({ "health": health }),
            seven(json!({
                "hand": ["core-044", "core-035"],
                "backrow": [{ "def": "core-041", "faceUp": false }],
            })),
        );
        let b = board(
            "unstoppable-b",
            AI,
            json!({ "health": health }),
            seven(json!({
                "hand": ["core-016", "core-002"],
                "backrow": [{ "def": "core-060", "faceUp": false }],
            })),
        );
        assert_ne!(hash_state(&a), hash_state(&b), "health {health}");
        assert_eq!(
            hash_state(&redact(&a, AI)),
            hash_state(&redact(&b, AI)),
            "health {health}"
        );
        let doomed = unstoppable_lethal(&a, AI, None);
        assert_eq!(unstoppable_lethal(&b, AI, None), doomed, "health {health}");
        assert_eq!(doomed, health == 6, "health {health}");
    }
}

/// R2660000: Core #23 Reoccurring Dream rolls the game's random numbers for the Rapid Replenish
/// beside it, and the walk sees only one outcome of a roll: a line that rolls proves nothing, so the
/// bot holding it plays on, though no outcome of this one saves it. With nothing in hand to roll for,
/// the Dream rolls nothing, and the same bot is doomed.
#[test]
fn r2660000_a_line_that_rolls_proves_nothing() {
    let rolls = facing(
        json!({ "health": 6, "hand": ["core-023", "core-010"] }),
        json!({}),
    );
    assert!(!unstoppable_lethal(&rolls, AI, None));
    let alone = facing(json!({ "health": 6, "hand": ["core-023"] }), json!({}));
    assert!(unstoppable_lethal(&alone, AI, None));
}

/// R2660000: Core #5 Stockpile draws 2 from a library whose order the bot cannot see: whether the
/// draw finds the Hit Job that would save it is that order, so a line that draws proves nothing. It
/// proves nothing either from a library with nothing in it to save the bot, since the walk guesses
/// the order and cannot rule a card in or out by it.
#[test]
fn r2660000_a_line_that_draws_proves_nothing() {
    for library in [
        json!(["core-010", "core-016", "core-010", "core-010"]),
        json!(["core-010", "core-010", "core-010", "core-010"]),
    ] {
        let state = facing(
            json!({ "health": 4, "hand": ["core-005"], "library": library }),
            json!({}),
        );
        assert!(!unstoppable_lethal(&state, AI, None), "{library}");
    }
    // PREMISE: without the Stockpile, the same bot is doomed.
    let bare = facing(
        json!({ "health": 4, "library": ["core-010", "core-010"] }),
        json!({}),
    );
    assert!(unstoppable_lethal(&bare, AI, None));
}

/// R2660000: the opponent's lethal rolls too: Stinkyscape #53 Flak v3's Activate deals each enemy a
/// random 1 to 3, so with it p2's 7 kills a bot on 9 only by the roll, which a lethal that counts
/// cannot leave to chance. The solver's line is played again and rolls, so it proves nothing, on 8
/// as well, where every outcome would kill: the walk sees one outcome and cannot rule the others in.
/// Without the Activate's 1, p2's 7 does not reach 8.
#[test]
fn r2660000_an_opponents_lethal_that_rolls_proves_nothing() {
    for health in [8, 9] {
        let state = facing(
            json!({ "health": health }),
            json!({ "field": ["core-011", "core-008", "stinkyscape-053"] }),
        );
        assert!(!unstoppable_lethal(&state, AI, None), "health {health}");
    }
}

/// `facing(p1, p2)` with the Marketplace open (R1853000), offering the bot `offers` (its owner's) at
/// 40 Credits each, of which it holds 100.
fn with_market(p1: Value, p2: Value, offers: &[&str]) -> GameState {
    let mut state = facing(p1, p2);
    state.players[AI].market_credits = Some(100);
    let offer = |def_id: &&str| MarketOffer {
        def_id: def_id.to_string(),
        radiant: None,
        base: 40,
        price: 40,
        stock: 1,
        bought: PerPlayer::new(0, 0),
        owner: Some(AI),
        fixed: None,
        buffs: None,
        nerfs: None,
    };
    state.marketplace = Some(Marketplace {
        offers: offers.iter().map(offer).collect(),
        quotes: Vec::new(),
    });
    state
}

/// R2660000, R1853000: the opponent's turn start rolls it new Marketplace offers on the game's
/// random numbers, which play no part in its board: the bot doomed without a Marketplace is doomed
/// with one, and a Mr. Vanilla it can buy and play in Defense Position this turn is still its way out
#[test]
fn r2660000_the_marketplace_neither_hides_nor_makes_the_lethal() {
    let open = with_market(json!({ "health": 6 }), json!({}), &[]);
    assert!(unstoppable_lethal(&open, AI, None));
    let buys_a_wall = with_market(json!({ "health": 6 }), json!({}), &["core-008"]);
    assert!(
        legal_actions(&buys_a_wall, AI).contains(&ActionBody::MarketBuy { offer: 0 }),
        "PREMISE: the bot can buy the offer"
    );
    assert!(!unstoppable_lethal(&buys_a_wall, AI, None));
}

/// R3047001: a card bought at the Marketplace costs (0), so a bot with no mana left still buys the
/// offered Mr. Vanilla (a (1) card) and plays it in Defense Position this turn: the wall it holds is
/// real, and the lethal is not unstoppable. The same bot with nothing to buy is doomed.
#[test]
fn r3047001_a_card_bought_costs_nothing_so_a_bot_without_mana_still_walls_the_lethal() {
    let broke = |offers: &[&str]| with_market(json!({ "health": 6, "mana": 0 }), json!({}), offers);
    let walled = broke(&["core-008"]);
    assert_eq!(walled.players[AI].mana.current, 0, "PREMISE: no mana");
    assert!(
        legal_actions(&walled, AI).contains(&ActionBody::MarketBuy { offer: 0 }),
        "PREMISE: the bot can buy the offer with Credits"
    );
    assert!(!unstoppable_lethal(&walled, AI, None));
    assert!(unstoppable_lethal(&broke(&[]), AI, None));
}

/// Classic+ #35 Rollback, a (4) Spell: "Choose 1, 2 or 3. Return the board to how it was that many
/// turns ago."
const ROLLBACK: &str = "classicplus-035";

/// `facing(p1, {})` with a board history (R419) whose one snapshot, last turn's, holds neither side's
/// field: p2's 3/3 and 4/4 came after it.
fn with_history_before_the_lethal(p1: Value) -> GameState {
    let mut state = facing(p1, json!({}));
    let mut before = board("unstoppable", AI, json!({}), json!({}));
    before.turn = state.turn - 1;
    subsystems::board_history::record_board_snapshot(&mut before);
    state.board_history = before.board_history;
    state
}

/// R4796000: a bot holding Classic+ #35 Rollback, with the mana for it, can return the board to last
/// turn's, before p2's lethal stood there. The history a Rollback reads is no seat's to see in full
/// (`redact`), so a step that returns the board to a snapshot proves nothing, and the bot plays on. The
/// same bot a crystal short of it is doomed.
#[test]
fn r4796000_a_bot_that_can_roll_the_board_back_plays_on() {
    let state = with_history_before_the_lethal(json!({ "health": 6, "hand": [ROLLBACK], "mana": 4 }));
    let rollback = state.players[AI].hand[0].id.clone();
    let takes_the_lethal_off = legal_actions(&state, AI).into_iter().any(|action| {
        let ours = matches!(&action, ActionBody::Play { instance_id, .. } if *instance_id == rollback);
        let played = reduce_unpiloted(&state, &Action::new(action, AI, "premise"));
        ours && played.error.is_none() && active_units_of(&played.state, HUMAN).is_empty()
    });
    assert!(
        takes_the_lethal_off,
        "PREMISE: in the game, the Rollback takes p2's units off the field"
    );
    assert!(!unstoppable_lethal(&state, AI, None));
    let short = with_history_before_the_lethal(json!({ "health": 6, "hand": [ROLLBACK], "mana": 3 }));
    assert!(unstoppable_lethal(&short, AI, None));
}

/// Meditative #22 Mind Games, a (2) Spell: Greed, Attack or Defend, chosen in secret, its reward at its
/// caster's next turn start; Attack deals 8 damage to each enemy.
const MIND_GAMES: &str = "meditative-022";
/// The Fortify Mind (M #22.1) Mind Games adds to the other player's hand, which guesses the choice.
const FORTIFY_MIND: &str = "meditative-022-1";
/// The bot's library in `after_mind_games`: Rapid Replenishes, as many as put each of the check's
/// worlds' guesses of the secret on Attack.
const MIND_GAMES_LIBRARY: usize = 29;

/// The bot, p1 on `health`, on its own turn after p2 (its 3/3 and 4/4 on the field) cast Mind Games
/// choosing `choice`, so the reward lands at p2's next turn start, with `library` Rapid Replenishes in
/// its library. The bot holds no Fortify Mind: its hand was full, and the token burned.
fn after_mind_games(choice: &str, health: i32, library: usize) -> GameState {
    register_cards();
    let mut s = scenario(json!({
        "seed": "unstoppable",
        "active": HUMAN,
        "p1": { "library": vec!["core-010"; library] },
        "p2": { "field": ["core-011", "core-008"], "hand": [MIND_GAMES], "mana": 2 },
    }));
    for player in [AI, HUMAN] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s.play(MIND_GAMES, json!({ "modes": [choice] }));
    s.end_turn();
    let mut state = s.state().clone();
    state.players[AI].hand.retain(|card| card.def_id != FORTIFY_MIND);
    state.players[AI].hero.health = health;
    state
}

/// What each of the check's worlds would guess p2's secret to be, were the secret kept in them: the
/// determinizations `unstoppable_lethal` draws, on its own stream, of the state `redact` gives the bot.
fn guessed_secrets(state: &GameState) -> Vec<Option<SecretChoice>> {
    let public = redact(state, AI);
    let ground = det_ground(&public, AI);
    let mut rng = Rng::new(UNSTOPPABLE_LETHAL_SEED, 0);
    (0..UNSTOPPABLE_LETHAL_DETERMINIZATIONS)
        .map(|_| {
            let world = determinize_on(&public, AI, &mut rng, DeterminizeOptions::default(), &ground);
            world.secrets.iter().flatten().find_map(|secret| secret.choice)
        })
        .collect()
}

/// R4796000: p2's Mind Games secret is Defend, so in the game its reward heals p2 and the bot on 12
/// lives through the 7. A world that guessed Attack, 8 to each enemy before p2 attacks, would make the
/// lethal by the guess alone, and on this stream all of them guess Attack. An unrevealed secret is
/// taken out with the opponent's hand, its reward doing nothing, so the bot plays on; the bot on 6,
/// whom p2's board kills whatever the secret, is still doomed.
#[test]
fn r4796000_an_unrevealed_secret_neither_makes_nor_hides_the_lethal() {
    let defended = after_mind_games("defend", 12, MIND_GAMES_LIBRARY);
    assert_eq!(defended.active, AI, "PREMISE: the bot's turn");
    assert_eq!(
        guessed_secrets(&defended),
        vec![Some(SecretChoice::Attack); UNSTOPPABLE_LETHAL_DETERMINIZATIONS],
        "PREMISE: every world guesses Attack"
    );
    assert!(!unstoppable_lethal(&defended, AI, None));
    assert!(unstoppable_lethal(
        &after_mind_games("defend", 6, MIND_GAMES_LIBRARY),
        AI,
        None
    ));
}
