//! S T-dormant-starship Dormant Starship (SPEC §8.10 row T-dormant-starship; R349, R1839000, R1852000,
//! R1852003–R1852007, R1857001, R1859013, R2521000): (3) Unit, Legendary, 0/0, the set's one entry that
//! is neither a numbered card nor a token, of the Stinkyscape set.
//!
//! Base and Radiant: "Dormant until it launches. Your Starship Pieces that die fuse into this.
//! \nActivate: Pay (3) to launch this." (R2249000: the rest is the Starship and Launch terms'.)
//! Engine: its text is the engine's rules written out (R1859013), so its script is the hull alone:
//! `StaticFlags { slumber, starshipHull }` on both faces, Dormant with no countdown (R1839000), its
//! stats only what fuses into it (R77). The engine summons it when a piece dies (R1852000) and fuses
//! into it the dead piece's base version (R2521000): the piece it was printed as, never a card fused
//! into it, on its Radiant face if it was Radiant, with its printed numbers and none of its Buffs, Nerfs
//! or Upgrades. The engine exiles a Stack card played onto it (R1852007) and gives it its one ability,
//! the launch for `STARSHIP_LAUNCH_COST` (`starship::launch_decl`, R1852003, moved by R1852006): a name
//! from `STARSHIP_NAMES` (R1852004), its fused Cries, then its fused When launched. No token, so it
//! reaches a graveyard; in no pool and no deck (R1857001); its Radiant face is R349's fallback
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
// T-dormant-starship: a Starship Piece's death summons it 0/0 into its owner's leftmost open unit zone, else
// on top of lane 3, and the piece's copy fuses in at once (R1852000), the dead piece's base version (#2521,
// R2521000); it is Dormant, unattackable and unreachable (R1839000); a Stack card played onto it is exiled
// before it resolves (R1852007); its Activate for (3) launches it: the Starship keyword, a name (R1852004),
// its fused Cries, then When launched (R1852003); "Your Starship launch costs (0)" prices it (R1852006); no
// pool and no deck holds it (R1857001); radiant R349's fallback, 0/0.
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
    // Issue #2521: Stratos (S #68) is no Starship Piece (B9), so a keywords-only piece stands in.
    const AP_ROUNDS: &str = "stinkyscape-100"; // (1) 3/1 Starship Piece, Equip, Pierce: keywords only.
    const FLAK_V3: &str = "stinkyscape-053"; // (2) 0/4 Starship Piece, Equip; Radiant 0/8, Lucky 1.
    const OVERDRIVE: &str = "stinkyscape-101"; // (2) 5/5 Starship Piece, Equip, Brittle 2, Reborn.
    const CAPSULE: &str = "stinkyscape-040"; // (3) 2/2 Starship Piece, "Cry: Place 2 Plague Counters."
    const CAPACITOR: &str = "stinkyscape-133"; // When launched: +1/+1 per other piece.
    const MONKEY: &str = "classic-077"; // (2) 5/5 Stack, "Aura: Spells cost (1) more."
    const HIT_JOB: &str = "core-016"; // (3) Spell: "Destroy a Unit."
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";
    const TRAP: &str = "core-085"; // (2) Trap, set face down to fill a backrow.
    /// R2521000: what a test's Buff (an Upgrade's tuning, B3.4) adds to a piece's attack and health.
    const BUFF: i32 = 2;

    /// Every zone of a backrow taken by a face-down Trap, so a Starship finds none open (R2249000).
    fn full_backrow() -> Value {
        json!(vec![json!({ "def": TRAP, "faceUp": false }); BACKROW_ZONES as usize])
    }

    fn ship_on_view(s: &Scenario) -> jackioh_engine::wire::UnitView {
        let lane = STARSHIP_LANE_ORDER[0] as usize - 1;
        match s.view(P1).you.backrow[lane].as_ref() {
            Some(BackrowView::Public(card)) => card.unit.clone(),
            _ => None,
        }
        .expect("the middle of the backrow shows the Starship")
    }

    /// p1's Dormant Starship holding one AP Rounds, standing in the middle of the backrow (R2249000), a
    /// Vanilla in unit lane 1.
    fn one_piece(seed: &str, p1: Value, p2: Value) -> (Scenario, String) {
        let mut p1 = p1;
        let mut field = vec![json!({ "def": VANILLA, "lane": 1 })];
        field.extend(p1["field"].as_array().cloned().unwrap_or_default());
        p1["field"] = json!(field);
        built_starship(seed, json!([{ "def": AP_ROUNDS, "lane": 2 }]), p1, p2)
    }

    fn launched_name(s: &Scenario, ship: &str) -> Option<String> {
        s.card(ship)
            .starship
            .as_ref()
            .and_then(|memory| memory.name.clone())
    }

    /// R1852005: one piece as a Starship's memory holds it.
    fn record(def_id: &str, radiant: bool) -> StarshipPieceRecord {
        StarshipPieceRecord {
            def_id: def_id.to_string(),
            radiant: if radiant { Some(true) } else { None },
        }
    }

    /// The definitions a fused definition lists, in ingredient order (`CardDef.ingredients`).
    fn ingredient_ids(s: &Scenario, def_id: &str) -> Vec<String> {
        jackioh_engine::catalog::find_def(Some(s.state()), def_id)
            .and_then(|def| def.ingredients.clone())
            .unwrap_or_default()
            .into_iter()
            .map(|spec| spec.def_id)
            .collect()
    }

    /// p1's main phase with a Hit Job and `hand` in hand and `field` on the field, mana to spare.
    fn hit_job_board(seed: &str, hand: Value, field: Value) -> Scenario {
        let mut cards = vec![json!(HIT_JOB)];
        cards.extend(hand.as_array().cloned().unwrap_or_default());
        crate::scenario(json!({
            "seed": seed,
            "active": "p1",
            "p1": { "hand": cards, "field": field, "library": crate::filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    fn the_ship(s: &Scenario) -> CardInstance {
        dormant_starship_of(s.state(), P1)
            .expect("p1's Dormant Starship")
            .clone()
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
            json!([{ "def": AP_ROUNDS, "lane": 2 }]),
            json!({ "field": [{ "def": AP_ROUNDS, "lane": 1 }] }),
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
        assert_eq!((stats.attack, stats.health), (3, 1), "only what fused into it");
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
                "field": [VANILLA, VANILLA, VANILLA, VANILLA, OVERDRIVE],
                "backrow": full_backrow(),
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        // p1's Overdrive Module dies, and Reborn returns it to its zone before its copy fuses (§4.5
        // step 4), so p1's rows are full again when the Starship is summoned. A piece dies on its
        // controller's side and builds its controller's ship (R2618000), so only a body that comes
        // back leaves that side full.
        let piece = s.unit_at(P1, 5).id;
        let beneath = s.unit_at(P1, STARSHIP_FULL_ROW_LANE).id;
        s.play(HIT_JOB, json!({ "targets": pick::id(&piece) }));
        assert_eq!(s.unit_at(P1, 5).id, piece, "Reborn brought it back first");
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
            (5, 5),
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
        s.state_mut().players.p1.mana.current = 10;
        s.expect_refused(|s| s.play(HIT_JOB, json!({ "targets": pick::id(&ship) })));
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
        // An AP Rounds holds no tag, so the name is one that suits any ship.
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

    // B10–B12 (issue #2521): a dead piece fuses its base version.

    #[test]
    fn r2521000_a_flak_v3_with_a_card_fused_into_it_fuses_flak_v3_alone() {
        let mut s = hit_job_board(
            "r2521000-flak",
            json!([AP_ROUNDS]),
            json!([{ "def": FLAK_V3, "lane": 1 }]),
        );
        let host = s.unit_at(P1, 1).id;
        s.play(AP_ROUNDS, json!({ "zone": 1, "equip": true }));
        assert_eq!(s.unit_at(P1, 1).id, host, "Flak v3 is the card the Equip keeps");
        let fused = s.card(host.as_str()).def_id.clone();
        assert_eq!(
            ingredient_ids(&s, &fused),
            vec![AP_ROUNDS.to_string(), FLAK_V3.to_string()]
        );
        s.play(HIT_JOB, json!({ "targets": pick::id(&host) }));
        s.expect_in_zone(host.as_str(), "graveyard");
        let ship = the_ship(&s);
        // The memory and the event name Flak v3 alone, not the fusion.
        assert_eq!(starship_pieces_of(&ship), vec![record(FLAK_V3, false)]);
        assert!(s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::StarshipFused { piece_id, def_id, .. } if *piece_id == host && def_id == FLAK_V3
        )));
        // The Starship gains only Flak v3's text: its 0/4, none of the AP Rounds' 3/1 or Pierce.
        let stats = s.stats(ship.id.as_str());
        assert_eq!((stats.attack, stats.health), (0, 4));
        assert!(
            !stats
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Pierce)
        );
        let held = ingredient_ids(&s, &ship.def_id);
        assert!(held.contains(&FLAK_V3.to_string()), "{held:?}");
        assert!(!held.contains(&AP_ROUNDS.to_string()), "{held:?}");
    }

    #[test]
    fn r2521000_a_buffed_piece_fuses_with_its_printed_numbers() {
        for radiant in [false, true] {
            let mut s = hit_job_board(
                "r2521000-buffed",
                json!([]),
                json!([{ "def": AP_ROUNDS, "lane": 1, "radiant": radiant }]),
            );
            let piece = s.unit_at(P1, 1).id;
            s.card_mut(piece.as_str()).tuning = Some(Tuning {
                attack: Some(BUFF),
                health: Some(BUFF),
                ..Tuning::default()
            });
            let printed = if radiant { (6, 2) } else { (3, 1) };
            let buffed = s.stats(piece.as_str());
            assert_eq!(
                (buffed.attack, buffed.health),
                (printed.0 + BUFF, printed.1 + BUFF),
                "radiant {radiant}"
            );
            s.play(HIT_JOB, json!({ "targets": pick::id(&piece) }));
            let ship = the_ship(&s);
            assert_eq!(
                starship_pieces_of(&ship),
                vec![record(AP_ROUNDS, radiant)],
                "radiant {radiant}"
            );
            // The Buff was the dead card's: the Starship holds the printed numbers and no tuning.
            let stats = s.stats(ship.id.as_str());
            assert_eq!((stats.attack, stats.health), printed, "radiant {radiant}");
            assert_eq!(ship.tuning, None, "radiant {radiant}");
        }
    }

    #[test]
    fn r2521000_a_radiant_piece_fuses_on_its_radiant_face() {
        for radiant in [false, true] {
            let (s, ship) = built_starship(
                "r2521000-radiant",
                json!([
                    { "def": AP_ROUNDS, "lane": 1, "radiant": radiant },
                    { "def": FLAK_V3, "lane": 2, "radiant": radiant },
                ]),
                json!({}),
                json!({}),
            );
            assert_eq!(
                starship_pieces_of(s.card(ship.as_str())),
                vec![record(AP_ROUNDS, radiant), record(FLAK_V3, radiant)],
                "radiant {radiant}"
            );
            // Base 3/1 and 0/4; Radiant 6/2 and 0/8, with the Radiant Flak v3's Lucky 1.
            let stats = s.stats(ship.as_str());
            let expected = if radiant { (6, 10) } else { (3, 5) };
            assert_eq!((stats.attack, stats.health), expected, "radiant {radiant}");
            let lucky = stats
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Lucky);
            assert_eq!(lucky, radiant, "radiant {radiant}");
        }
    }
}
