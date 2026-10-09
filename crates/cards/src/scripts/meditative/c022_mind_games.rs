//! Meditative #22 Mind Games (SPEC §8.8 row 22, BUILD M10 row M 22): (2) Spell, Epic.
//!   Base:    "Choose one in secret: Greed, Attack or Defend. At the start of your next turn, gain
//!            its reward. Greed: Gain {mana} mana and draw {draw} cards. Attack: Deal {damage}
//!            damage to each enemy. Defend: Heal your hero and each of your Units {heal}. They gain
//!            +{armor} Armor. Add a Fortify Mind to your opponent's hand."
//!   Radiant: the same, ending "Add a Fortify Mind to your opponent's hand. It costs ({fortifyCost})."
//! The mode is declared with the play (R81) and kept secret (R860, R865): filed as a secret record
//! on the caster by `keep_secret`, never carried on an event. The reward is the caster's `next`
//! start-of-turn delayed effect (R861): after the refresh, before the triggers and the draw. A play
//! with no mode keeps no secret — that only happens in a redacted copy (R865) — and still hands the
//! opponent its Fortify Mind.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-022";

/// The token this hands the opponent (M #22.1).
const FORTIFY_MIND: &str = "meditative-022-1";

/// The `resume` step the reward re-enters.
const REWARD: &str = "reward";

/// `forEachCard`'s `cards`, typed (TS `(ctx) => readonly (CardInstance | string)[]`, ids here).
fn cards_of(f: impl Fn(&mut EffectContext<'_>) -> Vec<String> + Send + Sync + 'static) -> ForEachCardCards {
    Arc::new(f)
}

/// `forEachCard`'s `each`, typed (TS `(instanceId) => Effect`).
fn each_of(f: impl Fn(&str) -> Effect + Send + Sync + 'static) -> ForEachCardEach {
    Arc::new(f)
}

fn secret_modes() -> Vec<ModeDecl> {
    vec![ModeDecl {
        kind: PromptKind::Mode,
        options: SecretChoice::ALL.iter().map(|choice| choice.as_str().to_string()).collect(),
    }]
}

fn choice_of(mode: Option<String>) -> Option<SecretChoice> {
    mode?.parse::<SecretChoice>().ok()
}

/// The reward's Defend: heal the caster's hero and each of their Units, then grant the hero Armor
/// and each Unit Armor `n` (R59: one hit each, closed by one state check).
fn defend_side(heal_n: i32, armor_n: i32) -> Vec<Effect> {
    let mut out = vec![heal(json_as(json!({ "target": { "of": "selfHero" }, "amount": heal_n })))];
    out.push(for_each_card(ForEachCardArgs {
        cards: cards_of(|ctx| {
            active_units_of(ctx.state, ctx.controller).iter().map(|unit| unit.id.clone()).collect()
        }),
        each: each_of(move |instance_id| {
            heal(json_as(json!({
                "target": { "of": "instance", "instanceId": instance_id },
                "amount": heal_n,
            })))
        }),
    }));
    out.push(gain_hero_armor(json_as(json!({ "amount": armor_n }))));
    out.push(for_each_card(ForEachCardArgs {
        cards: cards_of(|ctx| {
            active_units_of(ctx.state, ctx.controller).iter().map(|unit| unit.id.clone()).collect()
        }),
        each: each_of(move |instance_id| {
            grant_keyword(json_as(json!({
                "target": { "of": "instance", "instanceId": instance_id },
                "keyword": { "kind": "Armor", "n": armor_n },
            })))
        }),
    }));
    out
}

pub fn script() -> CardScripts {
    let cry = |radiant: bool| {
        hook(move |ctx| {
            let Some(choice) = choice_of(chosen_options(ctx).into_iter().next()) else {
                // R865: a redacted copy has no mode to file, so no secret is kept — but the
                // Fortify Mind it would have linked still reaches the enemy hand.
                return vec![add_to_hand(json_as(json!({ "defId": FORTIFY_MIND, "player": "enemy" })))];
            };
            let cost_override = if radiant { Some(param(&*ctx, "fortifyCost")) } else { None };
            vec![keep_secret(json_as(json!({
                "choice": choice.as_str(),
                "step": REWARD,
                "link": { "defId": FORTIFY_MIND, "player": "enemy", "costOverride": cost_override },
            })))]
        })
    };
    let reward = hook(|ctx| {
        let Some(Value::String(secret_id)) = ctx.data.get(SECRET_KEY) else {
            return vec![];
        };
        let secret_id = secret_id.clone();
        let Some(choice) = secret_of(ctx.state, &secret_id).and_then(|secret| secret.choice) else {
            return vec![];
        };
        let mut out = vec![resolve_secret(json_as(json!({ "secretId": secret_id })))];
        match choice {
            SecretChoice::Greed => {
                out.push(gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))));
                out.push(draw(json_as(json!({ "count": param(&*ctx, "draw") }))));
            }
            SecretChoice::Attack => {
                out.push(damage_all(json_as(json!({
                    "amount": param(&*ctx, "damage"),
                    "side": "enemy",
                    "heroes": true,
                }))));
            }
            SecretChoice::Defend => {
                out.extend(defend_side(param(&*ctx, "heal"), param(&*ctx, "armor")));
            }
        }
        out
    });
    let mut base_resume = IndexMap::new();
    base_resume.insert(REWARD, reward);
    let base = Script {
        modes: secret_modes(),
        secret_modes: true,
        cry: Some(cry(false)),
        resume: base_resume.clone(),
        ..Script::default()
    };
    // The Radiant face prices the Fortify Mind it hands over; the reward matches the base face.
    let radiant = Script {
        modes: secret_modes(),
        secret_modes: true,
        cry: Some(cry(true)),
        resume: base_resume,
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// Meditative #22 Mind Games — SPEC §8.8 row 22, BUILD M10 row M 22: "(2) Epic Spell: the secret
// mode reaches no opponent view, event or log line until it is revealed; at the start of your next
// turn, after the refresh and before the start-of-turn triggers and the draw, the reward resolves
// (Greed 2 temporary mana and draw 2; Attack 8 to the enemy hero and each enemy Unit; Defend heal
// 8 on your hero and each of your Units, your hero +2 Armor and each Unit Armor 2); a Fortify Mind
// linked to the secret goes to the opponent's hand, burned at a full hand; a winning guess removes
// the reward; its numbers read through `param()`; radiant the Fortify Mind costs (2)".
#[cfg(test)]
mod tests {
    use super::{FORTIFY_MIND, ID};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::js;

    const MIND: &str = "meditative-022";
    const FORTIFY: &str = "meditative-022-1";
    const FILLER: &str = "core-005"; // (1) Spell.
    const MENACE: &str = "core-019"; // (3) 9/9.

    /// p1 plays Mind Games with `mode` (or the Radiant face), over `p1field`/`p2field` units.
    fn mind(mode: &str, radiant: bool, p1field: Value, p2field: Value) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": MIND, "radiant": radiant }, FILLER],
                "field": p1field,
                "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": {
                "hand": [FILLER],
                "field": p2field,
                "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
            },
        }));
        s.play(MIND, json!({ "modes": [mode] }));
        s
    }

    /// p2 answers with the same guess — a tie, so the reward still lands — and both turns end.
    fn tied(s: &mut Scenario, guess: &str) {
        s.end_turn();
        s.play(FORTIFY, json!({ "modes": [guess] }));
        s.end_turn();
    }

    fn fortify_in(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.hand(player).into_iter().filter(|card| card.def_id == FORTIFY).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn is_a_2_cost_epic_spell_with_its_numbers() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(2));
            assert_eq!(def.type_, CardType::Spell);
            assert_eq!(def.rarity, Rarity::Epic);
            assert!(def.tags.is_empty());
            assert_eq!(def.refs, Some(vec![FORTIFY_MIND.to_string()]));
            let params = def.params.clone().unwrap_or_default();
            let pair = |key: &str| {
                params.iter().find(|param| param.key == key).map(|param| (param.base, param.radiant))
            };
            assert_eq!(pair("mana"), Some((2, 2)));
            assert_eq!(pair("draw"), Some((2, 2)));
            assert_eq!(pair("damage"), Some((8, 8)));
            assert_eq!(pair("heal"), Some((8, 8)));
            assert_eq!(pair("armor"), Some((2, 2)));
            assert_eq!(pair("fortifyCost"), Some((2, 2)));
        }

        #[test]
        fn r860_the_opponent_sees_only_that_a_secret_is_held() {
            let s = mind("greed", false, json!([]), json!([]));
            let foe = s.view("p2");
            let held = foe.opponent.secrets.clone().expect("the opponent sees a secret is held");
            assert_eq!(held.len(), 1);
            assert_eq!(held[0].choice, None);
            // No event the opponent sees carries the choice — and `cardPlayed` carries no modes.
            let sent = js(&foe.events).to_string();
            assert!(!sent.contains("choice"));
            assert!(!sent.contains("modes"));
            assert!(!sent.contains("greed"));
            // The owner's view carries the choice.
            let own = s.view("p1");
            let mine = own.you.secrets.clone().expect("the owner reads their secret");
            assert_eq!(mine[0].choice, Some(SecretChoice::Greed));
        }

        #[test]
        fn r860_games_differing_only_in_the_mode_look_alike_to_the_opponent() {
            let a = mind("greed", false, json!([]), json!([]));
            let b = mind("defend", false, json!([]), json!([]));
            assert_eq!(a.view("p2"), b.view("p2"));
        }

        #[test]
        fn r861_the_reward_waits_through_the_opponents_turn() {
            let mut s = mind("attack", false, json!([]), json!([]));
            assert_eq!(s.state().delayed.len(), 1);
            s.end_turn(); // p2's turn: the secret is still held, the reward not yet resolved.
            assert!(s.state().secrets.is_some());
            assert_eq!(s.state().delayed.len(), 1);
            assert!(
                !s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::SecretRevealed { .. }))
            );
            assert_eq!(s.view("p2").opponent.secrets.expect("still held")[0].choice, None);
        }

        #[test]
        fn r861_greed_gains_2_temporary_mana_and_draws_2_before_the_turn_draw() {
            let mut s = mind("greed", false, json!([]), json!([]));
            let hand_after_play = s.hand(P1).len();
            tied(&mut s, "greed");
            // The reward's 2 draws plus the turn's own draw.
            assert_eq!(s.hand(P1).len(), hand_after_play + 3);
            let mana = &s.state().players[P1].mana;
            assert_eq!(mana.current, mana.max + 2);
            // The reveal came before the turn's draw: the last `drawn` is the turn draw.
            let types: Vec<GameEventType> =
                s.events().iter().map(|event| event.event_type()).collect();
            let revealed = types.iter().position(|type_| *type_ == GameEventType::SecretRevealed);
            let last_drawn = types.iter().rposition(|type_| *type_ == GameEventType::Drawn);
            assert!(revealed.expect("revealed") < last_drawn.expect("drawn"));
        }

        #[test]
        fn r861_attack_deals_8_to_the_enemy_hero_and_each_enemy_unit() {
            let mut s = mind("attack", false, json!([]), json!([MENACE, MENACE]));
            tied(&mut s, "attack");
            s.expect_health(P2, 30 - 8);
            for lane in 1..=2 {
                let unit = s.unit(P2, lane).expect("the unit stands");
                assert_eq!(unit.damage, 8);
            }
            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::SecretRevealed { choice: SecretChoice::Attack, .. }))
            );
        }

        #[test]
        fn r861_defend_heals_8_and_gives_2_armor() {
            let mut s = mind("defend", false, json!([MENACE]), json!([]));
            s.state_mut().players[P1].hero.health = 20;
            s.card_mut(MENACE).damage = 5;
            tied(&mut s, "defend");
            assert_eq!(s.state().players[P1].hero.health, 28);
            assert_eq!(s.view(P1).you.hero.armor, 2);
            let unit = s.card(MENACE);
            assert_eq!(unit.damage, 0);
            assert!(unit.granted_keywords.contains(&Keyword::Armor { n: 2 }));
            assert_eq!(
                s.events().iter().filter(|event| matches!(event, GameEvent::Healed { .. })).count(),
                2
            );
        }

        #[test]
        fn adds_a_linked_fortify_mind_costing_0_to_the_opponents_hand() {
            let s = mind("defend", false, json!([]), json!([]));
            let handed = fortify_in(&s, P2);
            assert_eq!(handed.len(), 1);
            let secret_id = s
                .state()
                .secrets
                .as_ref()
                .and_then(|secrets| secrets.first())
                .map(|secret| secret.id.clone())
                .expect("a secret is filed");
            assert_eq!(handed[0].memory.get(SECRET_KEY), Some(&json!(secret_id)));
            assert_eq!(handed[0].cost_override, None);
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.def_id == MIND));
        }

        #[test]
        fn a_full_hand_burns_the_fortify_mind_and_the_secret_stays() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": MIND }, FILLER],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                },
                "p2": {
                    "hand": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                },
            }));
            s.play(MIND, json!({ "modes": ["greed"] }));
            // The Fortify Mind burned: it reached no hand, and a burn is no discard (R862).
            assert!(fortify_in(&s, P2).is_empty());
            assert!(s.events().iter().any(|event| matches!(
                event,
                GameEvent::Burned { def_id, .. } if def_id == FORTIFY
            )));
            assert!(
                !s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Discarded { .. }))
            );
            assert!(s.state().secrets.is_some());
            assert_eq!(s.state().delayed.len(), 1);
        }

        #[test]
        fn reads_damage_through_param_when_tuned() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": MIND }, FILLER],
                    "library": [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER, FILLER] },
            }));
            step_param(s.card_mut(MIND), "damage", 1);
            s.play(MIND, json!({ "modes": ["attack"] }));
            tied(&mut s, "attack");
            s.expect_health(P2, 30 - 10);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_the_fortify_mind_costs_2() {
            let s = mind("greed", true, json!([]), json!([]));
            let handed = fortify_in(&s, P2);
            assert_eq!(handed.len(), 1);
            assert_eq!(handed[0].cost_override, Some(2));
        }

        #[test]
        fn radiant_greed_reward_matches_the_base() {
            let mut s = mind("greed", true, json!([]), json!([]));
            let hand_after_play = s.hand(P1).len();
            tied(&mut s, "greed");
            assert_eq!(s.hand(P1).len(), hand_after_play + 3);
            let mana = &s.state().players[P1].mana;
            assert_eq!(mana.current, mana.max + 2);
        }
    }
}
