//! S #106 Warpfield Amplifier (SPEC §8.10 row 106; R20, R91, R1852000, R1852003, R1859001, R1859002):
//! (1) Unit, Rare, 2/3 → 4/6, Starship Piece, a card of the Stinkyscape set.
//!
//! Base and Radiant: "Starship Piece\nWhen launched, switch each of your Units to Attack Position and
//! give it +{attackGain} Attack." AttackGain 2, Radiant 4.
//! Engine: its death fuses a copy into its owner's Dormant Starship (R1852000); When launched
//! (`when_launched`, R1852003) runs only as part of that Starship. Then, for each Unit its controller

use std::sync::Arc;

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-106";

fn warpfield(radiant: bool) -> Script {
    Script {
        triggers: vec![when_launched("warpfield-amplifier", move |ctx, _event| {
            let gain = piece_number(ctx, ID, radiant, "attackGain");
            vec![
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx: &mut EffectContext<'_>| -> Vec<String> {
                        active_units_of(&*ctx.state, ctx.controller)
                            .iter()
                            .map(|card| card.id.clone())
                            .collect()
                    }),
                    each: Arc::new(|instance_id: &str| {
                        switch_position_of(json_as(json!({
                            "to": "ATK",
                            "target": { "of": "instance", "instanceId": instance_id },
                        })))
                    }),
                }),
                buff_all_units(json_as(json!({ "attack": gain }))),
            ]
        })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: warpfield(false),
        radiant: warpfield(true),
    }
}

// S #106 Warpfield Amplifier — SPEC §8.10 row 106, BUILD M12 row S 106 (R1859001, R20).
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{built_starship, launch_starship};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const AP_ROUNDS: &str = "stinkyscape-100"; // (1) 3/1 Starship Piece, Equip, Pierce: keywords only.

    fn launched(seed: &str, radiant: bool) -> (Scenario, String) {
        let (mut s, ship) = built_starship(
            seed,
            json!([{ "def": ID, "lane": 1, "radiant": radiant }]),
            json!({ "field": [
                { "def": VANILLA, "lane": 2, "position": "DEF" },
                { "def": VANILLA, "lane": 3 },
            ] }),
            json!({ "field": [{ "def": VANILLA, "lane": 1, "position": "DEF" }] }),
        );
        assert_eq!(s.stats(s.unit_at(P1, 2).id.as_str()).position, Position::Def);
        launch_starship(&mut s, &ship);
        (s, ship)
    }

    fn attack_of(s: &Scenario, player: PlayerId, lane: i32) -> i32 {
        s.stats(s.unit_at(player, lane).id.as_str()).attack
    }

    #[test]
    fn r1859002_each_of_its_controllers_units_goes_to_attack_and_gains_2_attack_the_starship_among_them() {
        let (s, ship) = launched("r1859002-base", false);
        let defender = s.unit_at(P1, 2).id;
        assert_eq!(s.stats(defender.as_str()).position, Position::Atk);
        assert!(!s.card(defender.as_str()).exertion.switched, "an effect's switch spends nothing (R20)");
        assert_eq!(attack_of(&s, P1, 2), 4 + 2);
        assert_eq!(attack_of(&s, P1, 3), 4 + 2, "a Unit already in Attack Position gains it too");
        assert_eq!(s.stats(ship.as_str()).attack, 2 + 2);
        let theirs = s.unit_at(P2, 1).id;
        assert_eq!(s.stats(theirs.as_str()).position, Position::Def);
        assert_eq!(s.stats(theirs.as_str()).attack, 4);
    }

    #[test]
    fn r1859001_a_radiant_piece_gives_4_attack_read_on_the_face_it_fused_on() {
        let (s, ship) = launched("r1859002-radiant", true);
        assert_eq!(attack_of(&s, P1, 2), 4 + 4);
        assert_eq!(s.stats(ship.as_str()).attack, 4 + 4);
    }

    #[test]
    fn r1859002_the_gain_lasts_past_the_turn() {
        let (mut s, _ship) = launched("r1859002-lasts", false);
        s.end_turn();
        s.end_turn();
        assert_eq!(attack_of(&s, P1, 3), 4 + 2);
    }

    #[test]
    fn r1859002_on_the_field_it_does_nothing_when_a_starship_launches() {
        let (mut s, ship) = built_starship(
            "r1859002-field",
            json!([{ "def": AP_ROUNDS, "lane": 1 }]),
            json!({ "field": [{ "def": ID, "lane": 2, "position": "DEF" }] }),
            json!({}),
        );
        launch_starship(&mut s, &ship);
        assert_eq!(s.stats(s.unit_at(P1, 2).id.as_str()).position, Position::Def);
        assert_eq!(attack_of(&s, P1, 2), 2);
    }
}
