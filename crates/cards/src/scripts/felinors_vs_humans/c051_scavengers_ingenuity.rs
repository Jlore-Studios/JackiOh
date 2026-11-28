//! FvH #51 Scavengers Ingenuity (SPEC §8.14 row 51): (1) Spell, Common, of the Felinors vs Humans set.
//!   Base:    "Draw a Felinor from your deck. Give it +{buff}/+{buff}." (buff 3)
//!   Radiant: the same, buff 9.
//!
//! One random Felinor-tagged card of the caster's deck, any type (its tags now, granted ones counted,
//! R923), picked with the match rng as the list reaches it (R60; a lone one is taken without a draw, and a
//! deck with none gives nothing, R129), as Classic #35 Prep's deck pick. It is drawn by
//! `draw_from_library`, a real draw: a full hand burns it (R4) and a cast on draw casts it (R58). Then each
//! Felinor this run drew that is now in a hand or on the field is buffed where it landed by the declared
//! `buff` (R386): the one picked, unless the hand cap burned it. The pick is the first part's memo and the
//! buff reads the run's own draws, so a prompt the draw opens resumes over the same card (R113, R136).
//! Both faces run one script: the Radiant face's 9 is its declared `buff`.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-051";

/// R60, R129: the Felinor of the caster's deck the draw takes, none from a deck with none.
fn felinor_in_deck(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let felinors: Vec<String> = zone_cards(&*ctx.state, ctx.controller, OffFieldZone::Library)
        .into_iter()
        .filter(|card| tags_of(&*ctx.state, card).contains(&Tag::Felinor))
        .map(|card| card.id)
        .collect();
    if felinors.len() <= 1 {
        return felinors;
    }
    ctx.rng.pick(&felinors).cloned().into_iter().collect()
}

/// R136: the Felinors this run drew for the caster that are in a hand or on the field now, each once.
fn drawn_felinors(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for event in ctx.events.iter().skip(ctx.events_from) {
        let GameEvent::Drawn { player, instance_id, .. } = event else {
            continue;
        };
        if *player != ctx.controller || ids.contains(instance_id) {
            continue;
        }
        let Some(card) = find_instance(&*ctx.state, instance_id) else {
            continue;
        };
        let landed = matches!(card.zone.z(), ZoneName::Hand | ZoneName::Field);
        if landed && tags_of(&*ctx.state, card).contains(&Tag::Felinor) {
            ids.push(instance_id.clone());
        }
    }
    ids
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "buff");
            vec![
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(felinor_in_deck),
                    each: Arc::new(|instance_id: &str| {
                        draw_from_library(json_as(json!({ "instanceId": instance_id })))
                    }),
                }),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(drawn_felinors),
                    each: Arc::new(move |instance_id: &str| {
                        buff(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "attack": amount,
                            "health": amount,
                        })))
                    }),
                }),
            ]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's +9/+9 is its declared `buff`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #51 Scavengers Ingenuity — SPEC §8.14 row 51 (R60, R129).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// Stockpile: a Spell the hand keeps and the deck is filled with.
    const FILLER: &str = "core-005";
    /// Felinor Token: the deck's Felinor.
    const FELINOR: &str = "core-t-felinor";

    fn board(radiant: bool, library: Value) -> Scenario {
        scenario(json!({
            "seed": "scavengers-ingenuity",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": library },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    /// p1's one library Felinor.
    fn deck_felinor(s: &Scenario) -> String {
        let felinor = s.pile(P1, "library").into_iter().find(|card| card.def_id == FELINOR);
        must(felinor, "a Felinor in p1's deck").id
    }

    /// The card's buffs, (attack, health).
    fn buffs_of(s: &Scenario, id: &str) -> (i32, i32) {
        let card = s.card(id);
        (card.buffs.attack, card.buffs.health)
    }

    mod base {
        use super::*;

        #[test]
        fn draws_the_felinor_from_the_deck_and_gives_it_3_3() {
            let mut s = board(false, json!([FILLER, FELINOR, FILLER]));
            let felinor = deck_felinor(&s);
            s.play(ID, json!({}));
            s.expect_in_zone(&felinor, "hand");
            assert_eq!(buffs_of(&s, &felinor), (3, 3));
            assert_eq!(s.last_count("drawn"), 1, "one draw, the Felinor's");
            assert_eq!(s.pile(P1, "library").len(), 2, "the deck's other cards stay");
        }

        #[test]
        fn r60_of_several_felinors_it_draws_one() {
            let mut s = board(false, json!([FELINOR, FILLER, FELINOR]));
            s.play(ID, json!({}));
            let drawn: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id == FELINOR).collect();
            assert_eq!(drawn.len(), 1, "one Felinor drawn");
            assert_eq!(buffs_of(&s, &drawn[0].id), (3, 3));
            let left = s.pile(P1, "library");
            let still: Vec<&CardInstance> = left.iter().filter(|card| card.def_id == FELINOR).collect();
            assert_eq!(still.len(), 1, "the other stays in the deck");
            assert_eq!(buffs_of(&s, &still[0].id), (0, 0), "unbuffed");
        }

        #[test]
        fn r129_a_deck_with_no_felinor_draws_nothing() {
            let mut s = board(false, json!([FILLER, FILLER]));
            s.play(ID, json!({}));
            assert_eq!(s.last_count("drawn"), 0);
            assert_eq!(s.last_count("buffed"), 0);
            assert_eq!(s.pile(P1, "library").len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn draws_the_felinor_from_the_deck_and_gives_it_9_9() {
            let mut s = board(true, json!([FILLER, FELINOR, FILLER]));
            let felinor = deck_felinor(&s);
            s.play(ID, json!({}));
            s.expect_in_zone(&felinor, "hand");
            assert_eq!(buffs_of(&s, &felinor), (9, 9));
        }
    }
}
