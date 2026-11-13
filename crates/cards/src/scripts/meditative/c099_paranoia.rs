//! M #99 Paranoia (SPEC §8.8 row 99): (2) Field Spell, Epic.
//!
//! Base:    "Aura: Your Spells have Cloaked. Spells you set face-down resolve when set and when
//!           revealed.
//!           Cry: Draw {draw}."
//! Radiant: "Aura: Your Spells have Cloaked. Spells you set face-down resolve when set and when
//!           revealed. Spells you reveal have Echo +{echo}.
//!           Cry: Draw {draw}."
//! Engine: Cloaked for the Spells in its controller's hand while it acts (`cloak_hand_aura`, R1044);
//! a Spell its controller sets face-down while it acts resolves as it is set, its choices prompted,
//! and stays set to resolve again as it reveals (`resolves_set_spells`, R3806000); the Radiant face's
//! Echo +{echo} for each Spell its controller reveals while it acts, read as the Spell reveals
//! (`reveal_echo`, R1045), never as it is set; the Cry draws. Kept script v0.6.0d (R767000).

use jackioh_engine::alt_play::cloak_hand_aura;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-099";

fn paranoia(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let count = param(&*ctx, "draw");
            vec![draw(json_as(json!({ "count": count })))]
        })),
        aura: Some(cloak_hand_aura(CardType::Spell)),
        reveal_echo: radiant.then(|| -> RevealEchoHook { Arc::new(|args| param(&args, "echo")) }),
        resolves_set_spells: true,
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: paranoia(false),
        radiant: paranoia(true),
    }
}

/// R767000: the script v0.6.0d and before ran: a set Spell resolved only as it revealed (before
/// R3806000). `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_0d() -> CardScripts {
    let mut scripts = script();
    scripts.base.resolves_set_spells = false;
    scripts.radiant.resolves_set_spells = false;
    scripts
}

// M #99 Paranoia — SPEC §8.8 row 99, BUILD M10 row M 99.
#[cfg(test)]
mod tests {
    use jackioh_engine::alt_play::CLOAKED_REVEAL;
    use jackioh_engine::prelude::keywords_of;
    use jackioh_engine::testkit::*;

    use super::script_v0_6_0d;
    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PARANOIA: &str = "meditative-099";
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw, heal; no declared targets
    const ECLIPSE: &str = "core-035"; // Lunar Eclipse: Deal 3 to a target
    const VANILLA: &str = "core-008";
    const SOLARIUS: &str = "classicplus-038"; // Solarius: Spell Damage +2
    const TOE_CRACKER: &str = "classic-006"; // Cloaked Toe Cracker: Aura: your Traps cost nothing
    const COUNTERSPELL: &str = "classic-017"; // Counterspell: counters the opponent's next Spell
    const HIT_JOB: &str = "core-016"; // Hit Job: destroy a Unit
    const SURGERY: &str = "core-063"; // Plastic Surgery: buff a Unit, a needed target (R703)
    const GREAT_WALL: &str = "meditative-097-7"; // The Great Wall: Immutable
    const TRUE_STRIKE: &str = "core-044"; // True Strike: deal 4 to a target, exile this
    const CATALYST_COMET: &str = "stinkyscape-156"; // Catalyst Comet: add a Catalyst card, shuffle this into your deck

    /// Answer the open Target prompt at the enemy hero.
    fn answer_hero(s: &mut Scenario) {
        let pending = s.state().pending.clone().expect("an open prompt");
        let picked = pending
            .options
            .iter()
            .find(|option| option.selection == Selection::Hero { player: P2 })
            .expect("the enemy hero")
            .selection
            .clone();
        s.answer(json!([picked]));
    }

    fn set_plays(s: &Scenario, id: &str) -> Vec<ActionBody> {
        legal_actions(s.state(), P1)
            .into_iter()
            .filter(|action| {
                matches!(
                    action,
                    ActionBody::Play { instance_id, face_down: Some(_), .. }
                    if instance_id == id
                )
            })
            .collect()
    }

    fn cloaked(s: &Scenario, id: &str) -> bool {
        keywords_of(s.state(), s.card(id))
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Cloaked)
    }

    fn in_hand(s: &Scenario, def: &str) -> CardInstance {
        s.hand(P1)
            .iter()
            .find(|card| card.def_id == def)
            .cloned()
            .expect("the card in p1's hand")
    }

    /// Paranoia acting in p1's backrow (Radiant on `radiant`), `hand` in p1's hand, `p1_field` beside it.
    fn paranoid_as(seed: &str, radiant: bool, hand: Value, p1_field: Value, p2: Value) -> Scenario {
        let mut field = vec![json!({ "def": PARANOIA, "lane": 1, "radiant": radiant })];
        field.extend(p1_field.as_array().cloned().unwrap_or_default());
        let mut p2 = p2;
        if p2.get("hand").is_none() {
            p2["hand"] = json!([VANILLA]);
        }
        p2["library"] = json!([VANILLA, VANILLA]);
        scenario(json!({
            "seed": seed,
            "p1": { "mana": 8, "hand": hand, "field": field, "library": [VANILLA, VANILLA] },
            "p2": p2,
        }))
    }

    fn paranoid(seed: &str, hand: Value, p1_field: Value, p2: Value) -> Scenario {
        paranoid_as(seed, false, hand, p1_field, p2)
    }

    /// No Paranoia out: `hand` in p1's hand, each card of `cloaked` given Cloaked by a Buff (R2727000).
    fn buffed(seed: &str, hand: Value, cloaked: &[&str]) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "mana": 8, "hand": hand, "library": [VANILLA, VANILLA] },
            "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA] },
        }));
        for def in cloaked {
            let id = in_hand(&s, def).id;
            s.card_mut(id.as_str()).tuning = Some(Tuning {
                add_keywords: Some(vec![Keyword::Cloaked]),
                ..Tuning::default()
            });
        }
        s
    }

    /// Whether p2's view names the definition anywhere: its board, its events, its prompts (R1046).
    fn named_to_p2(s: &Scenario, def: &str) -> bool {
        serde_json::to_string(&s.view(P2))
            .expect("a view serializes")
            .contains(def)
    }

    fn open_kind(s: &Scenario) -> Option<PromptKind> {
        s.state().pending.as_ref().map(|pending| pending.kind)
    }

    /// Set the card of `def` face-down by its Cloaked into backrow lane 2; the set card's fresh id.
    fn set(s: &mut Scenario, def: &str) -> String {
        let card = in_hand(s, def);
        s.play(&card.id, json!({ "zone": 2, "row": "backrow", "faceDown": "cloaked" }));
        // A set card takes a fresh id (R1040).
        s.backrow(P1, 2).expect("the set card").id
    }

    fn reveal(s: &mut Scenario, id: &str) {
        s.activate(id, json!({ "ability": CLOAKED_REVEAL }));
    }

    fn reveal_offered(s: &Scenario, id: &str) -> bool {
        legal_actions(s.state(), P1).into_iter().any(|action| {
            matches!(&action, ActionBody::Activate { instance_id, ability, .. }
                if instance_id == id && ability.as_deref() == Some(CLOAKED_REVEAL))
        })
    }

    /// The card of `def` in p1's graveyard, if one is there.
    fn in_grave(s: &Scenario, def: &str) -> Option<CardInstance> {
        s.state()
            .players
            .p1
            .graveyard
            .iter()
            .find(|card| card.def_id == def)
            .cloned()
    }

    /// Paranoia leaves p1's field (lane 1 of the backrow) for the graveyard.
    fn paranoia_leaves(s: &mut Scenario) {
        let state = s.state_mut();
        let mut aura = state.players.p1.backrow[0].take().expect("Paranoia");
        aura.zone = Zone::Graveyard { player: P1 };
        state.players.p1.graveyard.push(aura);
    }

    mod base {
        use super::*;

        #[test]
        fn cry_draws_one() {
            let mut s = scenario(json!({
                "seed": "paranoia-draw",
                "p1": { "mana": 8, "hand": [PARANOIA], "library": [VANILLA, VANILLA] },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            let before = s.hand(P1).len();
            let paranoia = in_hand(&s, PARANOIA);
            s.play(&paranoia.id, json!({}));
            assert_eq!(s.hand(P1).len(), before);
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn r1044_its_aura_cloaks_your_hand_spells_but_no_unit_or_field_spell() {
            let s = paranoid(
                "paranoia-cloaks",
                json!([PARANOIA, STOCKPILE, VANILLA]),
                json!([]),
                json!({}),
            );
            let spell = in_hand(&s, STOCKPILE);
            assert!(cloaked(&s, &spell.id));
            let plays = set_plays(&s, &spell.id);
            assert!(!plays.is_empty());
            // No timing to choose: a Cloaked set alone.
            assert!(
                plays
                    .iter()
                    .all(|action| matches!(action, ActionBody::Play { face_down: Some(RevealAt::Cloaked), .. }))
            );
            for other in [VANILLA, PARANOIA] {
                let card = in_hand(&s, other);
                assert!(!cloaked(&s, &card.id), "{other} is not a Spell");
                assert_eq!(set_plays(&s, &card.id), vec![]);
            }
        }

        #[test]
        fn r1044_no_spell_damage() {
            let mut s = paranoid(
                "paranoia-nodamage",
                json!([ECLIPSE, VANILLA]),
                json!([{ "def": SOLARIUS, "lane": 1 }]),
                json!({}),
            );
            let health = s.state().players.p2.hero.health;
            // Set, it resolves as it is set (R3806000): its 3 to the chosen hero is a Trap's, so
            // Solarius's Spell Damage +2 adds nothing (MD-E17).
            let eclipse = set(&mut s, ECLIPSE);
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 3);
            // Revealed, it deals its 3 again, a Trap's still.
            reveal(&mut s, &eclipse);
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 6);
        }

        #[test]
        fn r1044_set_at_its_price_shown_as_a_trap_play_with_its_cost() {
            let mut s = paranoid("paranoia-price", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
            let mana = s.state().players.p1.mana.current;
            set(&mut s, ECLIPSE);
            assert_eq!(s.state().players.p1.mana.current, mana - 1, "Lunar Eclipse's (1)");
            let by_type = s
                .state()
                .players
                .p1
                .turn_log
                .played_by_type
                .clone()
                .unwrap_or_default();
            assert_eq!(by_type.get(&CardType::Trap), Some(&1), "a Trap play: {by_type:?}");
            assert_eq!(by_type.get(&CardType::Spell), None);
            // The opponent sees a face-down card and its cost, nothing more.
            let foe = serde_json::to_value(s.view(P2).opponent.backrow[1].clone()).unwrap();
            assert_eq!(foe["faceDown"], json!(true));
            assert_eq!(foe["cost"], json!(1));
            assert!(foe.get("defId").is_none());
        }

        #[test]
        fn r1044_a_set_spell_is_priced_as_itself_so_toe_crackers_traps_discount_misses_it() {
            let mut s = paranoid(
                "paranoia-discount",
                json!([ECLIPSE, VANILLA]),
                json!([TOE_CRACKER]),
                json!({}),
            );
            let mana = s.state().players.p1.mana.current;
            set(&mut s, ECLIPSE);
            assert_eq!(s.state().players.p1.mana.current, mana - 1, "priced as the Spell: (1)");
        }

        #[test]
        fn r1044_a_counter_for_spells_never_sees_it() {
            let mut s = paranoid(
                "paranoia-counter",
                json!([ECLIPSE, VANILLA]),
                json!([]),
                json!({ "backrow": [{ "def": COUNTERSPELL, "faceUp": false }] }),
            );
            let counter = s.backrow(P2, 1).unwrap().id;
            let eclipse = set(&mut s, ECLIPSE);
            s.expect_in_zone(&eclipse, "field");
            s.expect_in_zone(&counter, "field");
            assert!(
                !s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Countered { .. })),
                "nothing was countered"
            );
        }

        #[test]
        fn r1044_it_never_reveals_by_itself_and_its_reveal_asks_then_resolves() {
            let mut s = paranoid("paranoia-reveal", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
            let health = s.state().players.p2.hero.health;
            let eclipse = set(&mut s, ECLIPSE);
            answer_hero(&mut s);
            s.end_turn().end_turn().end_turn().end_turn();
            assert!(s.state().pending.is_none());
            s.expect_in_zone(&eclipse, "field");
            reveal(&mut s, &eclipse);
            assert_eq!(open_kind(&s), Some(PromptKind::Target));
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 6);
            assert!(in_grave(&s, ECLIPSE).is_some());
        }

        #[test]
        fn r1044_r703_r3806000_a_needed_target_with_no_pick_bars_the_set_under_paranoia_and_the_reveal_under_a_buff()
         {
            // Plastic Surgery's Unit target is needed (R703). Under Paranoia it would resolve as it is
            // set, so with no Unit anywhere it is neither offered set nor set.
            let mut s = paranoid("paranoia-fizzle", json!([SURGERY, VANILLA]), json!([]), json!({}));
            let surgery = in_hand(&s, SURGERY);
            assert!(cloaked(&s, &surgery.id));
            assert_eq!(set_plays(&s, &surgery.id), vec![]);
            s.expect_refused(|s| {
                s.play(&surgery.id, json!({ "zone": 2, "row": "backrow", "faceDown": "cloaked" }))
            });

            // Cloaked by a Buff with no Paranoia out, it is set, resolving nothing, and its Reveal waits
            // for a Unit.
            let mut s = buffed("paranoia-fizzle-buffed", json!([SURGERY, VANILLA]), &[SURGERY]);
            let surgery = set(&mut s, SURGERY);
            assert!(s.state().pending.is_none());
            assert!(!reveal_offered(&s, &surgery));
            s.expect_refused(|s| s.activate(surgery.as_str(), json!({ "ability": CLOAKED_REVEAL })));
            s.expect_in_zone(&surgery, "field");
            s.play(VANILLA, json!({ "zone": 1 }));
            assert!(reveal_offered(&s, &surgery));
        }

        #[test]
        fn r1044_r90_a_target_it_can_go_without_lets_it_reveal_and_fizzle() {
            // Hit Job's target is not a needed one: with no Unit anywhere it reveals, as it could be
            // played face-up, and fizzles into the graveyard.
            let mut s = paranoid("paranoia-hit-job", json!([HIT_JOB, VANILLA]), json!([]), json!({}));
            let hit_job = set(&mut s, HIT_JOB);
            assert!(reveal_offered(&s, &hit_job));
            reveal(&mut s, &hit_job);
            assert!(s.state().pending.is_none(), "no prompt: it fizzled");
            assert!(in_grave(&s, HIT_JOB).is_some(), "the Spell lands in the graveyard");
        }

        #[test]
        fn r1044_it_goes_to_your_graveyard_as_the_spell() {
            let mut s = paranoid("paranoia-grave", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
            let eclipse = set(&mut s, ECLIPSE);
            answer_hero(&mut s);
            assert!(in_grave(&s, ECLIPSE).is_none(), "still set after it resolved as it was set");
            reveal(&mut s, &eclipse);
            answer_hero(&mut s);
            let card = in_grave(&s, ECLIPSE).expect("the Spell in the graveyard");
            assert_eq!(card.set_as, None);
            assert_eq!(card_type_of(s.state(), &card), CardType::Spell);
            assert_eq!(card.owner, P1);
        }

        #[test]
        fn r1045_set_before_paranoia_left_it_stays_set_and_still_reveals() {
            let mut s = paranoid("paranoia-left", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
            let health = s.state().players.p2.hero.health;
            let eclipse = set(&mut s, ECLIPSE);
            answer_hero(&mut s);
            paranoia_leaves(&mut s);
            s.expect_in_zone(&eclipse, "field");
            s.end_turn().end_turn();
            s.expect_in_zone(&eclipse, "field");
            // Paranoia gone, it still resolves as it reveals (R3806000).
            reveal(&mut s, &eclipse);
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 6);
        }

        #[test]
        fn r1045_bounced_it_is_the_spell_again() {
            let mut s = paranoid("paranoia-bounce", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
            let eclipse = set(&mut s, ECLIPSE);
            answer_hero(&mut s);
            let card = s.card(eclipse.as_str()).clone();
            assert!(card.set_as.is_some());
            {
                let state = s.state_mut();
                let mut events = Vec::new();
                let mut rng = Rng::new(&state.seed, state.rng_cursor);
                let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
                jackioh_engine::effects::move_::bounce_card(&mut sink, &card);
            }
            let home = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == ECLIPSE)
                .expect("the bounced Spell in hand");
            assert_eq!(home.set_as, None);
            assert_eq!(card_type_of(s.state(), &home), CardType::Spell);
        }

        #[test]
        fn r781001_its_revealed_spell_cannot_target_an_immutable_unit() {
            // The Great Wall (Immutable) on p2's field: the revealed Lunar Eclipse is a Spell card
            // however it was played, so its prompt never lists the Wall.
            let mut s = paranoid(
                "paranoia-immutable",
                json!([ECLIPSE, VANILLA]),
                json!([]),
                json!({ "field": [{ "def": GREAT_WALL, "lane": 1 }] }),
            );
            let wall = s.unit(P2, 1).expect("the Wall").id;
            let eclipse = set(&mut s, ECLIPSE);
            // As it is set (R3806000) and as it reveals, its prompt never lists the Wall.
            for resolution in ["set", "reveal"] {
                if resolution == "reveal" {
                    reveal(&mut s, &eclipse);
                }
                let pending = s.state().pending.clone().expect("the Target prompt");
                assert!(
                    !pending
                        .options
                        .iter()
                        .any(|option| option.selection == Selection::Instance { instance_id: wall.clone() }),
                    "the Immutable Wall is never offered as it resolves at its {resolution}"
                );
                assert!(
                    pending
                        .options
                        .iter()
                        .any(|option| option.selection == Selection::Hero { player: P2 })
                );
                answer_hero(&mut s);
            }
        }

        #[test]
        fn draw_reads_through_param() {
            let mut s = scenario(json!({
                "seed": "paranoia-tuned",
                "p1": { "mana": 8, "hand": [PARANOIA], "library": [VANILLA, VANILLA, VANILLA] },
                "p2": { "hand": [VANILLA], "library": [VANILLA] },
            }));
            assert_eq!(crate::upgrade_number(&mut s, PARANOIA, "draw"), 2);
            s.play(PARANOIA, json!({}));
            assert_eq!(s.hand(P1).len(), 2);
        }

        #[test]
        fn r1045_the_base_face_adds_no_echo() {
            let mut s = paranoid("paranoia-no-echo", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
            let health = s.state().players.p2.hero.health;
            let eclipse = set(&mut s, ECLIPSE);
            answer_hero(&mut s);
            reveal(&mut s, &eclipse);
            answer_hero(&mut s);
            assert!(s.state().pending.is_none(), "no Echo repeat");
            assert_eq!(s.state().players.p2.hero.health, health - 6);
        }

        #[test]
        fn r3806000_a_spell_set_under_paranoia_resolves_as_it_is_set_and_again_as_it_reveals() {
            let mut s = paranoid("paranoia-twice", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
            let health = s.state().players.p2.hero.health;
            let eclipse = set(&mut s, ECLIPSE);
            // Its target is asked as it is set (R467), and p2 is never told which Spell asks.
            assert_eq!(open_kind(&s), Some(PromptKind::Target));
            assert!(!named_to_p2(&s, ECLIPSE));
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 3);

            // It stays set: face-down in its zone, a Trap, hidden from p2, its Reveal offered.
            let still = s.backrow(P1, 2).expect("the set Spell");
            assert_eq!(still.id, eclipse);
            assert_eq!(still.face_up, None);
            assert_eq!(still.set_as.and_then(|set| set.revealing), None);
            assert_eq!(card_type_of(s.state(), &still), CardType::Trap);
            assert!(reveal_offered(&s, &eclipse));
            assert!(!named_to_p2(&s, ECLIPSE));
            let foe = serde_json::to_value(s.view(P2).opponent.backrow[1].clone()).unwrap();
            assert_eq!(foe["faceDown"], json!(true));
            assert!(foe.get("defId").is_none());
            // One Trap play: its resolution as it was set is no play (R70).
            let by_type = s
                .state()
                .players
                .p1
                .turn_log
                .played_by_type
                .clone()
                .unwrap_or_default();
            assert_eq!(by_type.get(&CardType::Trap), Some(&1), "{by_type:?}");
            assert_eq!(by_type.get(&CardType::Spell), None);

            reveal(&mut s, &eclipse);
            assert_eq!(open_kind(&s), Some(PromptKind::Target));
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 6);
            assert!(in_grave(&s, ECLIPSE).is_some());
        }

        #[test]
        fn r3806000_a_spell_set_by_a_buffed_cloaked_with_no_paranoia_out_resolves_only_as_it_reveals() {
            let mut s = buffed("paranoia-buffed", json!([ECLIPSE, VANILLA]), &[ECLIPSE]);
            let health = s.state().players.p2.hero.health;
            let eclipse = set(&mut s, ECLIPSE);
            assert!(s.state().pending.is_none(), "nothing asked as it is set");
            assert_eq!(s.state().players.p2.hero.health, health);
            reveal(&mut s, &eclipse);
            assert_eq!(open_kind(&s), Some(PromptKind::Target));
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 3);
        }

        #[test]
        fn r3806000_a_spell_that_exiles_itself_stays_set_at_its_set_and_is_exiled_as_it_reveals() {
            let mut s = paranoid("paranoia-exile", json!([TRUE_STRIKE, VANILLA]), json!([]), json!({}));
            let health = s.state().players.p2.hero.health;
            let strike = set(&mut s, TRUE_STRIKE);
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 4);
            // "Exile this" names where its reveal sends it: as it is set it stays set.
            assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(strike.clone()));
            assert!(s.state().players.p1.exile.is_empty());
            reveal(&mut s, &strike);
            answer_hero(&mut s);
            assert_eq!(s.state().players.p2.hero.health, health - 8);
            assert!(s.backrow(P1, 2).is_none());
            assert!(
                s.state()
                    .players
                    .p1
                    .exile
                    .iter()
                    .any(|card| card.def_id == TRUE_STRIKE),
                "exiled as it reveals"
            );
            assert!(in_grave(&s, TRUE_STRIKE).is_none());
        }

        #[test]
        fn r3806000_a_spell_that_shuffles_itself_into_the_deck_stays_set_at_its_set() {
            let mut s = paranoid("paranoia-shuffle", json!([CATALYST_COMET, VANILLA]), json!([]), json!({}));
            let others = |s: &Scenario| {
                s.hand(P1)
                    .iter()
                    .filter(|card| card.def_id != VANILLA && card.def_id != CATALYST_COMET)
                    .count()
            };
            let in_library = |s: &Scenario| {
                s.state()
                    .players
                    .p1
                    .library
                    .iter()
                    .filter(|card| card.def_id == CATALYST_COMET)
                    .count()
            };
            let comet = set(&mut s, CATALYST_COMET);
            assert!(s.state().pending.is_none());
            // Its Catalyst card is added as it is set, and "shuffle this into your deck" waits.
            assert_eq!(others(&s), 1);
            assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(comet.clone()));
            assert_eq!(in_library(&s), 0);
            reveal(&mut s, &comet);
            assert_eq!(others(&s), 2);
            assert!(s.backrow(P1, 2).is_none());
            assert_eq!(in_library(&s), 1, "shuffled in as it reveals");
        }

        #[test]
        fn r767000_r3806000_the_kept_v0_6_0d_script_resolves_nothing_as_a_spell_is_set() {
            crate::under_kept(PARANOIA, script_v0_6_0d(), || {
                let mut s = paranoid("paranoia-kept", json!([ECLIPSE, VANILLA]), json!([]), json!({}));
                let health = s.state().players.p2.hero.health;
                let eclipse = set(&mut s, ECLIPSE);
                assert!(s.state().pending.is_none(), "nothing asked as it is set");
                assert_eq!(s.state().players.p2.hero.health, health);
                reveal(&mut s, &eclipse);
                answer_hero(&mut s);
                assert_eq!(s.state().players.p2.hero.health, health - 3);
            });
        }
    }

    mod radiant {
        use super::*;

        fn radiant_paranoid(seed: &str, p1_field: Value) -> Scenario {
            // A Vanilla still to play, so a set never ends the turn by itself (R82).
            paranoid_as(seed, true, json!([ECLIPSE, VANILLA]), p1_field, json!({}))
        }

        #[test]
        fn r1045_r3806000_radiant_gives_a_revealed_spell_echo_plus_one_and_one_set_none() {
            let mut s = radiant_paranoid("paranoia-echo", json!([]));
            let health = s.state().players.p2.hero.health;
            let eclipse = set(&mut s, ECLIPSE);
            // As it is set, its Echo is its own, none: one resolution.
            assert_eq!(open_kind(&s), Some(PromptKind::Target));
            answer_hero(&mut s);
            assert!(s.state().pending.is_none(), "no Echo repeat as it is set");
            assert_eq!(s.state().players.p2.hero.health, health - 3);
            // As it reveals, Echo +1: two, each with its prompt.
            reveal(&mut s, &eclipse);
            assert_eq!(open_kind(&s), Some(PromptKind::Target));
            answer_hero(&mut s);
            assert_eq!(open_kind(&s), Some(PromptKind::Target));
            answer_hero(&mut s);
            assert!(s.state().pending.is_none());
            assert_eq!(s.state().players.p2.hero.health, health - 9);
        }

        #[test]
        fn r1045_the_echo_is_read_as_it_reveals_so_a_paranoia_that_left_adds_none() {
            let mut s = radiant_paranoid("paranoia-echo-left", json!([]));
            let health = s.state().players.p2.hero.health;
            let eclipse = set(&mut s, ECLIPSE);
            answer_hero(&mut s);
            paranoia_leaves(&mut s);
            reveal(&mut s, &eclipse);
            answer_hero(&mut s);
            assert!(s.state().pending.is_none(), "no Echo repeat");
            assert_eq!(s.state().players.p2.hero.health, health - 6);
        }

        #[test]
        fn r1045_two_radiant_paranoias_add_one_echo() {
            let mut s = radiant_paranoid(
                "paranoia-echo-two",
                json!([{ "def": PARANOIA, "lane": 3, "radiant": true }]),
            );
            let health = s.state().players.p2.hero.health;
            let eclipse = set(&mut s, ECLIPSE);
            answer_hero(&mut s);
            assert!(s.state().pending.is_none(), "no Echo as it is set");
            reveal(&mut s, &eclipse);
            answer_hero(&mut s);
            answer_hero(&mut s);
            assert!(s.state().pending.is_none(), "one repeat, not two");
            assert_eq!(s.state().players.p2.hero.health, health - 9);
        }

        #[test]
        fn r1045_r3806000_the_faces_print_their_aura_lines() {
            let def = crate::card_def(PARANOIA);
            assert_eq!(
                def.radiant.text,
                "Aura: Your Spells have Cloaked. Spells you set face-down resolve when set and when revealed. \
                 Spells you reveal have Echo +{echo}.\nCry: Draw {draw}."
            );
            assert_eq!(
                def.base.text,
                "Aura: Your Spells have Cloaked. Spells you set face-down resolve when set and when \
                 revealed.\nCry: Draw {draw}."
            );
        }
    }
}
