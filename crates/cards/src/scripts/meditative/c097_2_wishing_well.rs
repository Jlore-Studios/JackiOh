//! M #97.2 Wishing Well (SPEC §8.8 row 97.2). (1) Unit, Token (printed Common), 0/6 → 0/12.
//!   Base:    "Can't attack\nActivate: {chance}% chance to add a random Radiant card to your hand."
//!   Radiant: "Can't attack, Lucky 1\nActivate: {chance}% chance to add a random Radiant card to your hand."
//!   Engine:  "Activate (§6.2, R384): its controller, in their main phase, once a turn, the turn it arrives
//!            included. One `rng.chance`, the Radiant face's Lucky 1 and its controller's Luck (R987)
//!            rolling it again and keeping a success (§6.1 Lucky); a success adds a random non-token card
//!            of every set (R380), Radiant, hidden in your hand (R97), burned by a full hand (§2.4).
//!            Tunes: chance 10 ↑ (Radiant 12, step 2); Lucky 1 ↑, on the Radiant face only."
//!
//! One script serves both faces: the Radiant face's Lucky 1 is a numbered keyword on the face, read as
//! Nerf and Buff have moved it, plus any Lucky given to the Well (`lucky_on`, R1438) and its controller's
//! Luck (R987), so with neither the base face rolls once and the Radiant face twice. `script_v0_3_7b`
//! keeps the roll without them for the replays of v0.3.7b and before (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-097-2";

/// §8.8: "Activate" is once a turn.
const USES: i32 = 1;

/// The declared `chance` is a percentage.
const PERCENT: i32 = 100;

/// Which Lucky the wish rolls with: `lucky_of` today, `printed_lucky_of` in the script v0.3.7b kept.
type LuckyOf = fn(&EffectContext<'_>) -> i32;

/// §6.1, R1438, R987: the Well's Lucky X, printed (as Nerf and Buff have moved it) plus given
/// (`lucky_on`), and its controller's Luck.
fn lucky_of(ctx: &EffectContext<'_>) -> i32 {
    ctx.self_.as_ref().map_or(0, |own| lucky_on(ctx.sink.state, own)) + luck_of(ctx.sink.state, ctx.controller)
}

/// R767000: the face's printed Lucky X alone, as Nerf and Buff have moved it: what the wish rolled with
/// through v0.3.7b (`script_v0_3_7b`).
fn printed_lucky_of(ctx: &EffectContext<'_>) -> i32 {
    let Some(own) = ctx.self_.as_ref() else {
        return 0;
    };
    numbered_keywords_on(ctx.sink.state, own)
        .into_iter()
        .find(|keyword| keyword.key == NumberedKey::Lucky)
        .map(|keyword| keyword.value)
        .unwrap_or(0)
}

fn wish(read_lucky: LuckyOf) -> ActivationDecl {
    ActivationDecl {
        id: "wish".to_string(),
        label: "Make a wish".to_string(),
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            let chance = f64::from(param(&*ctx, "chance")) / f64::from(PERCENT);
            let lucky = read_lucky(ctx);
            if ctx.rng.lucky(lucky, |rng| rng.chance(chance), |a, b| a || b) {
                vec![add_random_from_catalog(json_as(json!({ "radiant": true })))]
            } else {
                vec![]
            }
        }),
    }
}

/// One face: the Activate, its roll's Lucky read by `read_lucky`.
fn face(read_lucky: LuckyOf) -> Script {
    Script {
        activations: vec![wish(read_lucky)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = face(lucky_of);
    // The Radiant face's Lucky 1 and 0/12 are catalog data, and its 12% is the declared `chance`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: the scripts v0.3.7b and every version before it ran, whose roll read the printed Lucky
/// alone, without its controller's Luck (R987) or a given Lucky (R1438); `KEPT_SCRIPTS` folds those
/// versions' replays under them.
pub fn script_v0_3_7b() -> CardScripts {
    let base = face(printed_lucky_of);
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M 97.2 Wishing Well — SPEC §8.8 row 97.2, BUILD M10 row M 97.2: "Can't attack; Activate, once a turn in
// your main phase, the turn it arrives included: a 10% roll adds a random Radiant non-token card to your
// hand, a failure nothing; a second use that turn is refused; chance reads through `param()`; radiant 0/12,
// 12% with Lucky 1, two rolls keeping a success".
//
// The seed sweeps assert what holds for any seed (a miss draws once, or twice with Lucky 1; the Radiant
// face hits wherever the base face does), never what one seed drew. Every scenario keeps a 0-cost Spell
// in each hand and spares in the libraries, so R82 does not end the turn after the Activate.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use crate::js;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-010"; // (0) Spell.
    const SPARE: &str = "core-005"; // (1) Spell: library spare.

    /// Seeds per sweep.
    const SEEDS: usize = 200;

    /// The Well on p1's field, ready to activate.
    fn setup(seed: &str, radiant: bool) -> Scenario {
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": radiant }], "library": [SPARE, SPARE] },
            "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
        }))
    }

    /// One Activate at `seed` with the declared chance: whether a card came, and how far the rng moved.
    fn wish(seed: &str, radiant: bool) -> (bool, u32) {
        let mut s = setup(seed, radiant);
        let hand = s.hand(P1).len();
        let cursor = s.state().rng_cursor;
        s.activate(ID, json!({}));
        (s.hand(P1).len() > hand, s.state().rng_cursor - cursor)
    }

    /// Activates at chance 100 and returns the card that came.
    fn sure_wish(radiant: bool) -> (Scenario, CardInstance) {
        let mut s = setup("well-sure", radiant);
        set_param(s.card_mut(ID), "chance", 100);
        let before: Vec<String> = s.hand(P1).into_iter().map(|card| card.id).collect();
        s.activate(ID, json!({}));
        let added: Vec<CardInstance> = s
            .hand(P1)
            .into_iter()
            .filter(|card| !before.contains(&card.id))
            .collect();
        assert_eq!(added.len(), 1);
        let card = added[0].clone();
        (s, card)
    }

    #[test]
    fn is_a_1_cost_0_6_token_printed_common_with_one_activate_on_each_face_and_a_declared_chance() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, "meditative-097-2");
        assert_eq!(js(&def.cost), json!(1));
        assert!(def.token);
        assert_eq!(def.printed_rarity, Some(PrintedRarity::Common));
        assert_eq!(
            [
                js(&def.base.attack),
                js(&def.base.health),
                js(&def.radiant.attack),
                js(&def.radiant.health)
            ],
            [json!(0), json!(6), json!(0), json!(12)]
        );
        assert_eq!(js(&def.base.keywords), json!([{ "kind": "Can't attack" }]));
        assert_eq!(
            js(&def.radiant.keywords),
            json!([{ "kind": "Can't attack" }, { "kind": "Lucky", "n": 1 }])
        );
        assert_eq!(
            js(&def.params),
            json!([{ "key": "chance", "base": 10, "radiant": 12, "better": "up", "step": 2, "min": 1, "max": 100 }])
        );
        let scripts = script();
        let uses = |script: &Script| {
            script
                .activations
                .iter()
                .map(|ability| js(&ability.uses))
                .collect::<Vec<_>>()
        };
        assert_eq!(uses(&scripts.base), vec![json!(1)]);
        assert_eq!(uses(&scripts.radiant), vec![json!(1)]);
    }

    mod base {
        use super::*;

        #[test]
        fn r384_activates_the_turn_it_arrives_a_second_use_is_refused() {
            crate::register_all();
            let mut s = crate::scenario(json!({
                "p1": { "hand": [ID, FILLER], "library": [SPARE, SPARE] },
                "p2": { "hand": [FILLER], "library": [SPARE, SPARE] },
            }));
            s.play(ID, json!({}));
            s.activate(ID, json!({}));
            assert!(
                s.last_events()
                    .iter()
                    .any(|event| event.event_type() == GameEventType::Activated)
            );
            s.expect_refused(|s| s.activate(ID, json!({})));
        }

        #[test]
        fn cant_attack() {
            let mut s = setup("well-attack", false);
            s.expect_refused(|s| s.attack(ID, "hero"));
        }

        #[test]
        fn r97_r380_a_success_adds_one_random_radiant_non_token_card_hidden_from_the_opponent() {
            let (s, added) = sure_wish(false);
            let def = crate::card_def(&added.def_id);
            assert!(added.radiant);
            assert!(!def.token);
            assert!(jackioh_engine::set_ships(def.set));
            assert_eq!(s.hand(P1).len(), 2);
            assert!(serde_json::to_string(&s.view(P1).you.hand).is_ok_and(|hand| hand.contains(&added.id)));
            assert_eq!(s.view(P2).opponent.hand, HandView::Count { count: 2 });
            assert!(!serde_json::to_string(&s.view(P2)).is_ok_and(|view| view.contains(&added.id)));
        }

        #[test]
        fn the_10_percent_roll_hits_at_some_seeds_and_misses_at_others() {
            let (mut hits, mut misses) = (0, 0);
            for n in 0..SEEDS {
                let (hit, drawn) = wish(&format!("well-{n}"), false);
                if hit {
                    hits += 1;
                } else {
                    misses += 1;
                    // One `chance` roll, and nothing else drawn: the base face has no Lucky.
                    assert_eq!(drawn, 1);
                }
            }
            assert!(hits > 0 && misses > 0);
            assert!(hits < misses);
        }

        #[test]
        fn chance_reads_through_param_at_1_it_nearly_always_misses() {
            let mut hits = 0;
            for n in 0..SEEDS {
                let mut s = setup(&format!("well-low-{n}"), false);
                set_param(s.card_mut(ID), "chance", 1);
                let hand = s.hand(P1).len();
                s.activate(ID, json!({}));
                if s.hand(P1).len() > hand {
                    hits += 1;
                }
            }
            assert!(hits < SEEDS / 20);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn lucky_1_at_12_hits_wherever_the_base_10_hits_and_at_more_seeds() {
            let (mut base_hits, mut radiant_hits, mut extra) = (0, 0, 0);
            for n in 0..SEEDS {
                let seed = format!("well-{n}");
                let (base, _) = wish(&seed, false);
                let (radiant, drawn) = wish(&seed, true);
                // Lucky 1 rolls twice and keeps a success, at 12%, so the first roll alone already hits
                // wherever the base face's 10% does.
                assert!(
                    radiant || !base,
                    "{seed}: the base face hit and the Radiant face missed"
                );
                if !radiant {
                    assert_eq!(drawn, 2);
                }
                base_hits += usize::from(base);
                radiant_hits += usize::from(radiant);
                extra += usize::from(radiant && !base);
            }
            assert!(radiant_hits > base_hits);
            assert!(extra > 0);
        }

        #[test]
        fn a_radiant_face_at_100_adds_a_radiant_card_and_is_0_12() {
            let (mut s, added) = sure_wish(true);
            assert!(added.radiant);
            assert!(!crate::card_def(&added.def_id).token);
            s.expect_stats(ID, json!({ "attack": 0, "maxHealth": 12 }));
            let well = s.card(ID).clone();
            assert!(
                s.stats(&well)
                    .keywords
                    .iter()
                    .any(|keyword| keyword.kind() == KeywordKind::Lucky),
                "the Radiant Well keeps its Lucky"
            );
        }
    }

    // R987, R1438: the wish rolls once, once more for each Lucky on the Well, printed or given, and once
    // more for each point of its controller's Luck. At chance 1 nearly every seed misses, and a miss
    // draws one `chance` per roll and nothing else.
    mod luck {
        use super::*;

        const FENG_SHUI: &str = "meditative-040"; // (2) Field Spell: "Aura: You have Luck 1".

        /// Seeds per count: at chance 1 most of them miss.
        const LUCK_SEEDS: usize = 20;

        /// The draws one Activate at chance 1 takes wherever it misses (the same at every such seed),
        /// with Feng Shui in `feng_shui`'s backrow and Lucky `given` given to the Well.
        fn rolls(radiant: bool, feng_shui: Option<PlayerId>, given: i32) -> u32 {
            let backrow = |player: PlayerId| {
                if feng_shui == Some(player) {
                    json!([FENG_SHUI])
                } else {
                    json!([])
                }
            };
            let mut counts = Vec::new();
            for n in 0..LUCK_SEEDS {
                let mut s = crate::scenario(json!({
                    "seed": format!("well-luck-{n}"),
                    "p1": {
                        "hand": [FILLER],
                        "field": [{ "def": ID, "radiant": radiant }],
                        "backrow": backrow(P1),
                        "library": [SPARE, SPARE]
                    },
                    "p2": { "hand": [FILLER], "backrow": backrow(P2), "library": [SPARE, SPARE] },
                }));
                assert_eq!(luck_of(s.state(), P1), i32::from(feng_shui == Some(P1)));
                set_param(s.card_mut(ID), "chance", 1);
                if given > 0 {
                    crate::give_lucky(&mut s, ID, given);
                }
                let hand = s.hand(P1).len();
                let cursor = s.state().rng_cursor;
                s.activate(ID, json!({}));
                if s.hand(P1).len() == hand {
                    counts.push(s.state().rng_cursor - cursor);
                }
            }
            assert!(!counts.is_empty(), "chance 1 misses at some seed");
            assert!(counts.iter().all(|&count| count == counts[0]), "{counts:?}");
            counts[0]
        }

        #[test]
        fn r987_feng_shui_on_its_controller_s_side_rolls_once_more_on_each_face() {
            assert_eq!(rolls(false, None, 0), 1);
            assert_eq!(rolls(false, Some(P1), 0), 2);
            assert_eq!(rolls(true, None, 0), 2);
            assert_eq!(rolls(true, Some(P1), 0), 3);
        }

        #[test]
        fn r987_feng_shui_on_the_opponent_s_side_adds_no_roll() {
            assert_eq!(rolls(false, Some(P2), 0), 1);
            assert_eq!(rolls(true, Some(P2), 0), 2);
        }

        #[test]
        fn r1438_lucky_given_to_the_well_rolls_once_more() {
            assert_eq!(rolls(false, None, 1), 2);
            assert_eq!(rolls(true, None, 1), 3);
        }

        #[test]
        fn r987_r767000_a_game_on_v0_3_7_folds_under_the_kept_script_without_its_controller_s_luck() {
            crate::register_all();
            let shipped = crate::shipped_catalog("v0.3.7").expect("v0.3.7 is carried");
            jackioh_engine::with_shipped(Some(shipped), || {
                assert_eq!(rolls(false, Some(P1), 0), 1);
                assert_eq!(rolls(true, Some(P1), 0), 2);
            });
        }
    }
}
