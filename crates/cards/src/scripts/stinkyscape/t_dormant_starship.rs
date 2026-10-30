//! S T-dormant-starship Dormant Starship (SPEC §8.10 row T-dormant-starship; R349, R1839000, R1852000,
//! R1852003–R1852007, R1857001, R1859013, R2521000): (3) Unit, Legendary, 0/0, the set's one entry that
//! is neither a numbered card nor a token, of the Stinkyscape set.
//!
//! Base and Radiant: "Dormant until it launches. Your Starship Pieces that die fuse into this.
//! \nActivate: Pay (3) to launch this." (R2249000: the rest is the Starship and Launch terms'.)
//! Engine: its text is the engine's rules written out (R1859013), so its script is the hull alone:
//! `StaticFlags { slumber, starshipHull }` on both faces, Dormant with no countdown (R1839000), its
//! stats only what fuses into it (R77). A dead piece fuses its base version (R2521000): the piece it
//! was printed as, never a card fused into it, on its Radiant face if it was Radiant, with its printed
//! numbers and none of its Buffs, Nerfs or Upgrades. The engine summons it when a piece dies (R1852000), exiles a
//! Stack card played onto it (R1852007) and gives it its one ability, the launch for
//! `STARSHIP_LAUNCH_COST` (`starship::launch_decl`, R1852003, moved by R1852006): a name from
//! `STARSHIP_NAMES` (R1852004), its fused Cries, then its fused When launched. No token, so it reaches
//! a graveyard; in no pool and no deck (R1857001); its Radiant face is R349's fallback
//! (`radiantFallback`). Declares no numbers: the price is the engine's constant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-dormant-starship";

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText::default()),
            starship_hull: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// S T-dormant-starship Dormant Starship — SPEC §8.10 row T-dormant-starship, BUILD M12 row S
// T-dormant-starship: "a Starship Piece's death summons it 0/0 into its owner's leftmost open unit zone,
// else on top of lane 3, and the piece's copy fuses in at once (R1852000); it is Dormant, unattackable and
// unreachable (R1839000); a Stack card played onto it is exiled before it resolves (R1852007); its
// Activate for (3) launches it: the Starship keyword, a name (R1852004), its fused Cries, then When
// launched (R1852003); "Your Starship launch costs (0)" prices it (R1852006); no pool and no deck holds it
// (R1857001); radiant R349's fallback, 0/0".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use crate::{built_starship, card_def, launch_starship};
    use jackioh_engine::prelude::{
        STARSHIP_FULL_ROW_LANE, STARSHIP_LANE_ORDER, STARSHIP_LAUNCH_ABILITY, STARSHIP_LAUNCH_COST,
        STARSHIP_NAMES, is_launched_starship,
    };
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const STRATOS: &str = "stinkyscape-068"; // (1) 3/3 Starship Piece, Flying: keywords only.
    const CAPSULE: &str = "stinkyscape-040"; // (3) 2/2 Starship Piece, "Cry: Place 2 Plague Counters."
    const CAPACITOR: &str = "stinkyscape-133"; // When launched: +1/+1 per other piece.
    const MONKEY: &str = "classic-077"; // (2) 5/5 Stack, "Aura: Spells cost (1) more."
    const HIT_JOB: &str = "core-016"; // (3) Spell: "Destroy a Unit."
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";
    const TRAP: &str = "core-085"; // (2) Trap, set face down to fill a backrow.

    /// Every zone of a backrow taken by a face-down Trap, so a Starship finds none open (R2249000).
    fn full_backrow() -> Value {
        json!(vec![json!({ "def": TRAP, "faceUp": false }); BACKROW_ZONES as usize])
    }

    /// R2249000: the Starship standing in the middle of p1's backrow, as p1's view shows it.
    fn ship_on_view(s: &Scenario) -> jackioh_engine::wire::UnitView {
        let lane = STARSHIP_LANE_ORDER[0] as usize - 1;
        match s.view(P1).you.backrow[lane].as_ref() {
            Some(BackrowView::Public(card)) => card.unit.clone(),
            _ => None,
        }
        .expect("the middle of the backrow shows the Starship")
    }

    /// p1's Dormant Starship holding one Stratos, standing in the middle of the backrow (R2249000), a
    /// Vanilla in unit lane 1.
    fn one_piece(seed: &str, p1: Value, p2: Value) -> (Scenario, String) {
        let mut p1 = p1;
        let mut field = vec![json!({ "def": VANILLA, "lane": 1 })];
        field.extend(p1["field"].as_array().cloned().unwrap_or_default());
        p1["field"] = json!(field);
        built_starship(seed, json!([{ "def": STRATOS, "lane": 2 }]), p1, p2)
    }

    fn launched_name(s: &Scenario, ship: &str) -> Option<String> {
        s.card(ship)
            .starship
            .as_ref()
            .and_then(|memory| memory.name.clone())
    }

    #[test]
    fn r1859013_its_script_is_the_hull_on_both_faces() {
        let scripts = script();
        for face in [&scripts.base, &scripts.radiant] {
            let flags = face.static_flags.as_ref().expect("the hull's flags");
            assert_eq!(flags.starship_hull, Some(true));
            assert!(flags.slumber.is_some());
            assert!(face.triggers.is_empty() && face.activations.is_empty() && face.cry.is_none());
        }
        let def = card_def(ID);
        assert!(!def.token);
        assert_eq!(def.rarity, Rarity::Legendary);
        assert_eq!(def.radiant_fallback, Some(true));
        assert_eq!((def.base.attack, def.base.health), (Some(0), Some(0)));
    }

    #[test]
    fn r1859013_a_dying_piece_summons_it_holding_the_copy_and_its_launch_is_offered_at_3() {
        let (mut s, ship) = built_starship(
            "r1859013-hull",
            json!([{ "def": STRATOS, "lane": 2 }]),
            json!({ "field": [{ "def": STRATOS, "lane": 1 }] }),
            json!({}),
        );
        let card = s.card(ship.as_str()).clone();
        assert_eq!(
            card.zone,
            Zone::Field {
                player: P1,
                row: Row::Backrow,
                lane: STARSHIP_LANE_ORDER[0]
            }
        );
        let stats = s.stats(ship.as_str());
        assert_eq!((stats.attack, stats.health), (3, 3), "only what fused into it");
        let unit = ship_on_view(&s);
        let abilities = unit.activations.clone().expect("the launch");
        assert_eq!(abilities.len(), 1);
        assert_eq!(abilities[0].ability, STARSHIP_LAUNCH_ABILITY);
        assert_eq!(abilities[0].label, format!("Launch ({STARSHIP_LAUNCH_COST})"));
        let mana = s.state().players.p1.mana.current;
        launch_starship(&mut s, &ship);
        assert_eq!(s.state().players.p1.mana.current, mana - STARSHIP_LAUNCH_COST);
        assert!(is_launched_starship(s.card(ship.as_str())));
        assert!(
            s.stats(ship.as_str())
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Starship)
        );
    }

    #[test]
    fn r1852000_with_both_its_owners_rows_full_it_is_stacked_on_top_of_lane_3() {
        let mut s = crate::scenario(json!({
            "seed": "r1852000-full-row",
            "active": "p1",
            "p1": {
                "hand": [HIT_JOB, FILLER],
                "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA],
                "backrow": full_backrow(),
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [STRATOS], "library": crate::filler(4) },
        }));
        // p1's Stratos, stolen: its death fuses into its owner's Starship (R1852002), p1's rows full.
        let piece = s.unit_at(P2, 1).id;
        s.card_mut(piece.as_str()).owner = P1;
        let beneath = s.unit_at(P1, STARSHIP_FULL_ROW_LANE).id;
        s.play(HIT_JOB, json!({ "targets": pick::id(&piece) }));
        let ship = dormant_starship_of(s.state(), P1)
            .expect("p1's Dormant Starship")
            .id
            .clone();
        assert_eq!(s.unit_at(P1, STARSHIP_FULL_ROW_LANE).id, ship, "on top of lane 3");
        let slot = ZoneSlot {
            player: P1,
            row: Row::Units,
            lane: STARSHIP_FULL_ROW_LANE,
        };
        let pile = pile_at(s.state(), slot).expect("lane 3's pile").clone();
        assert_eq!(pile.len(), 2);
        assert_eq!(pile[1].id, beneath, "the Vanilla lies beneath it");
        let stats = s.stats(ship.as_str());
        assert_eq!(
            (stats.attack, stats.health),
            (3, 3),
            "the piece's copy fused in at once"
        );
    }

    #[test]
    fn r1839000_dormant_it_is_neither_attackable_nor_reachable() {
        let (mut s, ship) = one_piece(
            "r1839000-hull",
            json!({ "hand": [HIT_JOB] }),
            json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
        );
        assert!(is_dormant_starship(s.card(ship.as_str())));
        // Unreachable: a declared target never names it.
        s.state_mut().players.p1.mana.current = 10;
        s.expect_refused(|s| s.play(HIT_JOB, json!({ "targets": pick::id(&ship) })));
        // Unattackable: on p2's turn their Vanilla may not attack it.
        s.state_mut()
            .players
            .p1
            .hand
            .retain(|card| card.def_id != HIT_JOB);
        s.end_turn();
        assert_eq!(s.state().active, P2);
        let attacker = s.unit_at(P2, 1).id;
        assert!(!legal_actions(s.state(), P2).iter().any(|action| matches!(
            action,
            ActionBody::Attack { attacker_id, target_id, .. } if *attacker_id == attacker && *target_id == ship
        )));
        s.expect_refused(|s| s.attack(attacker.as_str(), ship.as_str()));
        assert!(is_dormant_starship(s.card(ship.as_str())));
    }

    #[test]
    fn r1852007_a_stack_card_played_onto_it_is_exiled_before_it_resolves() {
        // With the backrow full the Starship stands in the unit row's middle (R2249000), under the Stack.
        let (mut s, ship) = one_piece(
            "r1852007-hull",
            json!({ "hand": [MONKEY], "backrow": full_backrow() }),
            json!({}),
        );
        let lane = STARSHIP_LANE_ORDER[0];
        assert_eq!(s.unit_at(P1, lane).id, ship);
        s.play(MONKEY, json!({ "zone": lane }));
        assert!(
            s.last_events()
                .iter()
                .any(|event| event.event_type() == GameEventType::CardPlayed)
        );
        let monkey = s.pile(P1, "exile").into_iter().find(|card| card.def_id == MONKEY);
        assert!(monkey.is_some(), "the Monkey is in exile");
        assert!(
            s.unit(P1, lane).is_some_and(|unit| unit.id == ship),
            "the Starship tops its zone again"
        );
        assert!(is_dormant_starship(s.card(ship.as_str())), "still Dormant");
    }

    #[test]
    fn r1852004_the_launch_gives_it_a_name() {
        let (mut s, ship) = one_piece("r1852004-hull", json!({}), json!({}));
        assert_eq!(launched_name(&s, &ship), None);
        launch_starship(&mut s, &ship);
        let name = launched_name(&s, &ship).expect("a name");
        // A Stratos holds no tag, so the name is one that suits any ship.
        let untagged: Vec<&str> = STARSHIP_NAMES
            .iter()
            .filter(|(_, tag)| tag.is_none())
            .map(|(name, _)| *name)
            .collect();
        assert!(untagged.contains(&name.as_str()), "{name}");
        assert!(s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::StarshipLaunched { name: launched, .. } if *launched == name
        )));
    }

    #[test]
    fn r1852003_the_launch_fires_its_fused_cries_then_its_when_launched() {
        let (mut s, ship) = built_starship(
            "r1852003-hull",
            json!([{ "def": CAPSULE, "lane": 1 }, { "def": CAPACITOR, "lane": 2 }]),
            json!({}),
            json!({}),
        );
        let before = s.stats(ship.as_str()).max_health;
        launch_starship(&mut s, &ship);
        assert!(is_launched_starship(s.card(ship.as_str())));
        // The Capsule's fused Cry asks where its counters go; the When launched waits for it.
        let pending = s.state().pending.clone().expect("the fused Cry's prompt");
        assert_eq!(pending.player_id, P1);
        assert_eq!(s.stats(ship.as_str()).max_health, before, "no When launched yet");
        s.answer(json!(ship));
        assert_eq!(s.card(ship.as_str()).counters.plague, Some(2), "the fused Cry");
        assert_eq!(
            s.stats(ship.as_str()).max_health,
            before + 1,
            "then the Capacitor's When launched"
        );
        assert!(s.state().pending.is_none());
    }

    #[test]
    fn r1852006_your_starship_launch_costs_0_prices_its_launch() {
        let (mut s, ship) = one_piece("r1852006-hull", json!({}), json!({}));
        s.state_mut().players.p1.mods.push(PlayerModifier {
            id: "liftoff".to_string(),
            expiry: ModifierExpiry::Used,
            kind: ModifierKind::StarshipLaunchCost { cost: 0 },
        });
        let unit = ship_on_view(&s);
        assert_eq!(
            unit.activations.as_ref().expect("the launch")[0].label,
            "Launch (0)"
        );
        let mana = s.state().players.p1.mana.current;
        launch_starship(&mut s, &ship);
        assert_eq!(s.state().players.p1.mana.current, mana, "the launch cost nothing");
        assert_eq!(
            launch_cost(s.state(), P1),
            STARSHIP_LAUNCH_COST,
            "and spent the price"
        );
    }

    #[test]
    fn r1857001_no_pool_and_no_deck_holds_it() {
        crate::register_all();
        for args in [
            json!({}),
            json!({ "type": "Unit" }),
            json!({ "set": "Stinkyscape", "type": "Unit" }),
            json!({ "cost": 3 }),
            json!({ "withTokens": true }),
            json!({ "everySet": true }),
        ] {
            let pool: Vec<String> = query(&json_as(args.clone()))
                .iter()
                .map(|def| def.id.clone())
                .collect();
            assert!(!pool.contains(&ID.to_string()), "{args}");
        }
    }
}
