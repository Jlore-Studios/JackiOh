//! C #88 Siphon Squad (SPEC §8.6 row 88). (3) Field Trap, Rare.
//!   Base:    "Start of turn: Reveal.
//!            Aura: Enemy Units have −X Attack, where X is {multiplier}× their number.
//!            When your opponent has no Units, Tribute this." — ×2
//!   Radiant: "Start of turn: Reveal.
//!            Aura: Enemy Units have 0 Attack.
//!            When your opponent has no Units, Tribute this."
//!   Engine:  "A layer-5 aura (§10.4), attack floored at 0; the Radiant's "0 attack" sets attack last, after
//!            every other layer. Dormant until revealed: set and unrevealed, the aura takes nothing and the
//!            preview shows nothing (R1149000); revealed by its own start of turn or anything else (R686),
//!            or face-up, it works. The self-Tribute is a condition checked at every state check, the one
//!            right after it is set included, live while face-down (R403). The base face's `preview`
//!            (R280) shows X once it is revealed, to its controller only while it is not face-up
//!            (§10.8). Tunes: multiplier 2 ↑."
//!
//! Through patch v0.3.7b the aura worked from the moment the card was set (R403's first reading);
//! `script_v0_3_7b` keeps that for those versions' replays (R767000).

use jackioh_engine::effects::reveal;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-088";

/// The aura's and `tribute_when`'s argument, and the preview's context narrowed to it (`read_of`).
type Read<'a> = HookArgs<'a>;

fn read_of(c: ConditionContext<'_>) -> Read<'_> {
    HookArgs {
        state: c.state,
        self_: c.self_,
        radiant: c.radiant,
    }
}

fn enemy_units(read: Read<'_>) -> i32 {
    active_units_of(read.state, opponent_of(read.self_.controller)).len() as i32
}

/// X: {multiplier} times the Units the opponent controls now.
fn x_now(read: Read<'_>) -> i32 {
    param(&read, "multiplier") * enemy_units(read)
}

/// R1149000: set and neither revealed (R686) nor face-up, the card is dormant: no aura, no preview.
fn dormant(read: Read<'_>) -> bool {
    read.self_.zone.z() == ZoneName::Field
        && read.self_.face_up != Some(true)
        && read.self_.revealed != Some(true)
}

/// `live_face_down` is v0.3.7b's reading, which never slept: the aura worked from the moment it was set.
fn siphon(
    mod_: fn(Read<'_>) -> StatMod,
    preview: fn(Read<'_>) -> Vec<PreviewValue>,
    live_face_down: bool,
) -> Script {
    Script {
        // "Start of Turn: Reveal" (R686): at its controller's start of turn the card shows its face to
        // both players, and from then on its aura works (R1149000) — revealed is not face-up, and a
        // Field Trap fires face-down or up alike.
        start_of_turn: Some(hook(|_ctx| vec![reveal(Default::default())])),
        aura: Some(aura_hook(move |read| {
            if !live_face_down && dormant(read) {
                return vec![];
            }
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| {
                    unit.zone.z() == ZoneName::Field && unit.controller != read.self_.controller
                }),
                mod_: mod_(read),
            }]
        })),
        tribute_when: Some(read_hook(|read| enemy_units(read) == 0)),
        preview: Some(condition_hook(move |c| {
            let read = read_of(c);
            if !live_face_down && dormant(read) {
                return vec![];
            }
            preview(read)
        })),
        ..Script::default()
    }
}

fn scripts(live_face_down: bool) -> CardScripts {
    let base = siphon(
        |read| StatMod {
            attack: Some(-x_now(read)),
            ..StatMod::default()
        },
        |read| {
            vec![PreviewValue {
                label: "−X Attack".into(),
                value: x_now(read),
                display: None,
                ids: None,
            }]
        },
        live_face_down,
    );

    // The Radiant face has no X: its preview is empty, which is no preview (R280).
    let radiant = siphon(
        |_read| StatMod {
            set_attack: Some(0),
            ..StatMod::default()
        },
        |_read| vec![],
        live_face_down,
    );

    CardScripts { base, radiant }
}

pub fn script() -> CardScripts {
    scripts(false)
}

/// R767000: the scripts v0.3.7b and every version before it ran, live from the moment the card was set
/// (R403's first reading); `KEPT_SCRIPTS` folds those versions' replays under them.
pub fn script_v0_3_7b() -> CardScripts {
    scripts(true)
}

// SPEC §8.6 (R1149000): dormant while set and unrevealed, its aura and preview wake once it is revealed,
// at its start of turn or by anything else (R686); the self-Tribute is live face-down (R403). Enemy
// units have −X Attack floored at 0; unrevealed, the opponent's view shows no drop and never names the
// card (R33); preview shows X once revealed (§10.8, R280). Multiplier is tuned via param() (R386).
// Radiant sets attack to 0 after all other layers (§10.4), so their hits are no hits (R63).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SIPHON: &str = "classic-088";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const TIMMY: &str = "core-011"; // (1) Unit 3/3 Rush, First Strike.
    const WEAPONS: &str = "core-014"; // (4) Field Spell: Aura: your Units have +3 attack, Rush and First Strike.
    const FIENDER: &str = "core-092"; // (2) Unit 5/7 Stack.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    use crate::scenario;

    use crate::js;

    use crate::matches_object;

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(found) => found,
            None => panic!("missing: {what}"),
        }
    }

    fn attack_of(s: &Scenario, player: PlayerId, lane: i32) -> i32 {
        match s.unit(player, lane) {
            Some(unit) => s.stats(&unit).attack,
            None => panic!("no unit in {player} lane {lane}"),
        }
    }

    /// p1 sets Siphon Squad against p2's board.
    fn set_against(enemies: &[&str], radiant_face: bool, mine: &[&str]) -> Scenario {
        let field: Vec<Value> = enemies
            .iter()
            .enumerate()
            .map(|(at, enemy)| json!({ "def": enemy, "lane": at + 1 }))
            .collect();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": SIPHON, "radiant": radiant_face }, ANCHOR, HIT_JOB], "field": mine, "mana": 10 },
            "p2": { "hand": [ANCHOR], "field": field }
        }));
        s.play(SIPHON, json!({}));
        s
    }

    /// R686: the card shows its face, as its own start of turn or any other reveal leaves it.
    fn reveal_siphon(s: &mut Scenario) {
        s.card_mut(SIPHON).revealed = Some(true);
    }

    /// `set_against`, then revealed: the aura is awake (R1149000).
    fn revealed_against(enemies: &[&str], radiant_face: bool, mine: &[&str]) -> Scenario {
        let mut s = set_against(enemies, radiant_face, mine);
        reveal_siphon(&mut s);
        s
    }

    fn at(card: &CardInstance) -> Value {
        json!([{ "pick": "instance", "instanceId": card.id }])
    }

    /// base
    mod base {
        use super::*;

        /// R1149000: set and unrevealed, enemy Units keep their Attack; revealed, each takes −2 × 2
        #[test]
        fn r1149000_set_and_unrevealed_enemy_units_keep_their_attack_and_lose_it_once_revealed() {
            let mut s = set_against(&[VANILLA, MENACE], false, &[]);
            let siphon = s.card(SIPHON).clone();

            assert_ne!(s.card(&siphon).face_up, Some(true));
            assert_ne!(s.card(&siphon).revealed, Some(true));
            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. })));
            assert_eq!(attack_of(&s, P2, 1), 4);
            assert_eq!(attack_of(&s, P2, 2), 9);

            reveal_siphon(&mut s);
            assert_ne!(s.card(&siphon).face_up, Some(true));
            assert_eq!(attack_of(&s, P2, 1), 0);
            assert_eq!(attack_of(&s, P2, 2), 5);
        }

        /// R1149000: the base face's preview shows nothing while it is unrevealed, and X once it is
        #[test]
        fn r1149000_the_base_preview_shows_nothing_while_unrevealed() {
            let mut s = set_against(&[VANILLA, MENACE], false, &[]);
            let preview = |s: &Scenario| js(&s.view(P1))["you"]["backrow"][0].get("preview").cloned();

            assert_eq!(preview(&s), None);
            reveal_siphon(&mut s);
            let shown = must(preview(&s), "the preview once revealed");
            assert_eq!(shown[0]["value"], json!(4));
        }

        /// R767000: a game on v0.3.7 folds under the kept script, live from the moment the card is set
        /// (R403's first reading)
        #[test]
        fn r1149000_r767000_a_game_on_v0_3_7_folds_under_the_kept_script_live_while_set_and_unrevealed() {
            crate::register_all();
            let shipped = crate::shipped_catalog("v0.3.7").expect("v0.3.7 is carried");
            jackioh_engine::with_shipped(Some(shipped), || {
                let s = set_against(&[VANILLA, MENACE], false, &[]);
                assert_ne!(s.card(SIPHON).revealed, Some(true));
                assert_eq!(attack_of(&s, P2, 1), 0);
                assert_eq!(attack_of(&s, P2, 2), 5);
            });
        }

        /// R686 Start of Turn: Reveal — at its controller's next start of turn it shows its face, still armed
        #[test]
        fn r686_r1149000_start_of_turn_reveal_at_its_controller_s_next_start_of_turn_it_shows_its_face_and_wakes() {
            let mut s = set_against(&[VANILLA, MENACE], false, &[]);
            assert_ne!(s.card(SIPHON).revealed, Some(true));
            s.end_turn(); // p2's turn: not its controller's, nothing shows, and the aura sleeps.
            assert_ne!(s.card(SIPHON).revealed, Some(true));
            assert_eq!(attack_of(&s, P2, 1), 4);
            assert_eq!(attack_of(&s, P2, 2), 9);
            s.end_turn(); // p1's turn: the start of turn reveals it.
            assert_eq!(s.card(SIPHON).revealed, Some(true));
            assert_ne!(s.card(SIPHON).face_up, Some(true));
            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. })));
            // The opponent reads its face now, and the aura works.
            assert!(serde_json::to_string(&s.view(P2)).expect("serialisable").contains(SIPHON));
            assert_eq!(attack_of(&s, P2, 1), 0);
            assert_eq!(attack_of(&s, P2, 2), 5);
        }

        /// its own Units are untouched
        #[test]
        fn its_own_units_are_untouched() {
            let s = revealed_against(&[VANILLA], false, &[MENACE]);
            assert_eq!(attack_of(&s, P1, 1), 9);
            assert_eq!(attack_of(&s, P2, 1), 2);
        }

        /// X is recomputed on every change: one enemy Unit fewer, X drops from 4 to 2
        #[test]
        fn x_is_recomputed_on_every_change_one_enemy_unit_fewer_x_drops_from_4_to_2() {
            let mut s = revealed_against(&[VANILLA, MENACE], false, &[]);
            assert_eq!(attack_of(&s, P2, 2), 5);

            let vanilla = s.card(VANILLA).clone();
            s.play(HIT_JOB, json!({ "targets": at(&vanilla) }));

            assert_eq!(attack_of(&s, P2, 2), 7);
        }

        /// §10.4 attack floors at 0: three enemy Units, X = 6, a 3-attack Unit reads 0
        #[test]
        fn s10_4_attack_floors_at_0_three_enemy_units_x_6_a_3_attack_unit_reads_0() {
            let s = revealed_against(&[TIMMY, VANILLA, MENACE], false, &[]);
            assert_eq!(attack_of(&s, P2, 1), 0);
            assert_eq!(attack_of(&s, P2, 3), 3);
        }

        /// R403 right after it is set, with the opponent holding no Units, it Tributes itself
        #[test]
        fn r403_right_after_it_is_set_with_the_opponent_holding_no_units_it_tributes_itself() {
            let mut s = set_against(&[], false, &[]);
            let siphon = s.card(SIPHON).clone();

            s.expect_in_zone(&siphon, "graveyard");
            assert!(s.events().iter().any(
                |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if *instance_id == siphon.id)
            ));
        }

        /// R403 whenever the opponent's last Unit leaves, the next state check Tributes it
        #[test]
        fn r403_whenever_the_opponent_s_last_unit_leaves_the_next_state_check_tributes_it() {
            let mut s = set_against(&[VANILLA], false, &[]);
            let siphon = s.card(SIPHON).clone();
            s.expect_in_zone(&siphon, "field");

            let vanilla = s.card(VANILLA).clone();
            s.play(HIT_JOB, json!({ "targets": at(&vanilla) }));

            s.expect_in_zone(&siphon, "graveyard");
        }

        /// R33, R1149000: unrevealed, the opponent's view never names the card and shows no drop; revealed,
        /// it reads the card and the drop
        #[test]
        fn r33_r1149000_unrevealed_the_opponent_s_view_never_names_the_card_and_shows_no_drop() {
            let mut s = set_against(&[VANILLA, MENACE], false, &[]);
            let theirs = js(&s.view(P2));

            let text = serde_json::to_string(&theirs).expect("serialisable");
            assert!(!text.contains(SIPHON));
            assert!(!text.contains("Siphon Squad"));
            let menace = &theirs["you"]["units"][1];
            assert!(matches_object(menace, &json!({ "defId": MENACE, "attack": 9 })));

            reveal_siphon(&mut s);
            let theirs = js(&s.view(P2));
            assert!(serde_json::to_string(&theirs).expect("serialisable").contains(SIPHON));
            let menace = &theirs["you"]["units"][1];
            assert!(matches_object(menace, &json!({ "defId": MENACE, "attack": 5 })));
        }

        /// §3.2 R13 a Stack pile is one Unit: only its top counts toward X and takes the −X
        #[test]
        fn s3_2_r13_a_stack_pile_is_one_unit_only_its_top_counts_toward_x_and_takes_the_x() {
            let mut s = scenario(json!({
                "p1": { "hand": [SIPHON, ANCHOR] },
                "p2": { "hand": [ANCHOR], "field": [VANILLA, { "def": FIENDER, "stack": true }] }
            }));
            s.play(SIPHON, json!({}));
            reveal_siphon(&mut s);

            assert_eq!(attack_of(&s, P2, 1), 5 - 2);
        }

        /// its own controller's Units never count toward X
        #[test]
        fn its_own_controller_s_units_never_count_toward_x() {
            let s = revealed_against(&[MENACE], false, &[VANILLA, TIMMY]);
            assert_eq!(attack_of(&s, P2, 1), 7);
        }

        /// R386 an Upgrade makes it 3× the count
        #[test]
        fn r386_an_upgrade_makes_it_3_the_count() {
            let mut s = scenario(json!({
                "p1": { "hand": [SIPHON, ANCHOR] },
                "p2": { "hand": [ANCHOR], "field": [MENACE, { "def": VANILLA, "lane": 2 }] }
            }));
            step_param(s.card_mut(SIPHON), "multiplier", 1);
            s.play(SIPHON, json!({}));
            reveal_siphon(&mut s);

            assert_eq!(attack_of(&s, P2, 1), 3);
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// R1149000: set and unrevealed, enemy Units keep their Attack; revealed, still face-down, they have 0
        #[test]
        fn r1149000_radiant_unrevealed_keeps_attack_revealed_sets_0() {
            let mut s = set_against(&[VANILLA, MENACE], true, &[]);

            assert_ne!(s.card(SIPHON).face_up, Some(true));
            assert_eq!(attack_of(&s, P2, 1), 4);
            assert_eq!(attack_of(&s, P2, 2), 9);

            reveal_siphon(&mut s);
            assert_ne!(s.card(SIPHON).face_up, Some(true));
            assert_eq!(attack_of(&s, P2, 1), 0);
            assert_eq!(attack_of(&s, P2, 2), 0);
        }

        /// §10.4 the 0 is set after every other layer: a buffed Unit under a +3 attack aura still reads 0
        #[test]
        fn s10_4_the_0_is_set_after_every_other_layer_a_buffed_unit_under_a_3_attack_aura_still_reads_0() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": SIPHON, "radiant": true }, ANCHOR] },
                "p2": { "hand": [ANCHOR], "field": [MENACE], "backrow": [WEAPONS] }
            }));
            let menace = s.card(MENACE).id.clone();
            must(find_instance_mut(s.state_mut(), &menace), "the Menace").buffs.attack += 5;
            assert_eq!(attack_of(&s, P2, 1), 9 + 5 + 3);

            s.play(SIPHON, json!({}));
            reveal_siphon(&mut s);

            assert_eq!(attack_of(&s, P2, 1), 0);
        }

        /// R63 their hits are no hits: a 9/9's strike back deals nothing, and a 0-attack Unit can't attack at all
        #[test]
        fn r63_their_hits_are_no_hits_a_9_9_s_strike_back_deals_nothing_and_a_0_attack_unit_can_t_attack_at_all() {
            let mut s = revealed_against(&[MENACE], true, &[VANILLA]);
            let mine = s.card(VANILLA).clone();

            let menace = s.card(MENACE).clone();
            s.attack(&mine, &menace);

            assert!(!s.last_events().iter().any(
                |event| matches!(event, GameEvent::Damage { target_id, .. } if *target_id == mine.id)
            ));
            s.expect_stats(&mine, json!({ "health": 4 }));
            s.end_turn();
            assert!(!legal_actions(s.state(), P2).iter().any(|action| js(action)["type"] == json!("attack")));
            let menace = s.card(MENACE).clone();
            s.expect_refused_with(|s| s.attack(&menace, "hero"), "0 attack");
        }

        /// R403 it Tributes itself when the opponent controls no Units
        #[test]
        fn r403_it_tributes_itself_when_the_opponent_controls_no_units() {
            let mut empty = set_against(&[], true, &[]);
            empty.expect_in_zone(SIPHON, "graveyard");

            let mut s = set_against(&[VANILLA], true, &[]);
            let vanilla = s.card(VANILLA).clone();
            s.play(HIT_JOB, json!({ "targets": at(&vanilla) }));
            s.expect_in_zone(SIPHON, "graveyard");
        }

        /// its own Units keep their attack
        #[test]
        fn its_own_units_keep_their_attack() {
            let s = revealed_against(&[VANILLA], true, &[MENACE]);
            assert_eq!(attack_of(&s, P1, 1), 9);
        }

        /// R386 the multiplier changes nothing on the Radiant face
        #[test]
        fn r386_the_multiplier_changes_nothing_on_the_radiant_face() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": SIPHON, "radiant": true }, ANCHOR] },
                "p2": { "hand": [ANCHOR], "field": [MENACE] }
            }));
            step_param(s.card_mut(SIPHON), "multiplier", 1);
            s.play(SIPHON, json!({}));
            reveal_siphon(&mut s);

            assert_eq!(attack_of(&s, P2, 1), 0);
        }
    }
}
