//! M #25 Blue-Eyes White Felinor (SPEC §8.8 row 25): (1) Unit, Felinor, Rare, 12/9 → 24/18.
//!
//! Base:    "Tribute 2, Immune to tribal tag based hate"
//! Radiant: "Tribute 2, Immutable, Immune to tribal tag based hate"
//! Engine:
//! - **Tribute 2** is `static_flags.tribute: 2`, paid at play time through the play validator
//!   (§6.3 Tribute X). R391 lets the zone the Tribute empties take the card.
//! - **"Immune to tribal tag based hate"** is the ME-TRIBAL keyword (MD-B1, R940): read through
//!   the layers as Immune to Spells is, protecting the card only while it acts on the field. A
//!   harmful effect whose card filter names a tribal tag in `tags` or `notTags` can neither pick
//!   it (a declaration or prompt of `aim: harm`, R656) nor reach it through a board or card scope.
//!   Helpful tag-filtered effects still reach it. The Radiant face adds Immutable (R23, R386).
//! - The keywords live in the catalog; there is nothing to run.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-025";

/// Tribute 2 on both faces; the keywords come from the catalog.
const TRIBUTE_COST: i32 = 2;

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    // The Radiant face keeps the Tribute and adds Immutable in the catalog; the script is shared.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// M #25 Blue-Eyes White Felinor — SPEC §8.8 row 25, BUILD M10 row M 25.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FELINOR: &str = "meditative-025";
    const BIGOT: &str = "core-002"; // Cry: destroy target enemy non-Human; radiant: destroy all such.
    const GAOKAO: &str = "meditative-034"; // Destroy neither-CN-nor-KY, then Buff either.
    const MANDATE: &str = "meditative-078"; // Exile every non-CN permanent.
    const RCTA: &str = "meditative-035"; // Give a non-CN permanent the CN tag (aim: help).
    const HIT_JOB: &str = "core-016"; // Destroy target unit, no tag filter.
    const VANILLA: &str = "core-008"; // Mr. Vanilla: (1) 4/4 Human, neither CN nor KY.
    const FILLER: &str = "core-005";
    const TRIBUTE_A: &str = "core-011"; // Tempo Timmy: (1) 3/3, tribute fodder.
    const TRIBUTE_B: &str = "core-020"; // Pointmaster: (2) 7/1, tribute fodder.

    fn library() -> Value {
        json!([FILLER, FILLER, FILLER, FILLER])
    }

    /// p1 holds the Felinor with two tributes on its field; p2 holds a filler hand and library.
    fn tributing(seed: &str) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 10,
                "hand": [{ "def": FELINOR }],
                "field": [{ "def": TRIBUTE_A, "lane": 1 }, { "def": TRIBUTE_B, "lane": 2 }],
                "library": library(),
            },
            "p2": { "hand": [FILLER], "library": library() },
        }))
    }

    /// p1 holds `card` and mana; p2 holds the Felinor plus `company` on its field.
    fn hunting(seed: &str, card: &str, radiant: bool, company: Value) -> Scenario {
        crate::register_all();
        let mut field = vec![json!({ "def": FELINOR, "lane": 1 })];
        for (lane, entry) in company.as_array().cloned().unwrap_or_default().into_iter().enumerate() {
            let mut entry = entry.clone();
            entry["lane"] = json!((lane + 2) as i32);
            field.push(entry);
        }
        scenario(json!({
            "seed": seed,
            "p1": { "mana": 10, "hand": [{ "def": card, "radiant": radiant }, FILLER], "library": library() },
            "p2": { "hand": [FILLER], "field": field, "library": library() },
        }))
    }

    fn felinor_id(s: &Scenario) -> String {
        s.unit(P2, 1).expect("the Felinor stands").id.clone()
    }

    #[test]
    fn the_play_needs_two_tributes() {
        let mut s = tributing("felinor-tribute");
        let a = s.unit(P1, 1).expect("tribute A").id.clone();
        // One tribute is refused.
        s.expect_refused(|s| s.play(FELINOR, json!({ "tributes": [a.clone()] })));
        // Two tributes land it.
        let b = s.unit(P1, 2).expect("tribute B").id.clone();
        s.play(FELINOR, json!({ "tributes": [a, b] }));
        s.expect_in_zone(FELINOR, "field");
    }

    #[test]
    fn r940_bigot_cannot_pick_it_and_bigots_radiant_destroy_passes_it_by() {
        // Base Bigot cannot pick it: the notTags [Human] declaration aims harm at a tribal tag.
        let mut s = hunting("felinor-bigot", BIGOT, false, json!([]));
        let felinor = felinor_id(&s);
        s.expect_refused(|s| {
            s.play(BIGOT, json!({ "targets": [{ "pick": "instance", "instanceId": felinor }] }))
        });
        // Radiant Bigot's destroy_all over notTags [Human] passes it by, killing only the victim.
        let mut s = hunting("felinor-bigot-r", BIGOT, true, json!([{ "def": VANILLA }]));
        s.play(BIGOT, json!({}));
        assert!(s.unit(P2, 1).is_some(), "the Felinor stands");
        assert_eq!(s.unit(P2, 1).expect("stands").def_id, FELINOR);
        assert!(s.unit(P2, 2).is_none(), "the non-Human victim is destroyed");
    }

    #[test]
    fn r940_gaokaos_destroy_and_occidentless_mandates_exile_pass_it_by() {
        // Gaokao destroys neither-CN-nor-KY; the Felinor (Felinor) would die but is immune.
        let mut s = hunting("felinor-gaokao", GAOKAO, false, json!([{ "def": VANILLA }]));
        s.play(GAOKAO, json!({}));
        assert_eq!(s.unit(P2, 1).expect("stands").def_id, FELINOR);
        assert!(s.unit(P2, 2).is_none(), "the Vanilla is destroyed");
        // Occidentless Mandate exiles non-CN; same passing-by.
        let mut s = hunting("felinor-mandate", MANDATE, false, json!([{ "def": VANILLA }]));
        s.play(MANDATE, json!({}));
        assert_eq!(s.unit(P2, 1).expect("stands").def_id, FELINOR);
        assert!(s.unit(P2, 2).is_none(), "the Vanilla is exiled");
    }

    #[test]
    fn r940_made_cn_by_rcta_it_is_still_immune_and_gaokao_still_buffs_it() {
        // One game: RCTA (aim: help) reaches the Felinor and makes it CN, then Gaokao resolves.
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "felinor-rcta-gaokao",
            "p1": {
                "mana": 10,
                "hand": [{ "def": RCTA }, { "def": GAOKAO }, FILLER],
                "library": library(),
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": FELINOR, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                "library": library(),
            },
        }));
        let felinor = felinor_id(&s);
        s.play(RCTA, json!({ "targets": [{ "pick": "instance", "instanceId": felinor }] }));
        let after = s.unit(P2, 1).expect("stands");
        assert!(tags_of(s.state(), &after).contains(&Tag::Cn), "it is CN now");
        let before = s.stats(&felinor);
        let mark = s.events().len();
        s.play(GAOKAO, json!({}));
        // Gaokao's destroy (notTags [CN, KY], harm) still passes the CN Felinor by …
        assert_eq!(s.unit(P2, 1).expect("stands").def_id, FELINOR);
        assert!(s.unit(P2, 2).is_none(), "the Vanilla is destroyed");
        // … and Gaokao's Buffs (tags [CN, KY], help) still reach it.
        let upgraded: Vec<_> = s.events()[mark..]
            .iter()
            .filter_map(|event| match event {
                GameEvent::Upgraded { instance_id, .. } if instance_id == &felinor => Some(()),
                _ => None,
            })
            .collect();
        assert_eq!(upgraded.len(), 2, "two Buffs land on the CN Felinor");
        assert!(s.stats(&felinor).attack > before.attack, "it grew");
    }

    #[test]
    fn r940_a_plain_destroy_still_hits_it() {
        // Hit Job names no tag: the immunity does not apply.
        let mut s = hunting("felinor-hitjob", HIT_JOB, false, json!([]));
        let felinor = felinor_id(&s);
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": felinor }] }));
        assert!(s.unit(P2, 1).is_none(), "a plain destroy still hits it");
    }

    #[test]
    fn r940_a_vanilla_takes_the_immunity() {
        // A Vanilla removes the keyword with the rest of the text: Bigot's sweep then hits it.
        let mut s = hunting("felinor-vanilla", BIGOT, true, json!([]));
        let felinor = felinor_id(&s);
        s.card_mut(felinor).vanilla = true;
        s.play(BIGOT, json!({}));
        assert!(s.unit(P2, 1).is_none(), "without its text it is no longer immune");
    }

    #[test]
    fn radiant_is_24_18_with_immutable() {
        crate::register_all();
        let def = crate::card_def(FELINOR);
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(12), Some(9), Some(24), Some(18)]
        );
        let kinds = |face: &CardFace| face.keywords.iter().map(|keyword| keyword.kind()).collect::<Vec<_>>();
        assert!(kinds(&def.base).contains(&KeywordKind::ImmuneToTribalHate));
        assert!(kinds(&def.radiant).contains(&KeywordKind::ImmuneToTribalHate));
        assert!(kinds(&def.radiant).contains(&KeywordKind::Immutable));
    }
}
