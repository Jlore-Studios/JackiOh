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
        assert!(s.unit(P1, 1).is_none());

        s.play(ID, json!({}));

        assert_eq!(s.view(P1).you.jade, Some(5));
        assert!(s.unit(P1, 1).is_some_and(|unit| unit.def_id == BEAUTY));
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

    #[test]
    fn r961_jade_changed_is_public_on_both_views() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [ID, FILLER] }, "p2": { "hand": [FILLER] } }));
        s.play(ID, json!({}));
        for viewer in [P1, P2] {
            let seen: Vec<GameEvent> = s
                .view(viewer)
                .events
                .into_iter()
                .filter(|event| matches!(event, GameEvent::JadeChanged { .. }))
                .collect();
            assert_eq!(
                seen,
                vec![GameEvent::JadeChanged { player: P1, value: 1 }],
                "{viewer}"
            );
        }
    }

    #[test]
    fn r961_the_counter_is_never_lowered() {
        // The crossing at 5 summons and keeps the count: the sixth Jade counts on from 5.
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ID, ID, ID, ID, ID, ID, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        let mut last = 0;
        for _ in 0..6 {
            s.play(ID, json!({}));
            let now = s.view(P1).you.jade.unwrap_or(0);
            assert!(now > last, "the counter only rises: {last} then {now}");
            last = now;
        }
        assert_eq!(last, 6);
        assert!(s.events().iter().all(|event| match event {
            GameEvent::JadeChanged { value, .. } => *value > 0,
            _ => true,
        }));
    }

    #[test]
    fn the_mana_is_temporary() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ID, FILLER], "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }));
        let max = s.view(P1).you.mana.max;
        s.play(ID, json!({}));
        // §2.3: the gain is current mana only, never the maximum, and the refresh takes it.
        assert_eq!(s.view(P1).you.mana.max, max);
        s.end_turn().end_turn();
        let mana = s.view(P1).you.mana;
        assert_eq!(mana.current, mana.max, "the next refresh leaves no extra mana");
    }

    #[test]
    fn r962_a_crossing_runs_inside_the_same_effect() {
        // From 4, the fifth Jade's crossing summons the Beauty before the Jade's mana is gained.
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ID, ID, ID, ID, ID, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        for _ in 0..4 {
            s.play(ID, json!({}));
        }
        let mark = s.events().len();
        s.play(ID, json!({}));
        let events = &s.events()[mark..];
        let jade = events
            .iter()
            .position(|event| matches!(event, GameEvent::JadeChanged { value: 5, .. }))
            .expect("the counter reached 5");
        let summoned = events
            .iter()
            .position(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == BEAUTY))
            .expect("the Beauty is summoned");
        let mana = events
            .iter()
            .rposition(|event| matches!(event, GameEvent::ManaChanged { player, .. } if *player == P1))
            .expect("the mana is gained");
        assert!(jade < summoned && summoned < mana, "{events:?}");
    }

    #[test]
    fn jade_and_mana_read_through_param() {
        crate::register_all();
        let mut s = scenario(json!({ "p1": { "hand": [ID, FILLER] }, "p2": { "hand": [FILLER] } }));
        set_param(s.card_mut(ID), "jade", 3);
        set_param(s.card_mut(ID), "mana", 4);
        let before = s.view(P1).you.mana.current;
        s.play(ID, json!({}));
        assert_eq!(s.view(P1).you.jade, Some(3));
        assert_eq!(s.view(P1).you.mana.current, before + 4);
    }
}
