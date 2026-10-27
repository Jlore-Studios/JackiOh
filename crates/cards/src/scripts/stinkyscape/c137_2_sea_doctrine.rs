//! S #137.2 Sea Doctrine (SPEC §8.10 row 137.2; R49, R1852005, R1859007, R1859009): (4) Spell, War
//! Doctrine, Token (printed Legendary), a token of the held Stinkyscape set (R1420), which #137
//! Firststrike Capability Discovers and casts.
//!
//! Base and Radiant: "For the rest of the game, your Units have Deft. Heal your hero {multiplier}× your
//! Starship's Health." Multiplier 2, Radiant 5.
//! Engine: `give_units_keyword { Deft }`, a `unitsHaveKeyword` player modifier for the rest of the game:
//! every Unit its caster controls has Deft (an attack and a switch in one turn, R49), those that arrive
//! later too, but a Vanilla one (R1859007). Then, once, as it is cast: `heal` on its caster's hero by
//! multiplier × the Health of their launched Starship (`launched_starship_of`, R1852005) as it stands,
//! damage counted, with no cap on a hero (§6.3 Heal, R1859009); no launched Starship, no heal.
//! Tunes: multiplier 2 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-137-2";

fn sea_doctrine() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let mut effects = vec![give_units_keyword(GiveUnitsKeywordArgs { keyword: Keyword::Deft })];
            let Some(ship) = launched_starship_of(&*ctx.state, ctx.controller) else {
                return effects;
            };
            let amount = param(&*ctx, "multiplier") * unit_view(&*ctx.state, ship).health;
            if amount > 0 {
                effects.push(heal(json_as(json!({
                    "target": { "of": "selfHero" },
                    "amount": amount,
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face's five times is its declared multiplier, which `param` reads off the face.
    let base = sea_doctrine();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #137.2 Sea Doctrine — SPEC §8.10 row 137.2, BUILD M12 row S 137.2: "after it, every Unit its caster
// controls has Deft (an attack and a switch in one turn, R49), later ones included; once, their hero heals
// twice their launched Starship's current Health, past its starting health; no Starship, no heal;
// multiplier reads through `param()`; radiant five times".
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{built_starship, launch_starship};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const STARTOS: &str = "stinkyscape-068"; // (1) 3/3 Starship Piece, Flying: keywords only.
    const VANILLA: &str = "core-008"; // (1) 4/4.

    fn has_deft(s: &Scenario, id: &str) -> bool {
        s.stats(id)
            .keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Deft)
    }

    /// p1's Starship (two Startos, 6/6), launched if `launch`, with 2 damage on it; p1's hero at 20, a
    /// Mr. Vanilla of p1's in lane 3 and the Doctrine (Radiant if `radiant`) in hand. The hero's health
    /// after the Doctrine.
    fn healed(seed: &str, radiant: bool, launch: bool) -> (Scenario, i32) {
        let (mut s, ship) = built_starship(
            seed,
            json!([{ "def": STARTOS, "lane": 1 }, { "def": STARTOS, "lane": 2 }]),
            json!({
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [{ "def": VANILLA, "lane": 3 }],
                "health": 20,
            }),
            json!({}),
        );
        if launch {
            launch_starship(&mut s, &ship);
            s.card_mut(ship.as_str()).damage = 2;
        }
        s.state_mut().players.p1.mana.current = 10;
        s.play(ID, json!({}));
        let health = s.state().players.p1.hero.health;
        (s, health)
    }

    #[test]
    fn r1859009_it_heals_its_casters_hero_twice_the_starships_health_damage_counted() {
        let (s, health) = healed("r1859009-sea", false, true);
        assert_eq!(health, 20 + 2 * (6 - 2));
        assert!(has_deft(&s, s.unit_at(P1, 3).id.as_str()));
    }

    #[test]
    fn r1859009_radiant_five_times_past_the_starting_health() {
        let (_s, health) = healed("r1859009-sea-radiant", true, true);
        assert_eq!(health, 20 + 5 * (6 - 2));
    }

    #[test]
    fn r1859007_with_no_launched_starship_only_the_keyword() {
        let (mut s, health) = healed("r1859007-sea-none", false, false);
        assert_eq!(health, 20);
        assert!(has_deft(&s, s.unit_at(P1, 3).id.as_str()));
        s.end_turn();
        s.end_turn();
        assert!(has_deft(&s, s.unit_at(P1, 3).id.as_str()), "for the rest of the game");
    }
}
