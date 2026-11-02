//! Stinkyscape #7 The Greatest Heist (SPEC §8.10 row 7). (3) Field Spell, Quest, Legendary.
//!   Base:    "Cry: Discover an Acclaimed Unit or a Jelly Bean.
//!            Quest: Stage 1, Initiation. Make an Acclaimed Unit Radiant. Reward: Summon a random
//!            Acclaimed Unit.
//!            Quest: Stage 2, Comradery. Have 2 Acclaimed Units you control die. Reward: Do this
//!            {finds|time|times}: at random, add an Anomaly Found! to your hand or gain a random
//!            Stinkyscape Resource.
//!            Quest: Stage 3, Empty the Vaults. Steal an enemy card. Reward: Steal your opponent's mana
//!            crystals." — finds 3
//!   Radiant: the Discover's options, Stage 1's Unit, Stage 2's Anomaly Found! and Resource Radiant.
//!
//! Staged Quest (R1854000, next_stage). Cry: Discover Acclaimed/Jelly Bean pool (R1873001, R380).
//! Stage 1: unitsMadeRadiant (R1854002); reward summons random Acclaimed Unit (R64, R1).
//! Stage 2: unitsDied; reward rolls Anomaly Found! or Resource (R1873002, §2.4, R1840002, R1840000).
//! Stage 3: cardsStolen; reward steal_mana_crystals (R1854003).

use jackioh_engine::effects::{AddToHandArgs, GainResourcesArgs, SummonRandomArgs, gain_resources, summon_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-007";

/// The set's Anomaly Found!, Stage 2's other find.
const ANOMALY_FOUND: &str = "stinkyscape-t-anomaly-found";

/// R1873002: "at random": each find an Anomaly Found! at even odds, else a Resource.
const ANOMALY_CHANCE: f64 = 0.5;

/// The Discover's resume step.
const PICKED: &str = "picked";

/// The line's three stages, by number (R1854000).
const INITIATION: u32 = 1;
const COMRADERY: u32 = 2;
const VAULTS: u32 = 3;

/// Stage 2's "Have 2 Acclaimed Units you control die" and Stage 3's "Steal an enemy card".
const DEATHS: i32 = 2;
const STOLEN: i32 = 1;

fn stages() -> Vec<QuestStage> {
    json_as(json!([
        {
            "name": "Initiation",
            "goals": [{ "id": "radiant", "text": "Make an Acclaimed Unit Radiant",
                        "goal": { "kind": "unitsMadeRadiant", "count": 1, "tag": Tag::Acclaimed } }],
            "reward": { "id": "initiation", "text": "Summon a random Acclaimed Unit" },
        },
        {
            "name": "Comradery",
            "goals": [{ "id": "deaths", "text": "Have 2 Acclaimed Units you control die",
                        "goal": { "kind": "unitsDied", "count": DEATHS, "tag": Tag::Acclaimed } }],
            "reward": { "id": "comradery",
                        "text": "At random, add an Anomaly Found! to your hand or gain a random Stinkyscape Resource" },
        },
        {
            "name": "Empty the Vaults",
            "goals": [{ "id": "steal", "text": "Steal an enemy card",
                        "goal": { "kind": "cardsStolen", "count": STOLEN } }],
            "reward": { "id": "vaults", "text": "Steal your opponent's mana crystals" },
        },
    ]))
}

/// R1873001: "Discover an Acclaimed Unit or a Jelly Bean", one pool.
fn discover() -> Effect {
    discover_from_catalog(json_as(json!({
        "step": PICKED,
        "query": { "anyTags": [Tag::Acclaimed, Tag::JellyBean] },
        "prompt": "Discover an Acclaimed Unit or a Jelly Bean",
    })))
}

/// R1873002: Stage 2's finds, each rolled as it is found.
fn finds(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    (0..param(&*ctx, "finds"))
        .map(|_| {
            if ctx.rng.chance(ANOMALY_CHANCE) {
                add_to_hand(AddToHandArgs {
                    def_id: Some(ANOMALY_FOUND.to_string()),
                    radiant: radiant.then_some(true),
                    ..AddToHandArgs::default()
                })
            } else {
                gain_resources(GainResourcesArgs {
                    count: 1,
                    radiant: radiant.then_some(true),
                    ..GainResourcesArgs::default()
                })
            }
        })
        .collect()
}

/// Each stage's reward, on the card's own `questCompleted` (R1854000).
fn rewards(radiant: bool) -> TriggerDef {
    TriggerDef::new(
        "stage-completed",
        &[GameEventType::QuestCompleted],
        move |ctx, event| match subsystems::completed_stage_of(ctx, event) {
            Some(INITIATION) => vec![
                summon_random(SummonRandomArgs {
                    query: Some(json_as(json!({ "type": CardType::Unit, "tags": [Tag::Acclaimed] }))),
                    radiant: radiant.then_some(true),
                    ..SummonRandomArgs::default()
                }),
                next_stage(),
            ],
            Some(COMRADERY) => {
                let mut out = finds(ctx, radiant);
                out.push(next_stage());
                out
            }
            Some(VAULTS) => vec![steal_mana_crystals()],
            _ => vec![],
        },
    )
}

/// The pick arrives as a `mode` selection carrying a catalog id (§10.6).
fn add_the_pick(radiant: bool) -> Hook {
    hook(move |ctx| {
        let Some(picked) = chosen_options(ctx).into_iter().next() else {
            return vec![];
        };
        vec![add_to_hand(AddToHandArgs {
            def_id: Some(picked),
            radiant: radiant.then_some(true),
            ..AddToHandArgs::default()
        })]
    })
}

fn heist(radiant: bool) -> Script {
    Script {
        stages: stages(),
        triggers: vec![rewards(radiant)],
        cry: Some(hook(|_ctx| vec![discover()])),
        resume: IndexMap::from([(PICKED, add_the_pick(radiant))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: heist(false),
        radiant: heist(true),
    }
}

// Stinkyscape #7 The Greatest Heist — SPEC §8.10 row 7, BUILD M12 row S 7 (R1854000, R1854003,
// R78).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TYLANNT: &str = "stinkyscape-006"; // Cry: complete the current stage on all your Quests.
    const WARDRUM: &str = "classicplus-037"; // an Acclaimed Unit that ships.
    const JELLY: &str = "core-026"; // (3) Spell: Make a card in your hand Radiant.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const MIND_CONTROL: &str = "core-049"; // (4) Spell: Steal an enemy Unit.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const CORE_SYSTEM: &str = "stinkyscape-117"; // (3) Spell: Bounce all permanents.

    const MANA: i32 = 20;

    /// Re-pinned for the Training
    /// Grounds release (#2204): its Jelly Beans #27 Hopeful Jelly Bean and #50 Glowy Jelly Bean (the
    /// snapshot of Core #26) join it.
    const POOL: [&str; 18] = [
        "core-026",
        "core-027",
        "core-028",
        "core-029",
        "classic-080",
        "classicplus-037",
        "meditative-049",
        "stinkyscape-006",
        "stinkyscape-026",
        "stinkyscape-037",
        "stinkyscape-087",
        "stinkyscape-088",
        "stinkyscape-109",
        "stinkyscape-110",
        "stinkyscape-111",
        "stinkyscape-115",
        "traininggrounds-027",
        "traininggrounds-050",
    ];

    /// Each turn ends when the
    /// test ends it (R82).
    fn holding(radiant: bool, seed: &str, hand: &[&str]) -> Scenario {
        let mut cards = vec![json!({ "def": ID, "radiant": radiant })];
        cards.extend(hand.iter().map(|card| json!(card)));
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": cards, "library": filler(8), "mana": MANA },
            "p2": { "hand": [VANILLA], "field": [VANILLA], "library": filler(8) },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    fn offered(s: &Scenario) -> Vec<String> {
        let pending = s.state().pending.clone().expect("the Heist's Discover");
        assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
        pending
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Mode { option } => Some(option.clone()),
                _ => None,
            })
            .collect()
    }

    fn played(s: &mut Scenario) -> String {
        s.play(ID, json!({ "zone": 1 }));
        let first = offered(s)[0].clone();
        s.answer(json!(first));
        first
    }

    fn stage(s: &Scenario) -> Option<u32> {
        let heist = s.state().players.p1.backrow[0].as_ref()?;
        subsystems::quest_memory_of(heist).and_then(|line| line.stage)
    }

    fn tylannt(s: &mut Scenario, lane: i32) {
        s.play(TYLANNT, json!({ "zone": lane }));
    }

    #[test]
    fn r1873001_its_cry_discovers_an_acclaimed_unit_or_a_jelly_bean_into_your_hand() {
        let mut seen: IndexSet<String> = IndexSet::new();
        // Re-pinned for the Stinkyscape release (#1884): the pool is sixteen, so more seeds see it all.
        for seed in 0..40 {
            let mut s = holding(false, &format!("heist-{seed}"), &[]);
            s.play(ID, json!({ "zone": 1 }));
            let offer = offered(&s);
            assert_eq!(offer.len(), DISCOVER_OPTIONS as usize, "{offer:?}");
            assert!(offer.iter().all(|id| POOL.contains(&id.as_str())), "{offer:?}");
            seen.extend(offer.iter().cloned());
            s.answer(json!(offer[0]));
            let card = s.hand(P1).into_iter().find(|card| card.def_id == offer[0]).expect("the pick");
            assert!(!card.radiant);
        }
        assert_eq!(seen.len(), POOL.len(), "{seen:?}");
    }

    #[test]
    fn r1854000_stage_1_opens_as_it_enters() {
        let mut s = holding(false, "heist-open", &[]);
        played(&mut s);
        assert_eq!(stage(&s), Some(INITIATION));
    }

    #[test]
    fn r1854000_leaving_the_field_resets_the_line_and_played_again_it_starts_at_stage_1() {
        let mut s = holding(false, "heist-reset", &[TYLANNT, CORE_SYSTEM]);
        played(&mut s);
        tylannt(&mut s, 1);
        assert_eq!(stage(&s), Some(COMRADERY));
        s.play(CORE_SYSTEM, json!({}));
        let bounced = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == ID)
            .expect("the Heist back in hand");
        assert!(
            subsystems::quest_memory_of(&bounced).is_none(),
            "R78: the line went with the stay"
        );
        played(&mut s);
        assert_eq!(stage(&s), Some(INITIATION));
    }

    #[test]
    fn r1854002_a_jelly_bean_on_an_acclaimed_hand_card_meets_stage_1_and_a_random_acclaimed_unit_is_summoned() {
        let mut s = holding(false, "heist-stage-1", &[JELLY, WARDRUM]);
        played(&mut s);
        let wardrum = s.hand(P1).into_iter().find(|card| card.def_id == WARDRUM).expect("Wardrum").id;
        s.play(JELLY, json!({ "targets": pick::id(&wardrum) }));
        assert_eq!(stage(&s), Some(COMRADERY));
        let summoned: Vec<String> = s
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { player: P1, def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(summoned.len(), 1, "{summoned:?}");
        let def = crate::card_def(&summoned[0]);
        assert!(def.tags.contains(&Tag::Acclaimed) && def.type_ == CardType::Unit, "{}", def.id);
    }

    #[test]
    fn r1854002_two_acclaimed_units_of_yours_dying_meet_stage_2() {
        let mut s = holding(false, "heist-stage-2", &[TYLANNT, HIT_JOB, HIT_JOB]);
        played(&mut s);
        tylannt(&mut s, 1);
        assert_eq!(stage(&s), Some(COMRADERY));
        let acclaimed: Vec<String> = (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|unit| crate::card_def(&unit.def_id).tags.contains(&Tag::Acclaimed))
            .map(|unit| unit.id)
            .collect();
        assert_eq!(acclaimed.len(), 2, "Tylannt and the Unit Stage 1 summoned");
        s.play(HIT_JOB, json!({ "targets": pick::id(&acclaimed[0]) }));
        assert_eq!(stage(&s), Some(COMRADERY));
        s.play(HIT_JOB, json!({ "targets": pick::id(&acclaimed[1]) }));
        assert_eq!(stage(&s), Some(VAULTS));
    }

    #[test]
    fn r1854002_an_enemy_acclaimed_unit_dying_counts_for_nothing() {
        let mut s = holding(false, "heist-enemy", &[TYLANNT, HIT_JOB, HIT_JOB]);
        s.state_mut().players.p2.auto_end_turn = Some(false);
        played(&mut s);
        tylannt(&mut s, 1);
        assert_eq!(stage(&s), Some(COMRADERY));
        let p1_acclaimed = s.unit(P1, 2).expect("the Unit Stage 1 summoned").id;
        s.play(HIT_JOB, json!({ "targets": pick::id(&p1_acclaimed) }));
        let enemy = s.unit(P2, 1).expect("p2's Vanilla").id;
        s.card_mut(enemy.as_str()).granted_tags.get_or_insert_with(Vec::new).push(Tag::Acclaimed);
        s.play(HIT_JOB, json!({ "targets": pick::id(&enemy) }));
        assert_eq!(stage(&s), Some(COMRADERY));
    }

    fn at_the_vaults(radiant: bool, seed: &str, hand: &[&str]) -> Scenario {
        let mut cards = vec![TYLANNT, TYLANNT];
        cards.extend_from_slice(hand);
        let mut s = holding(radiant, seed, &cards);
        played(&mut s);
        tylannt(&mut s, 1);
        tylannt(&mut s, 3);
        assert_eq!(stage(&s), Some(VAULTS));
        s
    }

    fn found(s: &Scenario) -> (Vec<bool>, Vec<bool>) {
        let anomalies = s.hand(P1).into_iter().filter(|card| card.def_id == ANOMALY_FOUND).map(|card| card.radiant).collect();
        let resources = resources_in_hand_of(s.state(), P1).into_iter().map(|card| card.radiant).collect();
        (anomalies, resources)
    }

    #[test]
    fn r1873002_stage_2s_reward_is_3_finds_each_an_anomaly_found_or_a_resource() {
        let mut kinds: IndexSet<(usize, usize)> = IndexSet::new();
        for seed in 0..8 {
            let s = at_the_vaults(false, &format!("heist-finds-{seed}"), &[]);
            let (anomalies, resources) = found(&s);
            assert_eq!(anomalies.len() + resources.len(), 3, "seed {seed}");
            assert!(anomalies.iter().chain(&resources).all(|radiant| !radiant));
            kinds.insert((anomalies.len(), resources.len()));
        }
        assert!(kinds.len() > 1, "both kinds are found: {kinds:?}");
    }

    #[test]
    fn r386_finds_reads_through_param() {
        let mut s = holding(false, "heist-param", &[TYLANNT, TYLANNT]);
        step_param(s.card_mut(ID), "finds", 1);
        played(&mut s);
        tylannt(&mut s, 1);
        tylannt(&mut s, 3);
        let (anomalies, resources) = found(&s);
        assert_eq!(anomalies.len() + resources.len(), 4);
    }

    #[test]
    fn r1854002_r1854003_stealing_an_enemy_card_meets_stage_3_which_steals_their_mana_crystals() {
        let mut s = at_the_vaults(false, "heist-vaults", &[MIND_CONTROL]);
        // A round, so each side holds crystals (R1854003: no maximum mana, nothing to steal). Stage 1's
        // Unit may be a Wardrum, whose end of turn casts a copy of the Heist, asking its Discover again.
        for _ in 0..2 {
            s.end_turn();
            while s.state().pending.is_some() {
                let first = offered(&s)[0].clone();
                s.answer(json!(first));
            }
        }
        s.state_mut().players.p1.mana.current = MANA;
        let theirs = s.state().players.p2.mana.max;
        let mine = s.state().players.p1.mana.max;
        let target = s.unit(P2, 1).expect("p2's Vanilla").id;
        s.play(MIND_CONTROL, json!({ "targets": pick::id(&target) }));
        let line = subsystems::quest_memory_of(s.state().players.p1.backrow[0].as_ref().expect("the Heist"))
            .expect("its line");
        assert_eq!(line.done.len(), 3, "the line is done: {line:?}");
        assert!(line.active.is_empty());
        let lost = theirs - s.state().players.p2.mana.max;
        assert!(lost > 0, "their crystals taken");
        assert_eq!(
            s.state().players.p1.mana.max,
            (mine + lost).min(MAX_MANA),
            "given to p1, never past the cap"
        );
    }

    #[test]
    fn radiant_the_options_stage_1s_unit_and_stage_2s_finds_are_radiant() {
        let mut s = holding(true, "heist-radiant", &[TYLANNT, TYLANNT]);
        s.play(ID, json!({ "zone": 1 }));
        let pick = offered(&s)[0].clone();
        s.answer(json!(pick));
        // Re-pinned for the Training Grounds release (#2204): the seed's first option is now S #6
        // Tylannt, which the hand already holds two of, so the pick is the one added last.
        let card = s.hand(P1).into_iter().rev().find(|card| card.def_id == pick).expect("the pick");
        assert!(card.radiant);
        tylannt(&mut s, 1);
        let summoned = s
            .last_events()
            .iter()
            .find_map(|event| match event {
                GameEvent::Summoned { player: P1, instance_id, .. } if s.card(instance_id).def_id != TYLANNT => {
                    Some(s.card(instance_id).clone())
                }
                _ => None,
            })
            .expect("Stage 1's Unit");
        assert!(summoned.radiant);
        tylannt(&mut s, 3);
        let (anomalies, resources) = found(&s);
        assert_eq!(anomalies.len() + resources.len(), 3);
        assert!(anomalies.iter().chain(&resources).all(|radiant| *radiant));
    }
}
