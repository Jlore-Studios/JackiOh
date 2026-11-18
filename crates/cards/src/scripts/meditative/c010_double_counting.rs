//! Meditative #10 Double Counting (SPEC §8.8 row 10; docs/meditative-set.md M6 #10; ME-TRIG (b); R820,
//! R822, R823, R4154211). (2) Field Spell, Epic.
//!   Base:    "Aura: Your Cry and Death effects trigger {extra|additional time|additional times}.\nActivate:
//!            Pay ({price}) to Discover a Cry or Death card." (extra 1, price 1)
//!   Radiant: "Aura: …\nActivate: Discover a Cry or Death card." (extra 2)
//!
//! The engine's Cry and Death multiplier (`Script.cry_death_extra`), Hearthstone's Brann Bronzebeard and
//! Baron Rivendare in one card: while this acts on its controller's field, the Cry of a permanent they
//! play or cast (§10.5 step 5) or trigger (C #54 Rewind) runs extra more times with the same choices,
//! and the Death of a card that died under their control runs extra more times on its snapshot (§4.5
//! step 3). A Spell's resolution is no Cry (R822). Several multipliers do not add: the highest holds (R820).
//! Issue #4154 added the Activate (R384, once a turn; the base face's price the declared `price`, R386):
//! a Discover (§6.3) of the shipped cards that print a Cry or a Death, read off each one's base face's
//! script — a Death on any card, a Cry on a Unit or a Field Spell, since a Spell's resolution is no Cry
//! (R822) — its pick to the hand on its base face (R4154211). Through v0.6.6g it had the Aura alone,
//! which `script_v0_6_6g` keeps (R767000).

use jackioh_engine::effects::{add_to_hand, discover_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-010";

/// The Activate's id, and the resume step the Discover's answer comes back to (§10.6).
const TALLY: &str = "tally";
const PICKED: &str = "picked";
/// "Pay (1)": the base face's price as written; the declared `price` is what is paid (R386).
const PRICE: i32 = 1;

/// R4154211: the shipped cards that print a Cry or a Death, off their base face's script.
fn cry_or_death_ids() -> Vec<String> {
    query(&CatalogQueryArgs::default())
        .into_iter()
        .filter(|def| {
            jackioh_engine::scripts::registered_entry(&def.id).is_some_and(|entry| {
                entry.base.death.is_some()
                    || (entry.base.cry.is_some() && matches!(def.type_, CardType::Unit | CardType::FieldSpell))
            })
        })
        .map(|def| def.id.clone())
        .collect()
}

/// "Activate: (Pay (1) to) Discover a Cry or Death card." (R384, R4154211).
fn tally(paid: bool) -> ActivationDecl {
    ActivationDecl {
        id: TALLY.to_string(),
        label: if paid {
            "Pay (1) to Discover a Cry or Death card".to_string()
        } else {
            "Discover a Cry or Death card".to_string()
        },
        uses: ActivationUses::Count(1),
        cost: paid.then(|| ActivationCost {
            mana: Some(PRICE),
            mana_param: Some("price"),
            ..ActivationCost::default()
        }),
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(|_ctx| {
            let mut discover: DiscoverFromCatalogArgs = json_as(json!({
                "step": PICKED,
                "prompt": "Discover a Cry or Death card",
            }));
            // Read as the Discover applies, never again when the hook is rebuilt to resume.
            discover.query_fn = Some(Arc::new(|_at: &mut EffectContext<'_>| {
                json_as::<CatalogQueryArgs>(json!({ "defId": cry_or_death_ids() }))
            }));
            vec![discover_from_catalog(discover)]
        }),
    }
}

/// The Aura; with `activate`, the Activate too, paid on the base face (`paid`).
fn counting(activate: bool, paid: bool) -> Script {
    Script {
        cry_death_extra: Some(read_hook(|args| param(&args, "extra"))),
        activations: if activate { vec![tally(paid)] } else { Vec::new() },
        resume: if activate {
            IndexMap::from([(
                PICKED,
                // A Discover's pick arrives as a `mode` selection carrying a catalog id (§10.6).
                hook(|ctx| {
                    let Some(picked) = chosen_options(ctx).into_iter().next() else {
                        return vec![];
                    };
                    vec![add_to_hand(json_as(json!({ "defId": picked })))]
                }),
            )])
        } else {
            IndexMap::new()
        },
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: counting(true, true),
        radiant: counting(true, false),
    }
}

/// R767000: the scripts through v0.6.6g, the Aura alone (issue #4154). `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_6g() -> CardScripts {
    CardScripts {
        base: counting(false, false),
        radiant: counting(false, false),
    }
}

// Meditative #10 Double Counting — SPEC §8.8 row 10, BUILD M10 row M 10.
// Fixtures: Core #68, #43, #11 and #73 and C+ #76, as their consts say.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SORCERER: &str = "core-068"; // Twisted Sorcerer: Unit. Cry: deal 4 damage to a declared target.
    const FELINOR: &str = "core-043"; // Big Felinor: no text that answers damage
    const TIMMY: &str = "stand-in (1) 3/3 Rush, First Strike R6/6 Charge, First Strike"; // one 4-hit kills it.
    const ARMOR: &str = "core-073"; // Anti-oneshot Armor: Field Spell. Cry: Draw 1.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const LAR: &str = "classicplus-076"; // Brother Lar: Unit. Death: Summon a Brother Ping.
    const PING: &str = "classicplus-076-1"; // Brother Ping
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy target Unit.
    const REWIND: &str = "classic-054"; // Rewind: trigger a Cry of one of your Units.
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-010"; // Rapid Replenish: a Spell, never played: it only keeps the turn open (R82)

    /// A side with mana to play and a stocked library, so no turn auto-ends (§2.5) or fatigues.
    fn side(extra: Value) -> Value {
        crate::merged(
            json!({ "mana": 10, "hand": [FILLER], "library": [FILLER, FILLER, FILLER] }),
            extra,
        )
    }

    fn damage_events(s: &Scenario, target_id: &str) -> usize {
        s.events()
            .iter()
            .filter(|event| {
                let event = crate::js(event);
                event["type"] == "damage" && event["targetId"] == target_id
            })
            .count()
    }

    fn pings(s: &Scenario, player: PlayerId) -> usize {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane))
            .filter(|unit| unit.def_id == PING)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r823_a_sorcerer_hits_its_declared_target_twice() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [SORCERER, FILLER] })),
                "p2": side(json!({ "field": [FELINOR] })),
            }));
            let victim = s.card(FELINOR).id.clone();
            s.play(SORCERER, json!({ "targets": pick::id(&victim) }));
            assert_eq!(damage_events(&s, &victim), 2);
            s.expect_stats(FELINOR, json!({ "health": 2, "maxHealth": 10 }));
        }

        #[test]
        fn r823_a_target_it_kills_is_not_hit_again() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [SORCERER, FILLER] })),
                "p2": side(json!({ "field": [TIMMY] })),
            }));
            let victim = s.card(TIMMY).id.clone();
            s.play(SORCERER, json!({ "targets": pick::id(&victim) }));
            s.expect_in_zone(TIMMY, "graveyard");
            assert_eq!(damage_events(&s, &victim), 1);
        }

        #[test]
        fn r823_a_field_spells_cry_draws_twice() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [ID],
                    "hand": [ARMOR, FILLER],
                    "library": [VANILLA, VANILLA, VANILLA],
                })),
                "p2": side(json!({})),
            }));
            s.play(ARMOR, json!({}));
            assert_eq!(s.hand(P1).len(), 3);
            assert_eq!(
                s.hand(P1).iter().filter(|card| card.def_id == VANILLA).count(),
                2
            );
        }

        #[test]
        fn r822_a_spells_resolution_runs_once() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [ID],
                    "hand": [STOCKPILE, FILLER],
                    "health": 20,
                    "library": [VANILLA, VANILLA, VANILLA],
                })),
                "p2": side(json!({})),
            }));
            s.play(STOCKPILE, json!({}));
            assert_eq!(s.hand(P1).len(), 3);
            s.expect_health(P1, 22);
        }

        #[test]
        fn r823_brother_lar_dies_into_two_brother_pings() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "field": [LAR], "hand": [HIT_JOB, FILLER] })),
                "p2": side(json!({})),
            }));
            let lar = s.card(LAR).id.clone();
            // Hit Job is a Spell, so its own resolution runs once (R822) while Lar's Death runs
            // twice (R823).
            s.play(HIT_JOB, json!({ "targets": pick::id(&lar) }));
            s.expect_in_zone(LAR, "graveyard");
            assert_eq!(pings(&s, P1), 2);
        }

        #[test]
        fn r823_the_opponents_brother_lar_dies_into_one() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "hand": [HIT_JOB, FILLER] })),
                "p2": side(json!({ "field": [LAR] })),
            }));
            let lar = s.card(LAR).id.clone();
            s.play(HIT_JOB, json!({ "targets": pick::id(&lar) }));
            assert_eq!(pings(&s, P2), 1);
            assert_eq!(pings(&s, P1), 0);
        }

        #[test]
        fn r823_a_rewound_cry_asks_once_and_hits_twice() {
            let mut s = scenario(json!({
                "p1": side(json!({ "backrow": [ID], "field": [SORCERER], "hand": [REWIND, FILLER] })),
                "p2": side(json!({})),
            }));
            let sorcerer = s.card(SORCERER).id.clone();
            s.play(REWIND, json!({ "targets": pick::id(&sorcerer) }));
            // The triggered Cry's declared target is asked once (R467); the extra run reuses it.
            assert!(s.state().pending.is_some());
            s.answer(json!("hero:p2"));
            assert!(s.state().pending.is_none());
            s.expect_health(P2, 22);
        }

        #[test]
        fn r820_two_double_countings_give_one_extra_not_two() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [ID, ID],
                    "hand": [ARMOR, FILLER],
                    "library": [VANILLA, VANILLA, VANILLA],
                })),
                "p2": side(json!({})),
            }));
            s.play(ARMOR, json!({}));
            assert_eq!(s.hand(P1).len(), 3);
        }

        #[test]
        fn r386_extra_reads_through_param_a_buff_gives_three_runs() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [ID],
                    "hand": [ARMOR, FILLER],
                    "library": [VANILLA, VANILLA, VANILLA, VANILLA],
                })),
                "p2": side(json!({})),
            }));
            assert_eq!(crate::upgrade_number(&mut s, ID, "extra"), 2);
            s.play(ARMOR, json!({}));
            assert_eq!(s.hand(P1).len(), 4);
        }
    }

    /// R4154211: p1's Double Counting on its backrow, with `mana`.
    fn tallying(radiant: bool, mana: i32) -> Scenario {
        let mut s = scenario(json!({
            "p1": side(json!({ "backrow": [{ "def": ID, "radiant": radiant }], "mana": mana })),
            "p2": side(json!({})),
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s
    }

    /// Whether `def_id`'s base face prints a Cry (on a Unit or a Field Spell) or a Death.
    fn prints_cry_or_death(def_id: &str) -> bool {
        let def = crate::card_def(def_id);
        let scripts = jackioh_engine::scripts::registered_entry(def_id).expect("a registered card");
        scripts.base.death.is_some()
            || (scripts.base.cry.is_some() && matches!(def.type_, CardType::Unit | CardType::FieldSpell))
    }

    #[test]
    fn r4154211_the_base_face_pays_1_to_discover_a_cry_or_death_card_once_a_turn() {
        let mut s = tallying(false, 5);
        s.activate(ID, json!({}));
        assert_eq!(s.state().players.p1.mana.current, 4, "it pays (1)");
        let options = s.prompt_modes();
        assert_eq!(options.len(), 3);
        for option in &options {
            assert!(prints_cry_or_death(option), "{option} prints a Cry or a Death");
            assert!(!crate::card_def(option).token, "{option} is no token");
        }
        let held = s.hand(P1).len();
        s.answer(json!(options[0]));
        let hand = s.hand(P1);
        assert_eq!(hand.len(), held + 1);
        let added = hand.last().expect("the pick");
        assert_eq!(added.def_id, options[0]);
        assert!(!added.radiant);
        s.expect_refused(|s| s.activate(ID, json!({})));
    }

    #[test]
    fn r4154211_the_base_face_needs_its_mana() {
        let mut s = tallying(false, 0);
        s.expect_refused(|s| s.activate(ID, json!({})));
    }

    #[test]
    fn r4154211_the_radiant_face_discovers_for_free() {
        let mut s = tallying(true, 0);
        s.activate(ID, json!({}));
        assert_eq!(s.state().players.p1.mana.current, 0);
        let options = s.prompt_modes();
        assert!(options.iter().all(|option| prints_cry_or_death(option)));
        s.answer(json!(options[0]));
        assert!(s.hand(P1).iter().any(|card| card.def_id == options[0] && !card.radiant));
    }

    /// R767000: through v0.6.6g it had no Activate (issue #4154).
    #[test]
    fn r767000_the_script_kept_for_v0_6_6g_has_no_activate() {
        let kept = super::script_v0_6_6g();
        assert!(kept.base.activations.is_empty() && kept.radiant.activations.is_empty());
        assert!(kept.base.cry_death_extra.is_some());
    }

    mod radiant {
        use super::*;

        #[test]
        fn r823_three_runs_in_all() {
            let mut s = scenario(json!({
                "p1": side(json!({
                    "backrow": [{ "def": ID, "radiant": true }],
                    "hand": [ARMOR, FILLER],
                    "library": [VANILLA, VANILLA, VANILLA, VANILLA],
                })),
                "p2": side(json!({})),
            }));
            s.play(ARMOR, json!({}));
            assert_eq!(s.hand(P1).len(), 4);
        }
    }
}
