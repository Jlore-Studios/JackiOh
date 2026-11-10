//! SPEC §8.12 S+ #160 Icarus (Unit 2/8 → 4/16, cost 4, Legendary), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Tribute 3
//!            Activate 1: Summon a Radi. It triggers its Cry.
//!            Cry: Discover an Epic Weapon; Equip it into this."
//!   Radiant: "Tribute 3
//!            Activate 1: Summon a Radiant Radi. It triggers its Cry.
//!            Cry: Discover a Legendary Weapon; Equip it into this."
//!
//! Tribute 3 is the play validator's (§6.3, R101), read off `staticFlags.tribute` (R366). The Cry is
//! `equip_picks` (R1841002, R1862002), one Discover of three from the Weapon-tagged cards of the rarity
//! (Epic, Radiant Legendary), the pool naming its set so that it reaches them while the set is held
//! (R1420); the pick is fused into this with its Activate (R1751000), wearing the face its host wears
//! (R77). The Activate, once a turn (R384), summons S+ #60 Radi (Radiant on the Radiant face) into its
//! controller's leftmost open zone (R64, R688) and triggers the Radi's Cry as if it was played, as
//! Tylannt's and League of Losers' summons do (R411): the Radi's own Discover is asked, and the Weapon
//! it picks is equipped into the Radi. R3537001: the Activate is offered while the Radi has a zone
//! to enter (R430); a Radi is no Flying Unit, so a full unit row has none (R2248000).

use jackioh_engine::effects::equip_picks;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-160";

/// S+ #60 Radi.
const RADI: &str = "stinkyscapeplus-060";

/// "Tribute 3" (R101).
const TRIBUTE: i32 = 3;

/// "Activate 1": one use a turn (R384, R1857003).
const USES: i32 = 1;

/// "Discover a … Weapon": one pick.
const PICKS: i32 = 1;

/// R411: the Radi this Activate summoned, if it is still there, has its Cry triggered.
fn radi_cry() -> Effect {
    for_each_card(ForEachCardArgs {
        cards: Arc::new(|ctx: &mut EffectContext<'_>| {
            summoned_so_far(ctx)
                .into_iter()
                .filter(|id| find_instance(&*ctx.state, id).is_some_and(|card| card.def_id == RADI))
                .collect()
        }),
        each: Arc::new(|instance_id: &str| trigger_cry(json_as(json!({ "instanceId": instance_id })))),
    })
}

/// "Summon a [Radiant] Radi. It triggers its Cry."
fn summon_radi(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: "summonRadi".to_string(),
        label: if radiant {
            "Summon a Radiant Radi".to_string()
        } else {
            "Summon a Radi".to_string()
        },
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        // R430, R64, R688: a Radi has a zone to enter, or the Activate is not offered.
        can_activate: Some(condition_hook(|c| first_entry_zone(c.state, c.controller, Row::Units).is_some())),
        has: None,
        run: hook(move |_ctx| vec![summon(json_as(json!({ "defId": RADI, "radiant": radiant }))), radi_cry()]),
    }
}

/// R1862002: "Discover a `rarity` Weapon; Equip it into this".
fn arm(radiant: bool, rarity: Rarity) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
            ..StaticFlags::default()
        }),
        activations: vec![summon_radi(radiant)],
        cry: Some(hook(move |_ctx| {
            vec![equip_picks(json_as(json!({
                "query": { "set": SetName::StinkyscapePlus, "tags": [Tag::Weapon], "rarity": rarity },
                "times": PICKS,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: arm(false, Rarity::Epic),
        radiant: arm(true, Rarity::Legendary),
    }
}

// S+ #160 Icarus — SPEC §8.12 row 160 (R3537001; R101, R411, R1841002, R1862002, R1751000, R384).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{next_turn_of, weapon_board};
    use jackioh_engine::effects::EQUIP_PICK_STEP;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    /// The S+ Rare, Epic and Legendary Weapons (§8.12): each rarity's pool is these three.
    const RARE: [&str; 3] = ["stinkyscapeplus-008", "stinkyscapeplus-143", "stinkyscapeplus-146"];
    const EPIC: [&str; 3] = ["stinkyscapeplus-009", "stinkyscapeplus-144", "stinkyscapeplus-147"];
    const LEGENDARY: [&str; 3] = ["stinkyscapeplus-010", "stinkyscapeplus-145", "stinkyscapeplus-148"];

    fn summon_it() -> Value {
        json!({ "ability": "summonRadi" })
    }

    /// The open Discover's offers, sorted, after checking it is the equip-pick of `player`.
    fn offers(s: &Scenario) -> Vec<String> {
        let pending = s.state().pending.clone().expect("a Discover is open");
        assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
        assert_eq!(pending.resume.step, EQUIP_PICK_STEP);
        let mut offered: Vec<String> = pending
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Mode { option } => Some(option.clone()),
                _ => None,
            })
            .collect();
        offered.sort();
        offered
    }

    /// Answers the open Discover with its first offer: `host` gains that Weapon's health on its face.
    fn pick_for(s: &mut Scenario, host: &str, pool: &[&str]) {
        let offered = offers(s);
        assert_eq!(offered, pool, "the whole pool is offered");
        let before = s.stats(host).health;
        s.answer(json!(offered[0]));
        let def = crate::card_def(&offered[0]);
        let gained = if s.card(host).radiant { def.radiant.health } else { def.base.health }.unwrap_or_default();
        assert_eq!(s.stats(host).health, before + gained, "{}", offered[0]);
    }

    fn icarus_on(radiant: bool) -> Scenario {
        weapon_board(
            "s160",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": [VANILLA] }),
        )
    }

    fn radis(s: &Scenario) -> Vec<CardInstance> {
        (1..=UNIT_ZONES)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|unit| unit.def_id == RADI)
            .collect()
    }

    fn played(radiant: bool, seed: &str) -> (Scenario, String) {
        let mut s = weapon_board(
            seed,
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": [VANILLA, VANILLA, VANILLA], "mana": 10 }),
            json!({ "field": [VANILLA] }),
        );
        let tributes = [1, 2, 3].map(|lane| s.unit_at(P1, lane).id);
        s.play(ID, json!({ "zone": 4, "tributes": tributes }));
        (s, ID.to_string())
    }

    mod base {
        use super::*;

        #[test]
        fn r1862002_the_cry_discovers_one_epic_weapon_and_equips_it() {
            let (mut s, _) = played(false, "s160-cry");
            let host = s.unit_at(P1, 4).id;
            pick_for(&mut s, host.as_str(), &EPIC);
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn r101_it_needs_tribute_3_exactly() {
            let mut s = weapon_board(
                "s160-tribute",
                json!({ "hand": [ID], "field": [VANILLA, VANILLA, VANILLA, VANILLA], "mana": 10 }),
                json!({}),
            );
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE);
            let lanes = [1, 2, 3, 4].map(|lane| s.unit_at(P1, lane).id);
            s.expect_refused(|s| s.play(ID, json!({ "zone": 5 })));
            s.expect_refused(|s| s.play(ID, json!({ "zone": 5, "tributes": [lanes[0].as_str(), lanes[1].as_str()] })));
            s.expect_refused(|s| s.play(ID, json!({ "zone": 5, "tributes": lanes })));
        }

        #[test]
        fn r411_the_activate_summons_a_radi_and_triggers_its_cry_which_equips_a_rare_weapon_into_it() {
            let mut s = icarus_on(false);
            assert_eq!(activate_actions_for(s.state(), P1, s.card(ID)).len(), 1);
            s.activate(ID, summon_it());
            let [radi] = radis(&s).try_into().expect("one Radi");
            assert!(!radi.radiant);
            assert_eq!(s.unit_at(P1, 2).id, radi.id, "the leftmost open zone (R64)");
            // The Radi's Cry was triggered as if it was played: its Discover of Rare Weapons (R411).
            let before = s.stats(radi.id.as_str()).health;
            assert_eq!(offers(&s), RARE);
            s.answer(json!(RARE[0]));
            let def = crate::card_def(RARE[0]);
            assert_eq!(s.stats(radi.id.as_str()).health, before + def.base.health.unwrap_or_default());
            assert!(s.state().pending.is_none());
            // R384: one use a turn; it is back next turn.
            assert!(activate_actions_for(s.state(), P1, s.card(ID)).is_empty());
            s.expect_refused(|s| s.activate(ID, summon_it()));
            next_turn_of(&mut s, P1);
            assert_eq!(activate_actions_for(s.state(), P1, s.card(ID)).len(), 1);
        }

        #[test]
        fn r3537001_with_no_unit_zone_the_activate_is_not_offered() {
            let mut s = weapon_board(
                "s160-full",
                json!({ "field": [ID, VANILLA, VANILLA, VANILLA, VANILLA] }),
                json!({}),
            );
            assert!(activate_actions_for(s.state(), P1, s.card(ID)).is_empty());
            s.expect_refused(|s| s.activate(ID, summon_it()));
            assert!(radis(&s).is_empty());
        }

        #[test]
        fn r3537001_the_radi_is_summoned_not_played_so_nothing_is_spent_but_the_use() {
            let mut s = icarus_on(false);
            let mana = s.state().players.p1.mana.current;
            s.activate(ID, summon_it());
            s.answer(json!(RARE[0]));
            assert_eq!(s.state().players.p1.mana.current, mana);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862002_the_cry_discovers_one_legendary_weapon_and_equips_it() {
            let (mut s, _) = played(true, "s160-radiant-cry");
            let host = s.unit_at(P1, 4).id;
            assert!(s.card(host.as_str()).radiant);
            pick_for(&mut s, host.as_str(), &LEGENDARY);
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn r411_the_activate_summons_a_radiant_radi_whose_cry_discovers_an_epic_weapon() {
            let mut s = icarus_on(true);
            s.activate(ID, summon_it());
            let [radi] = radis(&s).try_into().expect("one Radi");
            assert!(radi.radiant);
            assert_eq!(s.stats(radi.id.as_str()).attack, 2);
            let before = s.stats(radi.id.as_str()).health;
            assert_eq!(offers(&s), EPIC);
            s.answer(json!(EPIC[0]));
            let def = crate::card_def(EPIC[0]);
            assert_eq!(s.stats(radi.id.as_str()).health, before + def.radiant.health.unwrap_or_default());
            assert!(s.state().pending.is_none());
        }
    }
}
