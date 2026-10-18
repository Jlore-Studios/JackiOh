//! Meditative+ #24 Bob, Wisconsin's Strongest Soldier (SPEC §8.9 row 24, R1185003, R1185004,
//! R1174000, R81, R703, R596, R1242). (3) Unit, Legendary, 5/5 → 10/10.
//!   Base:    "Cry: Choose one: / Freeze all enemy Units. / Put an enemy Unit into your hand. Your
//!            opponent gets {coins|Coin|Coins}. / Discover a Unit. Make it Radiant. / Draw 1. Fill your
//!            hand with copies of it."
//!   Radiant: "Cry: Do all of these, in order: …" the same four.
//!
//! THE MODES are declared (R81), so the choice travels in the play and never pauses: `freeze`,
//! `take`, `discover`, `copies`. The take's target is declared with them, for that mode alone
//! (`forModes`), and needed (R703): with no enemy Unit the take is not offered, as Hearthstone greys
//! out a Choose One option with no target. The Radiant face declares no mode and does all four in
//! order, each its own step, its take's target declared with the play and not needed: with no enemy
//! Unit that step does nothing (R1185004).
//!   1. Freeze (§6.3, R1174000): every enemy Unit (`freeze_all`).
//!   2. The declared enemy Unit goes to your hand as yours (`take_to_hand`, R1185003: R78's reset, a
//!      token ceasing to exist, your full hand burning it), then The Coin to the opponent, `coins` of
//!      them. The Coins are the price of a declared take: given whenever a Unit was declared, even
//!      one gone by the time it resolves, and never when none was.
//!   3. Discover (§6.3) among three different Units of the sets that ship (R380), never Bob (R387),
//!      the pick added Radiant (M #88's shape).
//!   4. One draw, then copies of the card it put in the hand until the hand is full (`draw_priced`'s
//!      `fillWithCopies`, R596, R1242): a card cast on draw, burned or not drawn is copied by nothing.
//!
//! Every list is the same length whatever the board holds, so the Radiant list a Discover pauses
//! resumes where it stopped (R113).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-024";

/// The four modes, in the order the card prints them (R81).
const FREEZE: &str = "freeze";
const TAKE: &str = "take";
const DISCOVER: &str = "discover";
const COPIES: &str = "copies";

/// The Discover's resume step: the pick, added Radiant.
const PICKED: &str = "picked";
/// §6.3 Discover: one of 3.
const OFFERED: i32 = 3;
/// The Coin (§7).
const COIN: &str = "core-t-coin";

fn modes() -> Vec<ModeDecl> {
    vec![ModeDecl {
        kind: PromptKind::Mode,
        options: [FREEZE, TAKE, DISCOVER, COPIES].iter().map(|mode| mode.to_string()).collect(),
    }]
}

/// An enemy Unit, the top of its pile (R13).
fn an_enemy_unit() -> TargetDecl {
    TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))
}

fn freeze_enemies() -> Vec<Effect> {
    vec![freeze_all(json_as(json!({ "side": "enemy" })))]
}

/// Mode 2: the declared Unit to your hand, then the Coins — both only for a play that declared one.
fn take_and_pay(ctx: &EffectContext<'_>) -> Vec<Effect> {
    let declared = ctx
        .targets
        .iter()
        .any(|selection| matches!(selection, Selection::Instance { .. }));
    if !declared {
        return vec![];
    }
    let mut out = vec![take_to_hand(json_as(json!({ "target": { "of": "chosen" } })))];
    for _ in 0..param(ctx, "coins").max(0) {
        out.push(add_to_hand(json_as(json!({ "defId": COIN, "player": "enemy" }))));
    }
    out
}

fn discover_a_unit() -> Vec<Effect> {
    vec![discover_from_catalog(json_as(json!({
        "step": PICKED,
        "count": OFFERED,
        "query": { "type": "Unit" },
        "prompt": "Discover a Unit. Make it Radiant",
    })))]
}

fn draw_and_copy() -> Vec<Effect> {
    vec![draw_priced(json_as(json!({ "fillWithCopies": true })))]
}

/// The pick arrives as a `mode` selection carrying a catalog id (§10.6), added Radiant.
fn add_the_pick() -> Hook {
    hook(|ctx| {
        let Some(picked) = chosen_options(ctx).into_iter().next() else {
            return vec![];
        };
        vec![add_to_hand(json_as(json!({ "defId": picked, "radiant": true })))]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        modes: modes(),
        targets: vec![TargetDecl {
            for_modes: Some(vec![TAKE.to_string()]),
            required: Some(true),
            ..an_enemy_unit()
        }],
        cry: Some(hook(|ctx| match chosen_options(ctx).into_iter().next().as_deref() {
            Some(FREEZE) => freeze_enemies(),
            Some(TAKE) => take_and_pay(ctx),
            Some(DISCOVER) => discover_a_unit(),
            Some(COPIES) => draw_and_copy(),
            _ => vec![],
        })),
        resume: IndexMap::from([(PICKED, add_the_pick())]),
        ..Script::default()
    };
    let radiant = Script {
        targets: vec![an_enemy_unit()],
        cry: Some(hook(|ctx| {
            let mut out = freeze_enemies();
            out.extend(take_and_pay(ctx));
            out.extend(discover_a_unit());
            out.extend(draw_and_copy());
            out
        })),
        resume: IndexMap::from([(PICKED, add_the_pick())]),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// Meditative+ #24 Bob, Wisconsin's Strongest Soldier — SPEC §8.9 row 24, BUILD M11 row M+ 24: "mode 1
// freezes every enemy Unit (#1174's rule); mode 2 a declared enemy Unit goes to your hand as yours, a
// token ceasing to exist, and the opponent gets 3 Coins; mode 3 Discovers among 3 Units, the pick
// Radiant; mode 4 draws 1 and fills your hand with copies, a cast-on-draw card none; coins reads
// through `param()`; radiant 10/10 and the four in order, mode 2's target declared with the play".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    /// Core #8 Mr. Vanilla, a 4/4 Unit.
    const VANILLA: &str = "core-008";
    /// M #30.1 Love Bomb, cast on draw.
    const LOVE_BOMB: &str = "meditative-030-1";

    /// p1 holds Bob (its face as given) and a filler; p1's deck and both fields as given.
    fn bob(seed: &str, radiant: bool, p1_library: Value, p2_field: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": p1_library },
            "p2": { "hand": [FILLER], "field": p2_field, "library": [FILLER, FILLER, FILLER] },
        }))
    }

    fn taking(target: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": target.id }])
    }

    fn coins_of(s: &Scenario, player: PlayerId) -> usize {
        s.hand(player).iter().filter(|card| card.def_id == COIN).count()
    }

    /// The open Discover's options, by catalog id (§10.6: a pick is a `mode` selection carrying one).
    fn options(s: &Scenario) -> Vec<String> {
        let pending = s.state().pending.clone().expect("the Discover is open");
        pending
            .options
            .iter()
            .map(|option| match &option.selection {
                Selection::Mode { option } => option.clone(),
                other => panic!("not a Discover option: {other:?}"),
            })
            .collect()
    }

    fn first_option(s: &Scenario) -> String {
        options(s)[0].clone()
    }

    mod base {
        use super::*;

        #[test]
        fn r81_declares_four_modes_and_the_takes_target_alone() {
            let face = script().base;
            assert_eq!(face.modes[0].options, [FREEZE, TAKE, DISCOVER, COPIES]);
            assert_eq!(face.targets.len(), 1);
            assert_eq!(face.targets[0].for_modes.as_deref(), Some(&[TAKE.to_string()][..]));
        }

        #[test]
        fn r1174000_mode_1_freezes_every_enemy_unit() {
            let mut s = bob(
                "bob-freeze",
                false,
                json!([FILLER, FILLER]),
                json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }]),
            );
            s.play(ID, json!({ "zone": 1, "modes": [FREEZE] }));
            for lane in 1..=2 {
                assert!(s.unit(P2, lane).expect("an enemy Unit").frozen.is_some(), "lane {lane}");
            }
            assert!(s.unit(P1, 1).expect("Bob").frozen.is_none(), "Bob is not an enemy");
        }

        #[test]
        fn r1185003_mode_2_takes_the_declared_enemy_unit_into_your_hand_and_gives_3_coins() {
            let mut s = bob("bob-take", false, json!([FILLER, FILLER]), json!([{ "def": VANILLA, "lane": 3, "damage": 1 }]));
            let target = s.unit(P2, 3).expect("the enemy Unit");
            s.play(ID, json!({ "zone": 1, "modes": [TAKE], "targets": taking(&target) }));
            let taken = s.card(&target).clone();
            assert_eq!(taken.zone, Zone::Hand { player: P1 });
            assert_eq!((taken.owner, taken.damage), (P1, 0));
            assert_eq!(coins_of(&s, P2), 3);
            assert_eq!(coins_of(&s, P1), 0);
        }

        #[test]
        fn r1185003_a_token_taken_ceases_to_exist_and_the_coins_are_still_given() {
            let mut s = bob("bob-token", false, json!([FILLER, FILLER]), json!([{ "def": "core-t-rush", "lane": 1 }]));
            let token = s.unit(P2, 1).expect("the Rush Token");
            s.play(ID, json!({ "zone": 1, "modes": [TAKE], "targets": taking(&token) }));
            s.expect_in_zone(&token, "gone");
            assert_eq!(coins_of(&s, P2), 3);
        }

        #[test]
        fn r703_with_no_enemy_unit_the_take_is_not_offered_and_the_rest_are() {
            let s = bob("bob-no-target", false, json!([FILLER, FILLER]), json!([]));
            let me = s.hand(P1).into_iter().find(|card| card.def_id == ID).expect("Bob");
            let offered: Vec<Vec<String>> = legal_actions(s.state(), P1)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Play { instance_id, modes, .. } if instance_id == me.id => modes,
                    _ => None,
                })
                .collect();
            for mode in [FREEZE, DISCOVER, COPIES] {
                assert!(offered.iter().any(|modes| modes.first().map(String::as_str) == Some(mode)), "{mode}");
            }
            assert!(offered.iter().all(|modes| modes.first().map(String::as_str) != Some(TAKE)));
        }

        #[test]
        fn coins_reads_through_param() {
            let mut s = bob("bob-param", false, json!([FILLER, FILLER]), json!([{ "def": VANILLA, "lane": 1 }]));
            assert_eq!(crate::upgrade_number(&mut s, ID, "coins"), 2);
            let target = s.unit(P2, 1).expect("the enemy Unit");
            s.play(ID, json!({ "zone": 1, "modes": [TAKE], "targets": taking(&target) }));
            assert_eq!(coins_of(&s, P2), 2);
        }

        #[test]
        fn mode_3_discovers_among_3_different_units_and_the_pick_is_radiant() {
            let mut s = bob("bob-discover", false, json!([FILLER, FILLER]), json!([]));
            s.play(ID, json!({ "zone": 1, "modes": [DISCOVER] }));
            let ids = options(&s);
            assert_eq!(ids.len(), 3);
            for id in &ids {
                let def = def_of(Some(s.state()), id);
                assert_eq!(def.type_, CardType::Unit, "{id}");
                assert!(!def.token && id != ID, "{id}");
                assert!(set_ships(def.set), "{id}: a set that ships (R380)");
            }
            assert!(ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2]);
            let pick = first_option(&s);
            s.answer(json!(pick.clone()));
            let added = s.hand(P1).into_iter().find(|card| card.def_id == pick).expect("the pick");
            assert!(added.radiant, "Make it Radiant");
        }

        #[test]
        fn r1185004_mode_4_draws_1_and_fills_your_hand_with_copies_of_it() {
            let mut s = bob("bob-copies", false, json!([VANILLA, FILLER, FILLER]), json!([]));
            s.play(ID, json!({ "zone": 1, "modes": [COPIES] }));
            let hand = s.hand(P1);
            assert_eq!(hand.len() as i32, hand_cap_of(s.state(), P1));
            assert_eq!(hand.iter().filter(|card| card.def_id == VANILLA).count(), hand.len() - 1);
        }

        #[test]
        fn r596_mode_4_copies_nothing_when_the_draw_is_cast_on_draw() {
            let mut s = bob("bob-cast", false, json!([LOVE_BOMB]), json!([]));
            s.play(ID, json!({ "zone": 1, "modes": [COPIES] }));
            assert!(s.hand(P1).iter().all(|card| card.def_id != LOVE_BOMB));
            assert!(s.hand(P1).len() <= 2, "no copies: {:?}", s.hand(P1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1185004_does_all_four_in_order() {
            let mut s = bob(
                "bob-radiant",
                true,
                json!([VANILLA, FILLER, FILLER]),
                json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }]),
            );
            let target = s.unit(P2, 1).expect("the enemy Unit to take");
            let kept = s.unit(P2, 2).expect("the enemy Unit left");
            s.play(ID, json!({ "zone": 1, "targets": taking(&target) }));
            // 1, 2: frozen and taken, the Coins given, then 3: the Discover asks.
            assert!(s.card(&kept).frozen.is_some());
            assert_eq!(s.card(&target).zone, Zone::Hand { player: P1 });
            assert_eq!(coins_of(&s, P2), 3);
            let pick = first_option(&s);
            let before = s.hand(P1).len();
            s.answer(json!(pick.clone()));
            // 3's pick, Radiant, then 4: one draw and copies of it to a full hand.
            let hand = s.hand(P1);
            assert!(hand.iter().any(|card| card.def_id == pick && card.radiant));
            assert_eq!(hand.len() as i32, hand_cap_of(s.state(), P1));
            let drawn = hand
                .iter()
                .filter(|card| card.def_id == VANILLA && card.id != target.id && !card.radiant)
                .count();
            assert_eq!(drawn, hand.len() - before - 1, "the draw and its copies fill the rest");
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn r1185004_with_no_enemy_unit_the_take_does_nothing_and_no_coins_are_given() {
            let mut s = bob("bob-radiant-empty", true, json!([VANILLA, FILLER]), json!([]));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(coins_of(&s, P2), 0);
            let pick = first_option(&s);
            s.answer(json!(pick));
            assert_eq!(s.hand(P1).len() as i32, hand_cap_of(s.state(), P1));
        }
    }
}
