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
    const ANXIETY: &str = "meditative-052"; // +3 Attack attacking a Unit sharing no tag.
    const SCARAB: &str = "core-007"; // Jewelosco Scarab: (1) 1/1 with no tag and no Taunt.
    const HIT_JOB: &str = "core-016"; // Destroy target unit, no tag filter.
    const VANILLA: &str = "core-008"; // Mr. Vanilla: (1) 4/4 Human, neither CN nor KY.
    const MENACE: &str = "core-019"; // Midrange Menace: (3) 9/9 with no tag, so not Human.
    const FILLER: &str = "core-005";
    const TRIBUTE_A: &str = "core-011"; // Tempo Timmy: (1) 3/3, tribute fodder.
    const TRIBUTE_B: &str = "core-020"; // Pointmaster: (2) 7/1, tribute fodder.
    const LAR: &str = "classicplus-076"; // Brother Lar: (1) 1/1 CN Human.
    const BREAKER: &str = "meditative-045"; // Cry: Nerf every other CN or KY card, field, hands, decks.
    const PRIME: &str = "meditative-045-1"; // Cry: destroy (radiant: exile) every other CN or KY permanent.
    const FLAGBEARER: &str = "classicplus-046"; // Aura: your other Felinors have +1/+1.
    const FIENDER: &str = "core-092"; // Also has the stats of all your Felinors.

    use jackioh_engine::effects::{degrade, transform, upgrade, vanilla};
    use jackioh_engine::resolve::{HookOptions, make_context};

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
        // Radiant Bigot's destroy_all over notTags [Human] passes it by, killing only the victim
        // (untagged, so not Human: Mr. Vanilla is Human and Bigot spares it).
        let mut s = hunting("felinor-bigot-r", BIGOT, true, json!([{ "def": MENACE }]));
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
        let cost_before = cost_now(s.state(), s.card(&felinor));
        let mark = s.events().len();
        s.play(GAOKAO, json!({}));
        // Gaokao's destroy (notTags [CN, KY], harm) still passes the CN Felinor by …
        assert_eq!(s.unit(P2, 1).expect("stands").def_id, FELINOR);
        assert!(s.unit(P2, 2).is_none(), "the Vanilla is destroyed");
        // … and Gaokao's Buffs (tags [CN, KY], help) still reach it.
        let upgraded: Vec<TuningChange> = s.events()[mark..]
            .iter()
            .filter_map(|event| match event {
                GameEvent::Upgraded { instance_id, change, .. } if instance_id == &felinor => Some(change.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(upgraded.len(), 2, "two Buffs land on the CN Felinor");
        // A Buff improves one row drawn at random (stats, cost or a keyword, B3.4), so each one
        // changed something and the card grew, whichever rows the draws took.
        assert!(upgraded.iter().all(|change| *change != TuningChange::None), "{upgraded:?}");
        let after = s.stats(&felinor);
        assert!(
            after.attack > before.attack
                || after.max_health > before.max_health
                || after.keywords.len() > before.keywords.len()
                || cost_now(s.state(), s.card(&felinor)) < cost_before,
            "it grew: {before:?} → {after:?}"
        );
    }

    #[test]
    fn r940_economic_anxietys_bonus_does_not_apply_against_it() {
        // p1's Human behind Economic Anxiety shares no tag with the Felinor nor the untagged Scarab.
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "felinor-anxiety",
            "p1": { "field": [VANILLA], "backrow": [ANXIETY], "library": library() },
            "p2": {
                "field": [{ "def": FELINOR, "lane": 1 }, { "def": SCARAB, "lane": 2 }],
                "library": library(),
            },
        }));
        let felinor = felinor_id(&s);
        let scarab = s.unit(P2, 2).expect("the Scarab stands").id.clone();
        let vanilla = s.unit(P1, 1).expect("the Vanilla stands").id.clone();
        // The yellow (R1121) lists the Scarab, which takes the +3, and never the Felinor …
        let view = s.view(P1);
        let attacker = view.you.units.iter().flatten().find(|unit| unit.instance_id == vanilla);
        let yellow = attacker.and_then(|unit| unit.condition_targets.clone());
        assert_eq!(yellow, Some(vec![scarab]));
        // … and the Human's strike on it is its printed 4, never 4 + 3.
        s.attack(&vanilla, &felinor);
        assert_eq!(s.card(&felinor).damage, 4);
        s.expect_in_zone(&felinor, "field");
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

    /// Apply one effect as `controller`'s, outside any card.
    fn run_as(s: &mut Scenario, effect: Effect, controller: PlayerId) {
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
        {
            let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
            let mut ctx = make_context(&mut sink, None, HookOptions { controller: Some(controller), ..Default::default() });
            (effect.apply)(&mut ctx);
        }
        s.state_mut().rng_cursor = rng.cursor();
    }

    /// The board of `hunting` with the Felinor made CN (M 35's result) and a CN Brother Lar beside it.
    fn cn_hunting(seed: &str, card: &str, radiant: bool) -> Scenario {
        let mut s = hunting(seed, card, radiant, json!([{ "def": LAR }]));
        let felinor = felinor_id(&s);
        s.card_mut(felinor).granted_tags = Some(vec![Tag::Cn]);
        s
    }

    #[test]
    fn r391_tribute_2_may_be_paid_from_its_own_zone() {
        let mut s = tributing("felinor-own-zone");
        let a = s.unit(P1, 1).expect("tribute A").id.clone();
        let b = s.unit(P1, 2).expect("tribute B").id.clone();
        s.play(FELINOR, json!({ "zone": 1, "tributes": [a, b] }));
        assert_eq!(s.unit(P1, 1).expect("it stands").def_id, FELINOR);
    }

    #[test]
    fn r940_knowledge_breakers_nerfs_pass_it_by() {
        let mut s = cn_hunting("felinor-breaker", BREAKER, false);
        let felinor = felinor_id(&s);
        let lar = s.unit(P2, 2).expect("Lar").id.clone();
        s.play(BREAKER, json!({ "zone": 1 }));
        assert!(s.card(&lar).tuning.is_some(), "the other CN card is Nerfed");
        assert!(s.card(&felinor).tuning.is_none(), "the CN Felinor is passed by");
    }

    #[test]
    fn r940_knowledge_breaker_primes_destroy_and_radiant_exile_pass_it_by() {
        for (radiant, zone) in [(false, "graveyard"), (true, "exile")] {
            let mut s = cn_hunting("felinor-prime", PRIME, radiant);
            let felinor = felinor_id(&s);
            let lar = s.unit(P2, 2).expect("Lar").id.clone();
            s.play(PRIME, json!({ "zone": 1 }));
            assert_eq!(s.card(&lar).zone.z().as_str(), zone, "the other CN permanent goes ({radiant})");
            assert_eq!(s.unit(P2, 1).map(|unit| unit.id), Some(felinor), "the Felinor stands ({radiant})");
        }
    }

    #[test]
    fn r940_helpful_tag_filtered_effects_reach_it() {
        // Felinor Flagbearer's aura: "Your other Felinors have +1/+1".
        let mut s = hunting("felinor-flagbearer", FILLER, false, json!([{ "def": FLAGBEARER }]));
        let felinor = felinor_id(&s);
        s.expect_stats(&felinor, json!({ "attack": 13, "health": 10 }));
        // Felinor Fiender's sum: "Also has the stats of all your Felinors".
        let s = hunting("felinor-fiender", FILLER, false, json!([{ "def": FIENDER }]));
        let fiender = s.unit(P2, 2).expect("the Fiender").id.clone();
        let printed = crate::card_def(FIENDER).base.attack.expect("an attack");
        assert_eq!(s.stats(&fiender).attack, printed + 12, "the sum counts the Felinor");
    }

    #[test]
    fn r940_in_hand_or_deck_it_protects_nothing() {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "felinor-off-field",
            "p1": { "mana": 10, "hand": [BREAKER, FILLER], "library": library() },
            "p2": { "hand": [FELINOR, FILLER], "library": [FELINOR, FILLER, FILLER] },
        }));
        let held = s.pile(P2, "hand").into_iter().find(|card| card.def_id == FELINOR).expect("in hand").id;
        let deck = s.pile(P2, "library").into_iter().find(|card| card.def_id == FELINOR).expect("in deck").id;
        for id in [&held, &deck] {
            s.card_mut(id.as_str()).granted_tags = Some(vec![Tag::Cn]);
        }
        s.play(BREAKER, json!({ "zone": 1 }));
        assert!(s.card(&held).tuning.is_some(), "Nerfed in hand");
        assert!(s.card(&deck).tuning.is_some(), "Nerfed in the deck");
    }

    #[test]
    fn r940_a_nerf_can_take_the_keyword() {
        // A Nerf draws one row at random; on a seed where it takes the keyword, Bigot's sweep hits it.
        let mut found = false;
        for cursor in 0..64 {
            let mut s = hunting("felinor-nerfed", BIGOT, true, json!([]));
            let felinor = felinor_id(&s);
            s.state_mut().rng_cursor = cursor;
            run_as(&mut s, degrade(json_as(json!({ "instanceId": felinor }))), P1);
            let removed = s
                .card(&felinor)
                .tuning
                .as_ref()
                .and_then(|tuning| tuning.remove_keywords.clone())
                .unwrap_or_default();
            if !removed.contains(&KeywordKind::ImmuneToTribalHate) {
                continue;
            }
            assert!(!s.stats(&felinor).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::ImmuneToTribalHate));
            s.play(BIGOT, json!({}));
            assert!(s.unit(P2, 1).is_none(), "Nerfed out of its keyword, it is no longer immune");
            found = true;
            break;
        }
        assert!(found, "some Nerf takes the keyword");
    }

    #[test]
    fn tribute_2_is_a_fixed_static_flag_on_both_faces() {
        let scripts = script();
        for face in [&scripts.base, &scripts.radiant] {
            assert_eq!(face.static_flags.as_ref().and_then(|flags| flags.tribute), Some(2));
        }
        // A Nerf never moves it: it is no declared number.
        crate::register_all();
        assert!(crate::card_def(FELINOR).params.is_none());
    }

    #[test]
    fn r23_radiant_is_immutable_no_nerf_buff_transform_or_vanilla() {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "felinor-immutable",
            "p1": { "hand": [FILLER], "library": library() },
            "p2": { "hand": [FILLER], "field": [{ "def": FELINOR, "radiant": true, "lane": 1 }], "library": library() },
        }));
        let felinor = felinor_id(&s);
        run_as(&mut s, degrade(json_as(json!({ "instanceId": felinor }))), P1);
        run_as(&mut s, upgrade(json_as(json!({ "instanceId": felinor }))), P1);
        run_as(&mut s, vanilla(json_as(json!({ "instanceId": felinor }))), P1);
        run_as(&mut s, transform(json_as(json!({ "instanceId": felinor, "defId": VANILLA }))), P1);
        let card = s.card(&felinor).clone();
        assert_eq!(card.def_id, FELINOR, "no Transform");
        assert!(card.tuning.is_none(), "no Nerf or Buff");
        assert!(!card.vanilla, "no Vanilla");
        s.expect_stats(&felinor, json!({ "attack": 24, "health": 18 }));
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
