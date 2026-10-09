//! Meditative #39.2 Jade (SPEC §8.8 row 39.2). (0) Spell, CN, Token (printed Rare).
//!
//!   Base:    "Add {jade} to your Jade Counter. Gain {mana} mana." — jade 1, mana 1
//!   Radiant: "Add {jade} to your Jade Counter. Gain {mana} mana." — jade 2, mana 2
//!
//! Engine: ME-JADE's `add_jade`, whose crossings of 5 and 10 run inside the same effect (MD-C3),
//! then `gain_mana` (§2.3 temporary mana; can go above 4).
//! Tunes: jade 1 ↑; mana 1 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-039-2";

fn jade() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![
                add_jade(json_as(json!({ "amount": param(&*ctx, "jade") }))),
                gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the Radiant face's 2s are its declared `jade` and `mana`, which `param`
    // reads off the running face.
    let base = jade();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative #39.2 Jade — SPEC §8.8 row 39.2, BUILD M10 row M 39.2: "Raises its caster's Jade
// Counter by 1 (`jadeChanged`, public on both views, never lowered; MD-C2) and gains 1 temporary
// mana; a crossing of 5 or 10 runs inside the same effect (MD-C3); jade and mana read through
// `param()`; radiant 2 and 2".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BEAUTY: &str = "meditative-039-5";
    const FILLER: &str = "core-008";

    #[test]
    fn r961_adds_1_and_gains_1_mana_on_both_views() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [ID, FILLER] }, "p2": { "hand": [FILLER] } }));
        let before = s.view(P1).you.mana.current;

        s.play(ID, json!({}));

        // MD-C2: the caster's own counter rises, public on both seats beside the hero.
        assert_eq!(s.view(P1).you.jade, Some(1));
        assert_eq!(s.view(P1).opponent.jade, None);
        assert_eq!(s.view(P2).you.jade, None);
        assert_eq!(s.view(P2).opponent.jade, Some(1));
        // §2.3: temporary mana, one above whatever the refresh left.
        assert_eq!(s.view(P1).you.mana.current, before + 1);
    }

    #[test]
    fn r962_the_fifth_jade_summons_a_beauty() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ID, ID, ID, ID, ID, FILLER] },
            "p2": { "hand": [FILLER] },
        }));

        for _ in 0..4 {
            s.play(ID, json!({}));
        }
        assert_eq!(s.view(P1).you.jade, Some(4));
        assert!(s.unit(P1, 0).is_none());

        s.play(ID, json!({}));

        assert_eq!(s.view(P1).you.jade, Some(5));
        assert!(s.unit(P1, 0).is_some_and(|unit| unit.def_id == BEAUTY));
    }

    #[test]
    fn radiant_adds_2_and_gains_2() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        let before = s.view(P1).you.mana.current;

        s.play(ID, json!({}));

        assert_eq!(s.view(P1).you.jade, Some(2));
        assert_eq!(s.view(P1).you.mana.current, before + 2);
        let jade = s.view(P1).you.jade;
        assert_eq!(s.view(P2).opponent.jade, jade);
    }
}
