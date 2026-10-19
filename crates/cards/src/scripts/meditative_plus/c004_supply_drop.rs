//! Meditative+ #4 Supply Drop (SPEC §8.9 row 4, BUILD M11 row M+ 4, R1181000, R1181001, R81, R1431):
//! (2, Embiggened 4) Spell, Jlockeed, Epic.
//!   Base:    "Choose one: In {turns} turns, deal {damage} damage to each enemy, with Pierce and
//!            Lifesteal; or in {turns} turns, add {cards} random Radiant Jlockeed cards to your hand,
//!            and each costs ({suppliesCost}). / Embiggened (4): In {embiggenedTurns} turns instead;
//!            deal {embiggenedDamage} damage, or add {embiggenedCards}." (3; 4, 1 at (2); 5; 9, 5)
//!   Radiant: the same, "… or fill your hand with them." (3; 9, 2 at (0); 5; 15, a full hand)
//!
//! THE MODES are declared (R81), so the choice travels in the play and never pauses: `combat` and
//! `supplies`. The Cry arms one delayed effect "in N turns" (R1181000): at the start of the caster's
//! Nth turn from now, counted in their own turns started (an extra turn counts), before the draw, with
//! a countdown badge on the caster's side in both views, since the Spell is in the graveyard by then.
//! The numbers are fixed as the Cry resolves (the Embiggened version's where it was played, R81) and
//! carried in the entry's data, so the step reads nothing off the card it no longer is.
//!   - Combat (R1181001): one hit on every enemy Unit and the enemy hero, Pierce and Lifesteal, all
//!     before one state check (R59), each hit healing the caster's hero.
//!   - Supplies (R1181001): random non-token Jlockeed cards of the sets that ship, never Supply Drop
//!     (R387), added Radiant at `suppliesCost`; the Radiant face's Embiggened version adds as many as
//!     fill the hand, counted as the step resolves (M #77's count).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-004";

/// The two modes, in the order the card prints them (R81); each is also its delayed step.
const COMBAT: &str = "combat";
const SUPPLIES: &str = "supplies";

/// The tag Supplies draws from.
const JLOCKEED: &str = "Jlockeed";

fn modes() -> Vec<ModeDecl> {
    vec![ModeDecl {
        kind: PromptKind::Mode,
        options: vec![COMBAT.to_string(), SUPPLIES.to_string()],
    }]
}

/// A declared number, the Embiggened version's where the card was played Embiggened (R81, R386).
fn number(ctx: &EffectContext<'_>, normal: &str, embiggened: &str) -> i32 {
    param(ctx, if ctx.embiggened { embiggened } else { normal })
}

/// A number the Cry stored in the delayed entry's data.
fn stored(ctx: &EffectContext<'_>, key: &str) -> Option<i32> {
    ctx.data
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

/// The data the chosen mode's step reads, and its countdown's caption (R1181000).
fn armed(ctx: &EffectContext<'_>, mode: &str, radiant: bool) -> Option<(Value, String)> {
    let name = def_of(Some(&*ctx.state), ID).name.clone();
    match mode {
        COMBAT => {
            let amount = number(ctx, "damage", "embiggenedDamage");
            Some((json!({ "amount": amount }), format!("{name}: {amount} damage to each enemy")))
        }
        // "… or fill your hand with them": counted as the step resolves.
        SUPPLIES if radiant && ctx.embiggened => Some((
            json!({ "fill": true, "cost": param(ctx, "suppliesCost") }),
            format!("{name}: a hand of Radiant Jlockeed cards"),
        )),
        SUPPLIES => {
            let count = number(ctx, "cards", "embiggenedCards");
            let noun = if count == 1 { "card" } else { "cards" };
            Some((
                json!({ "count": count, "cost": param(ctx, "suppliesCost") }),
                format!("{name}: {count} Radiant Jlockeed {noun}"),
            ))
        }
        _ => None,
    }
}

/// Combat: every enemy Unit, then the enemy hero, Pierce and Lifesteal (R1181001).
fn combat_step(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(amount) = stored(ctx, "amount") else {
        return vec![];
    };
    vec![damage_all(json_as(json!({
        "amount": amount,
        "side": "enemy",
        "heroes": true,
        "ignoreArmor": true,
        "lifesteal": true,
    })))]
}

/// Supplies: random Radiant Jlockeed cards at the stored price, never this card (R387).
fn supplies_step(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(cost) = stored(ctx, "cost") else {
        return vec![];
    };
    let count = if ctx.data.get("fill").and_then(Value::as_bool) == Some(true) {
        hand_cap_of(ctx.state, ctx.controller) - zone_count(ctx.state, ctx.controller, OffFieldZone::Hand)
    } else {
        stored(ctx, "count").unwrap_or(0)
    };
    if count <= 0 {
        return vec![];
    }
    vec![add_random_from_catalog(json_as(json!({
        "query": { "tags": [JLOCKEED], "excludeDefId": ID },
        "count": count,
        "radiant": true,
        "costOverride": cost,
    })))]
}

fn supply_drop(radiant: bool) -> Script {
    Script {
        modes: modes(),
        cry: Some(hook(move |ctx| {
            let Some(mode) = chosen_options(ctx).into_iter().next() else {
                return vec![];
            };
            let Some((data, label)) = armed(ctx, &mode, radiant) else {
                return vec![];
            };
            vec![delay(json_as(json!({
                "at": { "phase": "start", "player": "self" },
                "step": mode,
                "hook": RESUME_HOOK,
                "data": data,
                "turns": number(ctx, "turns", "embiggenedTurns"),
                "label": label,
            })))]
        })),
        resume: IndexMap::from([(COMBAT, hook(combat_step)), (SUPPLIES, hook(supplies_step))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: supply_drop(false),
        radiant: supply_drop(true),
    }
}

// Meditative+ #4 Supply Drop — SPEC §8.9 row 4, BUILD M11 row M+ 4: "Combat: nothing on the next two
// of your turns, then at the start of your third, before the draw, 4 Pierce damage to the enemy hero
// (through Armor) and each enemy Unit, healing your hero 4 per hit, one state check; the countdown is
// public on your side; an extra turn counts; Supplies: at the same moment a random non-token Jlockeed
// card, Radiant, costing (2), never Supply Drop; Embiggened at (4): after 5 turns, 9 damage or 5
// cards; every number reads through `param()`; radiant Combat 9 (Embiggened 15), Supplies 2 cards at
// (0) (Embiggened: as many as fill your hand)".
#[cfg(test)]
mod tests {
    use super::{COMBAT, ID, SUPPLIES};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A (1) 4/4 with no text: the bodies and the cards to draw.
    const FILLER: &str = "core-008";
    /// (2) 3/2, Spell Damage +2.
    const SOLARIUS: &str = "classicplus-038";

    fn library() -> Value {
        json!([FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER])
    }

    /// p1 holds Supply Drop (Radiant on `radiant`) and one filler, at 20 health, with `p1` merged in;
    /// p2 has Armor 3 and two 4/4s, with `p2` merged in.
    fn game(seed: &str, radiant: bool, p1: Value, p2: Value) -> Scenario {
        let p1 = crate::merged(
            json!({
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": library(),
                "mana": 4,
                "health": 20,
            }),
            p1,
        );
        let p2 = crate::merged(
            json!({
                "hand": [FILLER],
                "library": library(),
                "armor": 3,
                "field": [{ "def": FILLER, "lane": 1 }, { "def": FILLER, "lane": 2 }],
            }),
            p2,
        );
        crate::scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
    }

    fn cast(s: &mut Scenario, mode: &str, embiggen: bool) {
        let mut action = json!({ "modes": [mode] });
        if embiggen {
            action["embiggen"] = json!(true);
        }
        s.play(ID, action);
    }

    /// The hits in these events, as (target, amount).
    fn hits(events: &[GameEvent]) -> Vec<(String, i32)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } => Some((target_id.clone(), *amount)),
                _ => None,
            })
            .collect()
    }

    /// Pass `turns` of p1's turns: end p1's turn and p2's, `turns` times.
    fn pass_own_turns(s: &mut Scenario, turns: usize) {
        for _ in 0..turns {
            s.end_turn().end_turn();
        }
    }

    /// The labels of the badges `viewer` sees on p1's seat.
    fn p1_badges(s: &Scenario, viewer: PlayerId) -> Vec<String> {
        let view = s.view(viewer);
        let seat = if viewer == P1 { view.you } else { view.opponent };
        seat.modifiers.into_iter().map(|badge| badge.label).collect()
    }

    /// The cards in p1's hand that Supplies added.
    fn supplies(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .collect()
    }

    fn health(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.health
    }

    /// Supplies' cards: Radiant non-token Jlockeed cards of an open set at `cost`, never Supply Drop.
    fn assert_supplies(cards: &[CardInstance], cost: i32) {
        for card in cards {
            let def = crate::card_def(&card.def_id);
            assert!(def.tags.contains(&Tag::Jlockeed), "{} is Jlockeed", card.def_id);
            assert!(!def.is_token(), "{} is no token", card.def_id);
            assert_ne!(card.def_id, ID, "never Supply Drop itself (R387)");
            assert!(card.radiant, "{} is Radiant", card.def_id);
            assert_eq!(card.cost_override, Some(cost), "{} costs ({cost})", card.def_id);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r1181001_combat_waits_two_turns_then_hits_each_enemy_through_armor_before_the_draw() {
            let mut s = game("m+004-combat", false, json!({}), json!({}));
            let lane1 = s.unit(P2, 1).expect("p2's first 4/4").id;
            let lane2 = s.unit(P2, 2).expect("p2's second 4/4").id;
            cast(&mut s, COMBAT, false);
            pass_own_turns(&mut s, 2);
            assert!(hits(s.events()).is_empty(), "nothing on the next two of p1's turns");
            assert_eq!(health(&s, P2), 30);

            pass_own_turns(&mut s, 1);
            let events = s.last_events().to_vec();
            assert_eq!(
                hits(&events),
                vec![(lane1.clone(), 4), (lane2.clone(), 4), ("hero-p2".to_string(), 4)],
                "each enemy Unit, then the enemy hero"
            );
            assert_eq!(health(&s, P2), 26, "Pierce: the Armor takes none of it");
            assert_eq!(s.state().players[P2].hero.armor, 3);
            assert_eq!(health(&s, P1), 32, "Lifesteal heals p1's hero 4 per hit");
            let last_hit = events
                .iter()
                .rposition(|event| matches!(event, GameEvent::Damage { .. }))
                .expect("a hit");
            let first_death = events
                .iter()
                .position(|event| matches!(event, GameEvent::Destroyed { .. }))
                .expect("the 4/4s die");
            assert!(last_hit < first_death, "every hit before one state check (R59)");
            let draw = events
                .iter()
                .position(|event| matches!(event, GameEvent::Drawn { player, .. } if *player == P1))
                .expect("p1 draws");
            assert!(last_hit < draw, "before the draw");
            s.expect_in_zone(lane1.as_str(), "graveyard");
            s.expect_in_zone(lane2.as_str(), "graveyard");
        }

        #[test]
        fn r1181000_the_countdown_is_a_badge_on_the_casters_side_in_both_views() {
            let mut s = game("m+004-badge", false, json!({}), json!({}));
            cast(&mut s, COMBAT, false);
            let label = "Supply Drop: 4 damage to each enemy in 3 turns".to_string();
            assert_eq!(p1_badges(&s, P1), vec![label.clone()]);
            assert_eq!(p1_badges(&s, P2), vec![label]);
            pass_own_turns(&mut s, 1);
            assert_eq!(
                p1_badges(&s, P2),
                vec!["Supply Drop: 4 damage to each enemy in 2 turns".to_string()]
            );
            pass_own_turns(&mut s, 2);
            assert!(p1_badges(&s, P1).is_empty(), "the badge goes as the effect resolves");
        }

        #[test]
        fn r1181000_an_extra_turn_counts() {
            let mut s = game("m+004-extra", false, json!({}), json!({}));
            cast(&mut s, COMBAT, false);
            s.state_mut().players[P1].extra_turns = Some(1);
            // p1's extra turn is the first of the three; then p2's, p1's, p2's, p1's.
            s.end_turn();
            assert!(hits(s.events()).is_empty());
            s.end_turn().end_turn().end_turn();
            assert!(hits(s.events()).is_empty(), "two of p1's turns have started");
            s.end_turn();
            assert_eq!(health(&s, P2), 26, "the third start of p1's turns, the extra one counted");
        }

        #[test]
        fn r1181001_the_hits_carry_spell_damage() {
            let mut s = game(
                "m+004-spell-damage",
                false,
                json!({ "field": [{ "def": SOLARIUS, "lane": 3 }] }),
                json!({ "field": [] }),
            );
            cast(&mut s, COMBAT, false);
            pass_own_turns(&mut s, 3);
            assert_eq!(health(&s, P2), 24, "4 damage and Spell Damage +2 (R1180000)");
        }

        #[test]
        fn r1181001_supplies_adds_a_radiant_jlockeed_card_at_2_never_supply_drop() {
            let _preview = preview_sets(&[SetName::MeditativePlus]);
            let mut s = game("m+004-supplies", false, json!({}), json!({}));
            cast(&mut s, SUPPLIES, false);
            pass_own_turns(&mut s, 2);
            assert!(supplies(&s).is_empty(), "nothing on the next two of p1's turns");
            pass_own_turns(&mut s, 1);
            let added = supplies(&s);
            assert_eq!(added.len(), 1);
            assert_supplies(&added, 2);
        }

        #[test]
        fn r387_supplies_never_adds_supply_drop_over_five_seeds() {
            let _preview = preview_sets(&[SetName::MeditativePlus]);
            for seed in 0..5 {
                let mut s = game(
                    &format!("m+004-never-itself-{seed}"),
                    true,
                    json!({}),
                    json!({}),
                );
                cast(&mut s, SUPPLIES, true);
                pass_own_turns(&mut s, 5);
                let added = supplies(&s);
                assert_eq!(added.len(), 5, "the hand of 5 filled to 10");
                assert_supplies(&added, 0);
            }
        }

        #[test]
        fn embiggened_combat_waits_five_turns_and_deals_9() {
            let mut s = game("m+004-embiggened-combat", false, json!({}), json!({ "field": [] }));
            cast(&mut s, COMBAT, true);
            pass_own_turns(&mut s, 4);
            assert_eq!(health(&s, P2), 30, "nothing for four of p1's turns");
            pass_own_turns(&mut s, 1);
            assert_eq!(health(&s, P2), 21);
        }

        #[test]
        fn embiggened_supplies_adds_5_cards_after_five_turns() {
            let _preview = preview_sets(&[SetName::MeditativePlus]);
            let mut s = game("m+004-embiggened-supplies", false, json!({}), json!({}));
            cast(&mut s, SUPPLIES, true);
            pass_own_turns(&mut s, 4);
            assert!(supplies(&s).is_empty());
            pass_own_turns(&mut s, 1);
            let added = supplies(&s);
            assert_eq!(added.len(), 5);
            assert_supplies(&added, 2);
        }

        #[test]
        fn r386_an_upgrade_raises_the_damage_and_a_degrade_lengthens_the_wait() {
            let mut s = game("m+004-tuned", false, json!({}), json!({ "field": [] }));
            let drop = s.hand(P1)[0].id.clone();
            assert_eq!(crate::upgrade_number(&mut s, &drop, "damage"), 5);
            assert_eq!(crate::degrade_number(&mut s, &drop, "turns"), 4);
            cast(&mut s, COMBAT, false);
            pass_own_turns(&mut s, 3);
            assert_eq!(health(&s, P2), 30, "four turns now");
            pass_own_turns(&mut s, 1);
            assert_eq!(health(&s, P2), 25);
        }

        #[test]
        fn r386_supplies_cards_and_cost_read_through_param() {
            let _preview = preview_sets(&[SetName::MeditativePlus]);
            let mut s = game("m+004-tuned-supplies", false, json!({}), json!({}));
            let drop = s.hand(P1)[0].id.clone();
            assert_eq!(crate::upgrade_number(&mut s, &drop, "cards"), 2);
            assert_eq!(crate::upgrade_number(&mut s, &drop, "suppliesCost"), 1);
            cast(&mut s, SUPPLIES, false);
            pass_own_turns(&mut s, 3);
            let added = supplies(&s);
            assert_eq!(added.len(), 2);
            assert_supplies(&added, 1);
        }

        #[test]
        fn r386_the_embiggened_combat_numbers_read_through_param() {
            let mut s = game("m+004-tuned-embiggened-combat", false, json!({}), json!({ "field": [] }));
            let drop = s.hand(P1)[0].id.clone();
            assert_eq!(crate::upgrade_number(&mut s, &drop, "embiggenedDamage"), 10);
            assert_eq!(crate::degrade_number(&mut s, &drop, "embiggenedTurns"), 6);
            cast(&mut s, COMBAT, true);
            pass_own_turns(&mut s, 5);
            assert_eq!(health(&s, P2), 30, "six turns now");
            pass_own_turns(&mut s, 1);
            assert_eq!(health(&s, P2), 20);
        }

        #[test]
        fn r386_the_embiggened_supplies_count_reads_through_param() {
            let _preview = preview_sets(&[SetName::MeditativePlus]);
            let mut s = game("m+004-tuned-embiggened-supplies", false, json!({}), json!({}));
            let drop = s.hand(P1)[0].id.clone();
            assert_eq!(crate::degrade_number(&mut s, &drop, "embiggenedCards"), 4);
            assert_eq!(crate::upgrade_number(&mut s, &drop, "embiggenedTurns"), 4);
            cast(&mut s, SUPPLIES, true);
            pass_own_turns(&mut s, 3);
            assert!(supplies(&s).is_empty(), "four turns now");
            pass_own_turns(&mut s, 1);
            let added = supplies(&s);
            assert_eq!(added.len(), 4);
            assert_supplies(&added, 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn combat_deals_9() {
            let mut s = game("m+004-radiant-combat", true, json!({}), json!({ "field": [] }));
            cast(&mut s, COMBAT, false);
            pass_own_turns(&mut s, 3);
            assert_eq!(health(&s, P2), 21);
            assert_eq!(health(&s, P1), 29, "Lifesteal heals the 9");
        }

        #[test]
        fn embiggened_combat_deals_15_after_five_turns() {
            let mut s = game("m+004-radiant-embiggened", true, json!({}), json!({ "field": [] }));
            cast(&mut s, COMBAT, true);
            pass_own_turns(&mut s, 4);
            assert_eq!(health(&s, P2), 30);
            pass_own_turns(&mut s, 1);
            assert_eq!(health(&s, P2), 15);
        }

        #[test]
        fn supplies_adds_2_cards_at_0() {
            let _preview = preview_sets(&[SetName::MeditativePlus]);
            let mut s = game("m+004-radiant-supplies", true, json!({}), json!({}));
            cast(&mut s, SUPPLIES, false);
            pass_own_turns(&mut s, 3);
            let added = supplies(&s);
            assert_eq!(added.len(), 2);
            assert_supplies(&added, 0);
        }

        #[test]
        fn r1181001_embiggened_supplies_fills_the_hand_counted_as_it_resolves() {
            let _preview = preview_sets(&[SetName::MeditativePlus]);
            let mut s = game("m+004-radiant-fill", true, json!({}), json!({}));
            cast(&mut s, SUPPLIES, true);
            assert_eq!(
                p1_badges(&s, P2),
                vec!["Supply Drop: a hand of Radiant Jlockeed cards in 5 turns".to_string()]
            );
            pass_own_turns(&mut s, 5);
            // The fill resolves before the draw, so the hand is full and the draw burns.
            assert_eq!(s.hand(P1).len(), 10);
            assert_supplies(&supplies(&s), 0);
        }
    }
}
