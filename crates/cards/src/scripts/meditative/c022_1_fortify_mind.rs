//! Meditative #22.1 Fortify Mind (SPEC §8.8 row 22.1, BUILD M10 row M 22.1): (0) Spell, Token
//! (printed Epic).
//!   Base:    "Temporary. Choose one: Greed, Attack or Defend, to guess your opponent's Mind Games.
//!            If yours beats theirs (Attack beats Greed, Greed beats Defend, Defend beats Attack),
//!            cancel their reward. If you chose the same, nothing happens. If theirs beats yours,
//!            take your choice's penalty. Greed: You have {mana} less mana next turn. Discard
//!            {discards} random cards. Attack: Deal {damage} damage to your hero and each of your
//!            Units. Defend: Heal your opponent's hero and each of their Units {heal}. They gain
//!            +{armor} Armor. When you discard this, take all three penalties."
//!   Radiant: the same without the last line.
//! The guess is a declared mode (R81), judged at resolution against the secret the Mind Games that
//! made this card linked in its memory (R862): only that Mind Games writes the link, so a copy, a
//! Fortify Mind another card makes, or one played after the secret resolved guesses nothing. A win
//! removes the reward's delayed effect (R864); a tie or a loss leaves it, both players now reading
//! the choice. The base face's discard clause answers its own `discarded` event — Temporary's
//! included — whatever the secret's state; being burned is not a discard. The penalties fall on
//! whoever plays or discards the card, even the secret's owner (R863).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-022-1";

/// `forEachCard`'s `cards`, typed (TS `(ctx) => readonly (CardInstance | string)[]`, ids here).
fn cards_of(f: impl Fn(&mut EffectContext<'_>) -> Vec<String> + Send + Sync + 'static) -> ForEachCardCards {
    Arc::new(f)
}

/// `forEachCard`'s `each`, typed (TS `(instanceId) => Effect`).
fn each_of(f: impl Fn(&str) -> Effect + Send + Sync + 'static) -> ForEachCardEach {
    Arc::new(f)
}

fn guess_modes() -> Vec<ModeDecl> {
    vec![ModeDecl {
        kind: PromptKind::Mode,
        options: SecretChoice::ALL.iter().map(|choice| choice.as_str().to_string()).collect(),
    }]
}

fn guess_of(mode: Option<String>) -> Option<SecretChoice> {
    mode?.parse::<SecretChoice>().ok()
}

/// One choice's penalty, on the player who plays or discards this card: Greed taxes their own next
/// refresh and discards at random; Attack hits their own hero and each of their Units; Defend heals
/// and armors the opponent's side instead (R863).
fn penalty(ctx: &EffectContext<'_>, guess: SecretChoice) -> Vec<Effect> {
    match guess {
        SecretChoice::Greed => vec![
            next_turn_mana(json_as(json!({ "amount": -param(ctx, "mana") }))),
            discard_random(json_as(json!({ "count": param(ctx, "discards") }))),
        ],
        SecretChoice::Attack => vec![damage_all(json_as(json!({
            "amount": param(ctx, "damage"),
            "side": "self",
            "heroes": true,
        })))],
        SecretChoice::Defend => {
            let heal_n = param(ctx, "heal");
            let armor_n = param(ctx, "armor");
            vec![
                heal(json_as(json!({ "target": { "of": "enemyHero" }, "amount": heal_n }))),
                for_each_card(ForEachCardArgs {
                    cards: cards_of(|ctx| {
                        active_units_of(ctx.state, ctx.controller.opponent())
                            .iter()
                            .map(|unit| unit.id.clone())
                            .collect()
                    }),
                    each: each_of(move |instance_id| {
                        heal(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "amount": heal_n,
                        })))
                    }),
                }),
                gain_hero_armor(json_as(json!({ "amount": armor_n, "player": "enemy" }))),
                for_each_card(ForEachCardArgs {
                    cards: cards_of(|ctx| {
                        active_units_of(ctx.state, ctx.controller.opponent())
                            .iter()
                            .map(|unit| unit.id.clone())
                            .collect()
                    }),
                    each: each_of(move |instance_id| {
                        grant_keyword(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "keyword": { "kind": "Armor", "n": armor_n },
                        })))
                    }),
                }),
            ]
        }
    }
}

/// Every penalty at once: the base face's discard clause (R862).
fn all_penalties(ctx: &EffectContext<'_>) -> Vec<Effect> {
    let mut out = penalty(ctx, SecretChoice::Greed);
    out.extend(penalty(ctx, SecretChoice::Attack));
    out.extend(penalty(ctx, SecretChoice::Defend));
    out
}

fn fortify_mind(discard_clause: bool) -> Script {
    Script {
        modes: guess_modes(),
        cry: Some(hook(|ctx| {
            let guess = guess_of(chosen_options(ctx).into_iter().next());
            // R862: the guess is judged against the secret this card's own Mind Games linked in its
            // memory. No guess, no link (a copy, or one another card made) or no live secret — a
            // resolved one included — and the card does nothing.
            let link = recalled(ctx, SECRET_KEY)
                .and_then(|value| value.as_str().map(str::to_string));
            let (Some(guess), Some(secret_id)) = (guess, link) else {
                return vec![];
            };
            let Some(choice) = secret_of(ctx.state, &secret_id).and_then(|secret| secret.choice)
            else {
                return vec![];
            };
            let mut out =
                vec![guess_secret(json_as(json!({ "secretId": secret_id, "guess": guess.as_str() })))];
            if judge(guess, choice) == PredictOutcome::Lost {
                out.extend(penalty(ctx, guess));
            }
            out
        })),
        graveyard_triggers: if discard_clause {
            vec![TriggerDef::new(
                "discardPenalty",
                &[GameEventType::Discarded],
                |ctx, event| {
                    // R862: the clause answers its own discard — Temporary's included — whatever the
                    // secret's state, even after it resolved or when none was ever kept.
                    let GameEvent::Discarded { instance_id, .. } = event else {
                        return vec![];
                    };
                    if !ctx.self_.as_ref().is_some_and(|card| &card.id == instance_id) {
                        return vec![];
                    }
                    all_penalties(ctx)
                },
            )]
        } else {
            Vec::new()
        },
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fortify_mind(true),
        radiant: fortify_mind(false),
    }
}

// Meditative #22.1 Fortify Mind — SPEC §8.8 row 22.1, BUILD M10 row M 22.1: "(0) Token Spell
// (printed Epic): Temporary; played, its declared guess is judged against its linked secret
// (Attack beats Greed, Greed beats Defend, Defend beats Attack remove the reward's delayed effect;
// the same does nothing; a losing guess takes that choice's penalty); `predicted` and
// `secretRevealed` are public; after the secret has resolved it does nothing; discarded by
// Temporary or by any effect, all three penalties land; stolen back, the penalties are the owner's;
// radiant no discard clause".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const MIND: &str = "meditative-022";
    const FORTIFY: &str = "meditative-022-1";
    const FILLER: &str = "core-005"; // (1) Spell.
    const MENACE: &str = "core-019"; // (3) 9/9.
    const WIND: &str = "classic-028"; // Second Wind: exile your deck, discard your hand.

    /// p1 keeps `secret`; p2 holds the linked Fortify Mind and it is p2's turn.
    fn table(secret: &str, p1field: Value, p2hand: Value, p2field: Value) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": MIND }, FILLER],
                "field": p1field,
                "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": {
                "hand": p2hand,
                "field": p2field,
                "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                "mana": 10,
            },
        }));
        s.play(MIND, json!({ "modes": [secret] }));
        s.end_turn();
        s
    }

    mod base {
        use super::*;

        #[test]
        fn is_a_0_cost_temporary_token_spell() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(0));
            assert_eq!(def.type_, CardType::Spell);
            assert_eq!(def.rarity, Rarity::Token);
            assert_eq!(def.printed_rarity, Some(PrintedRarity::Epic));
            assert!(def.token);
            assert_eq!(def.tags, vec![Tag::Token]);
            assert_eq!(def.refs, Some(vec![MIND.to_string()]));
            for face in [&def.base, &def.radiant] {
                assert_eq!(face.keywords, vec![Keyword::Temporary]);
            }
            let params = def.params.clone().unwrap_or_default();
            let pair = |key: &str| {
                params.iter().find(|param| param.key == key).map(|param| (param.base, param.radiant))
            };
            assert_eq!(pair("mana"), Some((2, 2)));
            assert_eq!(pair("discards"), Some((2, 2)));
            assert_eq!(pair("damage"), Some((8, 8)));
            assert_eq!(pair("heal"), Some((8, 8)));
            assert_eq!(pair("armor"), Some((2, 2)));
        }

        #[test]
        fn r864_attack_beats_greed_and_cancels_the_reward() {
            let mut s = table("greed", json!([]), json!([FILLER]), json!([]));
            s.play(FORTIFY, json!({ "modes": ["attack"] }));
            let events = s.last_events();
            assert!(events.iter().any(|event| matches!(
                event,
                GameEvent::SecretRevealed { choice: SecretChoice::Greed, .. }
            )));
            assert!(events.iter().any(|event| matches!(
                event,
                GameEvent::Predicted { guess: SecretChoice::Attack, outcome: PredictOutcome::Won, .. }
            )));
            // The secret and its reward's delayed effect are gone: p1's next turn draws only.
            assert!(s.state().secrets.is_none());
            assert!(s.state().delayed.is_empty());
            let hand = s.hand(P1).len();
            s.end_turn();
            s.end_turn();
            assert_eq!(s.hand(P1).len(), hand + 1);
        }

        #[test]
        fn r864_greed_beats_defend_and_defend_beats_attack() {
            let mut defended = table("defend", json!([]), json!([FILLER]), json!([]));
            defended.play(FORTIFY, json!({ "modes": ["greed"] }));
            assert!(defended.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Predicted { outcome: PredictOutcome::Won, .. }
            )));
            assert!(defended.state().secrets.is_none());

            let mut attacked = table("attack", json!([]), json!([FILLER]), json!([]));
            attacked.play(FORTIFY, json!({ "modes": ["defend"] }));
            assert!(attacked.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Predicted { outcome: PredictOutcome::Won, .. }
            )));
            assert!(attacked.state().secrets.is_none());
        }

        #[test]
        fn r864_the_same_guess_reveals_and_the_reward_lands() {
            let mut s = table("greed", json!([]), json!([FILLER]), json!([]));
            let hand = s.hand(P1).len();
            s.play(FORTIFY, json!({ "modes": ["greed"] }));
            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Predicted { outcome: PredictOutcome::Same, .. }
            )));
            assert_eq!(s.state().delayed.len(), 1);
            // Both players now read the choice, and the reward still lands on p1's next turn.
            assert_eq!(
                s.view(P2).opponent.secrets.expect("revealed to both")[0].choice,
                Some(SecretChoice::Greed)
            );
            s.end_turn();
            s.end_turn();
            assert_eq!(s.hand(P1).len(), hand + 3);
            assert!(s.state().secrets.is_none());
        }

        #[test]
        fn r864_losing_with_greed_costs_2_mana_next_turn_and_2_discards() {
            let mut s = table("attack", json!([]), json!([FILLER, FILLER, FILLER]), json!([]));
            s.play(FORTIFY, json!({ "modes": ["greed"] }));
            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Predicted { outcome: PredictOutcome::Lost, .. }
            )));
            assert_eq!(s.state().players[P2].mana.next_turn_mod, -2);
            // The played Fortify plus 2 random discards left 2 of the 5 (3 fillers, the link, a draw).
            assert_eq!(s.hand(P2).len(), 2);
            // The lost guess revealed the secret but kept its reward.
            assert_eq!(s.state().delayed.len(), 1);
        }

        #[test]
        fn r864_losing_with_attack_deals_8_to_your_side() {
            let mut s = table("defend", json!([]), json!([FILLER]), json!([MENACE]));
            s.play(FORTIFY, json!({ "modes": ["attack"] }));
            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Predicted { outcome: PredictOutcome::Lost, .. }
            )));
            s.expect_health(P2, 30 - 8);
            assert_eq!(s.unit(P2, 1).expect("the unit stands").damage, 8);
            assert_eq!(s.state().delayed.len(), 1);
        }

        #[test]
        fn r864_losing_with_defend_heals_and_armors_the_opponent() {
            let mut s = table("greed", json!([MENACE]), json!([FILLER]), json!([]));
            s.state_mut().players[P1].hero.health = 20;
            s.card_mut(MENACE).damage = 5;
            s.play(FORTIFY, json!({ "modes": ["defend"] }));
            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Predicted { outcome: PredictOutcome::Lost, .. }
            )));
            assert_eq!(s.state().players[P1].hero.health, 28);
            assert_eq!(s.view(P1).you.hero.armor, 2);
            let unit = s.card(MENACE);
            assert_eq!(unit.damage, 0);
            assert!(unit.granted_keywords.contains(&Keyword::Armor { n: 2 }));
        }

        #[test]
        fn r862_played_after_its_secret_resolved_it_does_nothing() {
            let mut s = table("greed", json!([]), json!([FILLER]), json!([]));
            // A resolved secret is removed, so a link naming it finds nothing — the same path as a
            // link naming an id no secret was ever filed under (a Fortify cannot sit out its own
            // secret's resolution: Temporary discards it at its holder's first turn end).
            s.card_mut(FORTIFY).memory.insert(SECRET_KEY.to_string(), json!("secret-999"));
            let health = s.state().players[P2].hero.health;
            let hand = s.hand(P2).len();
            s.play(FORTIFY, json!({ "modes": ["attack"] }));
            // No judgement, no penalty: the hand lost only the played card.
            assert!(
                !s.last_events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Predicted { .. }))
            );
            assert!(
                !s.last_events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::SecretRevealed { .. }))
            );
            assert_eq!(s.state().players[P2].hero.health, health);
            assert_eq!(s.hand(P2).len(), hand - 1);
            // The live secret is untouched by the stale guess.
            assert!(s.state().secrets.is_some());
            assert_eq!(s.state().delayed.len(), 1);
        }

        #[test]
        fn r862_with_no_link_it_does_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
                "p2": {
                    "hand": [{ "def": FORTIFY }, FILLER],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                },
            }));
            s.end_turn();
            let health = s.state().players[P2].hero.health;
            s.play(FORTIFY, json!({ "modes": ["attack"] }));
            assert!(
                !s.last_events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Predicted { .. }))
            );
            assert_eq!(s.state().players[P2].hero.health, health);
            assert_eq!(s.hand(P2).len(), 2);
        }

        #[test]
        fn r862_discarded_by_temporary_it_takes_all_three_penalties() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [FILLER],
                    "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                },
                "p2": {
                    "hand": [{ "def": FORTIFY }, FILLER, FILLER, FILLER],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                },
            }));
            s.end_turn();
            s.end_turn(); // p2's end of turn: Temporary discards the Fortify Mind.
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::Discarded { def_id, .. } if def_id == FORTIFY
            )));
            // Greed: 2 less mana next turn, and 2 random discards (the link, a draw, then 2 of 4).
            assert_eq!(s.state().players[P2].mana.next_turn_mod, -2);
            assert_eq!(s.hand(P2).len(), 2);
            // Attack: 8 to the discarder's hero.
            s.expect_health(P2, 30 - 8);
            // Defend: the opponent's hero gains 2 Armor.
            assert_eq!(s.view(P1).you.hero.armor, 2);
        }

        #[test]
        fn r862_discarded_by_an_effect_it_takes_all_three_penalties() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [FILLER],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                },
                "p2": {
                    "hand": [{ "def": WIND }, { "def": FORTIFY }, FILLER, FILLER],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                },
            }));
            s.end_turn();
            // Second Wind exiles p2's deck and discards their hand, the Fortify Mind included.
            s.play(WIND, json!({ "zone": 1 }));
            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Discarded { def_id, .. } if def_id == FORTIFY
            )));
            assert_eq!(s.state().players[P2].mana.next_turn_mod, -2);
            s.expect_health(P2, 30 - 8);
            assert_eq!(s.view(P1).you.hero.armor, 2);
        }

        #[test]
        fn r863_the_secrets_owner_playing_it_takes_the_penalty() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": MIND }, FILLER, FILLER, FILLER],
                    "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                },
                "p2": {
                    "hand": [{ "def": MIND }, FILLER],
                    "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                },
            }));
            s.play(MIND, json!({ "modes": ["attack"] })); // p1's own secret.
            s.end_turn();
            // p2 ties against it first, so the later discard at p2's turn end never comes to pass.
            s.play(FORTIFY, json!({ "modes": ["attack"] }));
            s.play(MIND, json!({ "modes": ["attack"] })); // p2's secret: p1 holds its Fortify.
            s.end_turn(); // p1's turn: p1's Attack reward resolves (8 to p2).
            s.expect_health(P2, 30 - 8);
            let hand = s.hand(P1).len();
            // p1 owns a secret and still takes their own guess's Greed penalty.
            s.play(FORTIFY, json!({ "modes": ["greed"] }));
            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Predicted { outcome: PredictOutcome::Lost, .. }
            )));
            assert_eq!(s.state().players[P1].mana.next_turn_mod, -2);
            assert_eq!(s.hand(P1).len(), hand - 3);
        }

        #[test]
        fn reads_its_numbers_through_param() {
            let mut s = table("defend", json!([]), json!([FILLER]), json!([]));
            // Only the linked Fortify is in p2's hand, so the tuning lands on it.
            step_param(s.card_mut(FORTIFY), "damage", 1);
            s.play(FORTIFY, json!({ "modes": ["attack"] }));
            s.expect_health(P2, 30 - 10);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn probe_temporary_discard() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [FILLER],
                    "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                },
                "p2": {
                    "hand": [{ "def": FORTIFY }, FILLER, FILLER, FILLER],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                },
            }));
            s.end_turn();
            s.end_turn();
            eprintln!("P1 mod: {}", s.state().players[P1].mana.next_turn_mod);
            eprintln!("P2 mod: {}", s.state().players[P2].mana.next_turn_mod);
            eprintln!("P1 hand: {}", s.hand(P1).len());
            eprintln!("P2 hand: {}", s.hand(P2).len());
            eprintln!("P1 health: {}", s.state().players[P1].hero.health);
            eprintln!("P2 health: {}", s.state().players[P2].hero.health);
            eprintln!("P1 armor: {}", s.view(P1).you.hero.armor);
            eprintln!(
                "predicted: {}",
                s.events().iter().filter(|e| matches!(e, GameEvent::Predicted { .. })).count()
            );
            panic!("probe");
        }

        #[test]
        fn radiant_discarded_it_takes_no_penalty() {
            let mut s = table("greed", json!([]), json!([FILLER]), json!([]));
            // The linked Fortify is the only one in p2's hand: make it the Radiant face.
            s.card_mut(FORTIFY).radiant = true;
            s.end_turn();
            s.end_turn(); // p2's end of turn: Temporary discards it, with no clause to answer.
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::Discarded { def_id, .. } if def_id == FORTIFY
            )));
            assert_eq!(s.hand(P2).len(), 2);
            s.expect_health(P2, 30);
            assert_eq!(s.state().players[P2].mana.next_turn_mod, 0);
            assert_eq!(s.view(P1).you.hero.armor, 0);
        }

        #[test]
        fn radiant_still_judges_its_guess() {
            let mut s = table("greed", json!([]), json!([FILLER]), json!([]));
            s.card_mut(FORTIFY).radiant = true;
            s.play(FORTIFY, json!({ "modes": ["attack"] }));
            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Predicted { outcome: PredictOutcome::Won, .. }
            )));
            assert!(s.state().secrets.is_none());
            assert!(s.state().delayed.is_empty());
        }
    }
}
