//! SPEC §8.14 FvH #46 Manifestor ((3) Unit, Legendary, 2/1 → 4/2), a card of the held Felinors vs Humans set
//! (R5235000). Built by S09 (#5259).
//!   Base:    "Cry: Summon a random (2) Cost Unit to the left of this and one to the right. Secretly choose one
//!            of them to gain Fragile. Your opponent sees Fragile on both."
//!   Radiant: the same with random Radiant (2) Cost Units.
//!
//! Engine: the Cry (R1) runs one `summon_random` of a (2) Cost Unit into each lane beside it in its row (R60),
//! Radiant on the Radiant face; an edge or a filled zone summons nothing there (R129). With two summoned, a
//! target prompt to its controller alone over exactly those two (`choose_target_where`), whose answer
//! `grant_keyword_secretly`s Fragile on the pick with both as the decoys (R5238001): the opponent's view and
//! the AI's redacted copy show Fragile on both until the truth shows (R97), and the grant reports nothing.
//! With one summoned, that one takes it at once, with no prompt; with none, nothing (R5259000). Fragile
//! destroys it at any hit that lowers its Health (R5236010). Declares no numbers.

use jackioh_engine::effects::{choose_target_where, grant_keyword_secretly, summon_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-046";

/// The resume step the secret pick comes back to (§10.6).
const SECRET: &str = "secret";

/// "a random (2) Cost Unit".
const SUMMONED_COST: i32 = 2;

/// The lanes beside the card in its unit row, those on the board.
fn lanes_beside(me: Option<&CardInstance>) -> Vec<i32> {
    let Some(me) = me else {
        return vec![];
    };
    let Zone::Field { row: Row::Units, lane, .. } = me.zone else {
        return vec![];
    };
    [lane - 1, lane + 1]
        .into_iter()
        .filter(|beside| (1..=UNIT_ZONES).contains(beside))
        .collect()
}

/// The Units this Cry summoned beside the card, on its controller's side, in the order summoned.
fn summoned_beside(ctx: &EffectContext<'_>) -> Vec<String> {
    let lanes = lanes_beside(ctx.self_.as_ref());
    let state: &GameState = ctx.state;
    summoned_so_far(ctx)
        .into_iter()
        .filter(|id| {
            find_instance(state, id).is_some_and(|card| match card.zone {
                Zone::Field { player, row: Row::Units, lane } => {
                    player == ctx.controller && lanes.contains(&lane)
                }
                _ => false,
            })
        })
        .collect()
}

/// R5238001: Fragile on the card `target` names, every summoned one shown it to the opponent.
fn fragile_on(target: TargetSpec, decoys: Vec<String>) -> Effect {
    grant_keyword_secretly(SecretGrantArgs {
        target,
        decoys,
        keyword: Keyword::Fragile,
    })
}

fn manifestor(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects: Vec<Effect> = lanes_beside(ctx.self_.as_ref())
                .into_iter()
                .map(|lane| {
                    let mut summon: SummonRandomArgs = json_as(json!({
                        "query": { "type": "Unit", "cost": SUMMONED_COST },
                        "lane": lane,
                    }));
                    if radiant {
                        summon.radiant = Some(true);
                    }
                    summon_random(summon)
                })
                .collect();
            // R5259000: one summoned is the one, and nothing is asked.
            effects.push(for_each_card(ForEachCardArgs {
                cards: Arc::new(|c: &mut EffectContext<'_>| {
                    let summoned = summoned_beside(c);
                    if summoned.len() == 1 { summoned } else { vec![] }
                }),
                each: Arc::new(|instance_id: &str| {
                    fragile_on(
                        json_as(json!({ "of": "instance", "instanceId": instance_id })),
                        vec![instance_id.to_string()],
                    )
                }),
            }));
            // Two summoned: its controller picks one of exactly those, in secret.
            let mut choose: ChooseTargetWhereArgs = json_as(json!({
                "step": SECRET,
                "scope": { "side": "ally", "of": ["unit"] },
                "prompt": "Secretly choose the one that gains Fragile",
            }));
            choose.where_ = Some(Arc::new(|c: &EffectContext<'_>, card: Option<&CardInstance>| {
                let Some(card) = card else {
                    return false;
                };
                let summoned = summoned_beside(c);
                summoned.len() > 1 && summoned.contains(&card.id)
            }));
            effects.push(choose_target_where(choose));
            effects
        })),
        resume: IndexMap::from([(
            SECRET,
            hook(|ctx| vec![fragile_on(json_as(json!({ "of": "chosen" })), summoned_beside(ctx))]),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: manifestor(false),
        radiant: manifestor(true),
    }
}

// FvH #46 Manifestor — SPEC §8.14 row 46 (R5238001, R5259000, R129, R97).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a card kept in hand, so no turn ends by itself
    const BODY: &str = "stand-in (1) 1/1"; // a Unit filling a zone

    fn played(seed: &str, radiant: bool, field: Value, lane: i32) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": field,
                "library": crate::filler(3),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }));
        s.play(ID, json!({ "zone": lane }));
        s
    }

    /// Manifestor played into lane 2 of an empty row, on the first seed whose two summons both land.
    fn both_summoned(prefix: &str, radiant: bool) -> Scenario {
        let lands = |s: &Scenario| s.unit(P1, 1).is_some() && s.unit(P1, 3).is_some();
        let seed = seed_where(prefix, |seed| lands(&played(seed, radiant, json!([]), 2)));
        played(&seed, radiant, json!([]), 2)
    }

    /// The keywords `viewer`'s view shows on the Unit `id`, on either side.
    fn shown(s: &Scenario, viewer: PlayerId, id: &str) -> Vec<KeywordKind> {
        let view = s.view(viewer);
        let unit = must(
            view.you
                .units
                .iter()
                .chain(view.opponent.units.iter())
                .flatten()
                .find(|unit| unit.instance_id == id)
                .cloned(),
            "the Unit in the view",
        );
        unit.keywords.iter().map(Keyword::kind).collect()
    }

    fn fragile(s: &Scenario, id: &str) -> bool {
        unit_has(s.state(), s.card(id), KeywordKind::Fragile)
    }

    /// The two summoned Units cost (2), the prompt is p1's over exactly them; returns (left, right).
    fn asked(s: &Scenario) -> (String, String) {
        let (left, right) = (s.unit_at(P1, 1), s.unit_at(P1, 3));
        for unit in [&left, &right] {
            assert_eq!(query_cost(&crate::card_def(&unit.def_id)), 2, "{}", unit.def_id);
            assert_eq!(crate::card_def(&unit.def_id).type_, CardType::Unit);
        }
        let pending = must(s.state().pending.clone(), "the secret pick");
        assert_eq!(pending.kind, PromptKind::Target);
        assert_eq!(pending.player_id, P1);
        let selections: Vec<Selection> = pending.options.iter().map(|option| option.selection.clone()).collect();
        let mut offered = pick::instance_ids(&selections);
        offered.sort();
        let mut both = vec![left.id.clone(), right.id.clone()];
        both.sort();
        assert_eq!(offered, both, "exactly the two summoned");
        (left.id, right.id)
    }

    /// The pick `chosen` holds Fragile and `decoy` does not; the opponent sees it on both, p1 the truth.
    fn secretly_fragile(s: &Scenario, chosen: &str, decoy: &str) {
        assert!(fragile(s, chosen), "the pick has Fragile");
        assert!(!fragile(s, decoy), "the decoy has none");
        assert!(shown(s, P2, chosen).contains(&KeywordKind::Fragile));
        assert!(shown(s, P2, decoy).contains(&KeywordKind::Fragile), "the opponent sees it on both");
        assert!(shown(s, P1, chosen).contains(&KeywordKind::Fragile));
        assert!(!shown(s, P1, decoy).contains(&KeywordKind::Fragile), "its player sees the truth");
    }

    #[test]
    fn declares_no_numbers() {
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r5238001_two_summoned_beside_it_and_the_secret_pick_gains_fragile() {
            let mut s = both_summoned("manifestor-both", false);
            let (left, right) = asked(&s);
            assert!(!s.unit_at(P1, 1).radiant && !s.unit_at(P1, 3).radiant);
            s.answer(pick::id(&right));
            assert!(s.state().pending.is_none());
            secretly_fragile(&s, &right, &left);
            assert_eq!(s.last_count("keywordGranted"), 0, "R5238001: the grant reports nothing");
        }

        #[test]
        fn r5259000_at_the_edge_one_is_summoned_and_takes_fragile_with_no_prompt() {
            let seed = seed_where("manifestor-edge", |seed| {
                played(seed, false, json!([]), 1).unit(P1, 2).is_some()
            });
            let s = played(&seed, false, json!([]), 1);
            assert!(s.state().pending.is_none(), "nothing is asked");
            let only = s.unit_at(P1, 2).id;
            assert!(fragile(&s, &only));
            assert!(shown(&s, P2, &only).contains(&KeywordKind::Fragile));
        }

        #[test]
        fn r129_a_filled_zone_beside_it_summons_nothing_there() {
            let field = json!([{ "def": BODY, "lane": 1 }]);
            let seed = seed_where("manifestor-filled", |seed| {
                played(seed, false, field.clone(), 2).unit(P1, 3).is_some()
            });
            let s = played(&seed, false, field, 2);
            assert_eq!(s.unit_at(P1, 1).def_id, BODY, "the filled zone keeps its Unit");
            assert!(!fragile(&s, &s.unit_at(P1, 1).id), "and it is none of them");
            assert!(s.state().pending.is_none());
            assert!(fragile(&s, &s.unit_at(P1, 3).id), "R5259000: the one summoned takes it");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5238001_two_radiant_units_summoned_and_the_secret_pick_gains_fragile() {
            let mut s = both_summoned("manifestor-radiant", true);
            let (left, right) = asked(&s);
            assert!(s.unit_at(P1, 1).radiant && s.unit_at(P1, 3).radiant, "Radiant ones");
            s.answer(pick::id(&left));
            secretly_fragile(&s, &left, &right);
        }
    }
}
