//! S #137.3 Air Doctrine (SPEC §8.10 row 137.3; R59, R60, R1748001, R1852005, R1859007, R1859010):
//! (4) Spell, War Doctrine, Token (printed Legendary), `dealsDamage`, a token of the Stinkyscape
//! set, which #137 Starfleet Commander Discovers and casts.
//!
//! Base and Radiant: "For the rest of the game, your Units have Rush. Deal damage equal to
//! {multiplier}× your Starship's Attack, split randomly among all enemies." Multiplier 1, Radiant 2.
//! Engine: `give_units_keyword { Rush }`, a `unitsHaveKeyword` player modifier for the rest of the
//! game: every Unit its caster controls has Rush, those that arrive later too, but a Vanilla one
//! (R1859007). Then, once, as it is cast: §6.3 Damage's split (`damage_split`), N hits of 1, N =
//! multiplier × the Attack of its caster's launched Starship (`launched_starship_of`, R1852005) as it
//! stands, each to a random enemy still standing, the hero or a Unit, by the match rng (R60), the
//! caster's Spell Damage raising each hit (R1859010); no launched Starship, nothing. Tunes: multiplier
//! 1 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-137-3";

fn air_doctrine() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let mut effects = vec![give_units_keyword(GiveUnitsKeywordArgs { keyword: Keyword::Rush })];
            let Some(ship) = launched_starship_of(&*ctx.state, ctx.controller) else {
                return effects;
            };
            let amount = param(&*ctx, "multiplier") * unit_view(&*ctx.state, ship).attack;
            if amount > 0 {
                effects.push(damage_split(json_as(json!({ "amount": amount }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face's twice is its declared multiplier, which `param` reads off the face.
    let base = air_doctrine();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #137.3 Air Doctrine — SPEC §8.10 row 137.3, BUILD M12 row S 137.3: "after it, every Unit its caster
// controls has Rush, later ones included; once, hits of 1 equal to their launched Starship's Attack go one
// by one to random enemies still standing, by rng picks that replay; no Starship, nothing; multiplier
// reads through `param()`; radiant twice its Attack".
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{built_starship, launch_starship};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const AP_ROUNDS: &str = "stinkyscape-100"; // (1) 3/1 Starship Piece, Equip, Pierce: keywords only.
    const VANILLA: &str = "core-008"; // (1) 4/4.

    /// The targets of the 1-damage hits among the last action's events.
    fn hits(s: &Scenario) -> Vec<String> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount: 1, .. } => Some(target_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// p1's Starship (two AP Rounds, 6/2), launched if `launch`; p2 with two Mr. Vanillas; the Doctrine
    /// (Radiant if `radiant`) in p1's hand, not yet cast. Returns the scenario and what an enemy hit may
    /// name: p2's hero and Units.
    fn board(seed: &str, radiant: bool, launch: bool) -> (Scenario, Vec<String>) {
        let (mut s, ship) = built_starship(
            seed,
            json!([{ "def": AP_ROUNDS, "lane": 1 }, { "def": AP_ROUNDS, "lane": 2 }]),
            json!({
                "hand": [{ "def": ID, "radiant": radiant }, VANILLA],
                "field": [{ "def": VANILLA, "lane": 3 }],
            }),
            json!({ "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }] }),
        );
        if launch {
            launch_starship(&mut s, &ship);
        }
        let enemies = vec![
            "hero-p2".to_string(),
            s.unit_at(PlayerId::P2, 1).id,
            s.unit_at(PlayerId::P2, 2).id,
        ];
        s.state_mut().players.p1.mana.current = 10;
        (s, enemies)
    }

    /// `board`, then the Doctrine cast.
    fn cast(seed: &str, radiant: bool, launch: bool) -> (Scenario, Vec<String>) {
        let (mut s, enemies) = board(seed, radiant, launch);
        s.play(ID, json!({}));
        (s, enemies)
    }

    #[test]
    fn r1859010_hits_of_1_equal_to_the_starships_attack_split_among_the_enemies() {
        let (s, enemies) = cast("r1859010-air", false, true);
        let hits = hits(&s);
        assert_eq!(hits.len(), 6);
        assert!(hits.iter().all(|target| enemies.contains(target)), "{hits:?}");
        let has_rush = s
            .stats(s.unit_at(P1, 3).id.as_str())
            .keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Rush);
        assert!(has_rush);
    }

    #[test]
    fn r1859010_the_split_replays_from_the_seed() {
        assert_eq!(hits(&cast("r1859010-air", false, true).0), hits(&cast("r1859010-air", false, true).0));
    }

    #[test]
    fn r1859010_radiant_twice_its_attack() {
        let (s, _) = cast("r1859010-air-radiant", true, true);
        assert_eq!(hits(&s).len(), 12);
    }

    #[test]
    fn multiplier_reads_through_param() {
        let (mut s, _) = board("air-param", false, true);
        let multiplier = crate::upgrade_number(&mut s, ID, "multiplier");
        s.play(ID, json!({}));
        assert_eq!(hits(&s).len(), usize::try_from(multiplier * 6).expect("a count"));
    }

    #[test]
    fn r1859007_with_no_launched_starship_only_the_keyword() {
        let (mut s, _) = cast("r1859007-air-none", false, false);
        assert!(hits(&s).is_empty());
        // A Unit played after it has Rush: it may attack at once.
        s.play(VANILLA, json!({ "zone": 4 }));
        let vanilla = s.unit_at(P1, 4).id;
        let has_rush = s
            .stats(vanilla.as_str())
            .keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Rush);
        assert!(has_rush);
    }
}
