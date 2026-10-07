//! A fused card's hooks: how each ingredient's list is handed its choices, resumed after a pause and
//! ordered against what is already owed (SPEC §10.5, §10.6, R77, R90, R102, R113, R122). Found by the
//! polish-4 edge-case hunt, round 4 (docs/polish/4-edge-cases.md, lens L7); every case here failed
//! before its fix.
//!
//! Fused cards are the one place in Core where a Cry can pause with more of its list still to run,
//! and where one answered step can itself pause with a tail, so they carry most of what lens L7
//! found. Craft a Card's and #85's fusions are built directly with `subsystems.fuse`, as
//! `099-craft-a-card.test.ts` does, so the ingredients are fixed rather than a seed's Discovers.
//!
//!  - R113: a paused fused Cry resumes ingredient by ingredient. It used to rebuild every ingredient's
//!    list against the board as it now stood and skip as many effects as had run, and #22 Carnivorous
//!    Cube's half is two effects shorter once its meal has gone, so the skip swallowed the next
//!    ingredient's damage without a word.
//!  - R90, R102: each ingredient resolves the slice of the play's choices that §10.5 step 1 read, not
//!    a slice measured against the board at step 5, where the crafted card itself stands.
//!  - R113, R122: answering a prompt takes the paused step up again, so the cursor resets and a pause
//!    inside the answered step is owed ahead of everything older.
//!  - Round 6, lenses L2 and "keywords and layers". R102, R41, §8 #68: each ingredient's list is built
//!    when the combined list reaches it, so it reads the board the ingredients before it left — a
//!    Cube crafted behind a Ceaseless Void eats nothing the Void exiled, and a Sorcerer crafted
//!    behind a Reno reads the hero Reno healed. §5.2: a Fuse that adds a printed Divine Shield or
//!    Reborn gives back a shield or a Reborn the kept card had spent.
//!  - Round 7, lenses "card by card", "keywords and layers" and L2. R102: what each ingredient leaves
//!    behind is its own — an answer comes back to the Mask that asked, each Cube remembers its own
//!    meal, two Twinspells' grants and two Armors add up — and R43, R151: a Heroic Power's text #85
//!    fuses onto a kept Mana Well rolls a power, as a card created later does.
//!  - Round 8, lens "keywords and layers". R102, R124: two Going Longs' hero Armor adds up across a
//!    Fuse, and each ingredient's text reads the price its own card was played for (a Suppressive
//!    Aura paid 4 fused onto a Mana Well stays −5/−5). §8 #65.1: a radiant Spikey Pillow's aura, fused
//!    into another card, still spares every Spikey Pillow.
//!
//! Port of `packages/cards/test/fused-hooks.test.ts` (SURFACE §4.1, §8). TS's live card objects are
//! owned copies here, read back from the state by id after every step and written through
//! `find_instance_mut`; a regular expression on a fused id is the hand check `is_fused_id`.

use jackioh_cards::register_all;
use jackioh_engine::subsystems::fuse::FuseArgs;
use jackioh_engine::testkit::*;
use jackioh_engine::PlayerId::{P1, P2};

const JEWELOSCO_SCARAB: &str = "core-007";
const MR_VANILLA: &str = "core-008";
const TEMPO_TIMMY: &str = "core-011";
const MIDRANGE_MENACE: &str = "core-019";
const CARNIVOROUS_CUBE: &str = "core-022";
const RENO: &str = "core-053";
const PREJUDICED_POSTDOC: &str = "core-061";
const MASOCHISM_MASK: &str = "core-065";
const TWISTED_SORCERER: &str = "core-068";

/// `scenario(...)` over the real cards: what importing the TS harness registered (`registerAll()`).
fn scenario(setup: Value) -> Scenario {
    register_all();
    jackioh_engine::testkit::scenario(setup)
}

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected {what}"),
    }
}

/// `subsystems.fuse(sinkFor(s), args)`: TS's `sinkFor(s)` is a sink over the scenario's state with an
/// event list of its own and an rng at the state's cursor, which nothing writes back.
fn fuse_in(s: &mut Scenario, args: FuseArgs) -> Option<CardInstance> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
    let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
    subsystems::fuse::fuse(&mut sink, args)
}

/// Craft a Card's result, built directly: the ingredients in hand fused into one hand card (R77).
fn craft(s: &mut Scenario, def_ids: &[&str]) -> CardInstance {
    let ingredients: Vec<CardInstance> = def_ids
        .iter()
        .map(|def_id| {
            must(
                s.hand(P1).iter().find(|card| card.def_id == *def_id).cloned(),
                &format!("{def_id} in p1's hand"),
            )
        })
        .collect();
    must(
        fuse_in(s, FuseArgs { ingredients, to_hand: Some(P1), ..Default::default() }),
        "the crafted card",
    )
}

/// TS `g.card(x).field = …`: the live card, written through.
fn card_mut<'a>(g: &'a mut Scenario, id: &str) -> &'a mut CardInstance {
    match find_instance_mut(g.state_mut(), id) {
        Some(card) => card,
        None => panic!("{id} is in no zone"),
    }
}

/// TS `toMatch(/^t-\d+:<parts>$/)`: a fused definition's id, `t-<n>:` and then its ingredients.
fn is_fused_id(id: &str, parts: &str) -> bool {
    match id.strip_prefix("t-").and_then(|rest| rest.split_once(':')) {
        Some((number, tail)) => {
            !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) && tail == parts
        }
        None => false,
    }
}

mod r113_a_fused_cry_that_pauses_resumes_the_rest_of_every_ingredients_list {
    use super::*;

    #[test]
    fn r113_r77_r102_a_crafted_cube_scarab_sorcerer_still_deals_the_sorcerers_damage_after_the_scarabs_discover() {
        // The fused Cry is [Cube: remember, sacrifice] + [Scarab: Discover] + [Sorcerer: 4 damage]. The
        // Discover pauses it at its third effect and the fourth is owed. Rebuilt on the answer, the
        // Cube's half is empty — its meal has left the field — so the resume must go by ingredient.
        // A second permanent stays on the field, so the Cube's declaration still takes its one pick.
        let mut s = scenario(json!({
            "p1": {
                "hand": [CARNIVOROUS_CUBE, JEWELOSCO_SCARAB, TWISTED_SORCERER, RENO],
                "field": [MR_VANILLA, MIDRANGE_MENACE],
                "mana": 4,
            },
            "p2": { "hand": [RENO] },
        }));
        let crafted = craft(&mut s, &[CARNIVOROUS_CUBE, JEWELOSCO_SCARAB, TWISTED_SORCERER]);
        let meal = must(s.unit(P1, 1).cloned(), "p1's Mr. Vanilla, the Cube's meal");

        let targets = json!([
            { "pick": "instance", "instanceId": meal.id },
            { "pick": "hero", "player": "p2" },
        ]);
        s.play(&crafted.id, json!({ "zone": 3, "targets": targets }));
        // The Cube's half has eaten the meal, and the Scarab's Discover is asking.
        assert_eq!(s.card(&meal.id).zone.z(), ZoneName::Graveyard);
        let pending = must(s.state().pending.clone(), "the Scarab's Discover");
        let key = must(pending.options.first(), "a Discover option").key.clone();
        s.answer(json!(key));

        // R77: "both Cry and Death lists run" — the Sorcerer's 4 lands on the target it was given.
        assert_eq!(s.state().players.p2.hero.health, 26);
    }
}

mod r90_r102_a_fused_cards_choices_are_split_as_the_play_declared_them {
    use super::*;

    #[test]
    fn r90_r102_a_crafted_postdoc_sorcerer_played_with_no_human_on_the_field_deals_the_sorcerers_4_to_its_target() {
        // Step 1 validates the play against the board with the crafted card in hand: no Human unit is on
        // the field, so the Postdoc's declaration takes nothing (R90) and the one target is the
        // Sorcerer's. The crafted card is a Human (R102 unions the tags), and by step 5 it stands on the
        // field itself — but the play's choices were already read declaration by declaration.
        let mut s = scenario(json!({
            "p1": { "hand": [PREJUDICED_POSTDOC, TWISTED_SORCERER, RENO], "mana": 4 },
            "p2": { "hand": [RENO], "field": [MIDRANGE_MENACE] },
        }));
        let crafted = craft(&mut s, &[PREJUDICED_POSTDOC, TWISTED_SORCERER]);
        assert!(s
            .state()
            .transient_defs
            .get(&crafted.def_id)
            .is_some_and(|def| def.tags.contains(&Tag::Human)));
        let hero_only: Vec<Selection> = vec![Selection::Hero { player: P2 }];
        let offered = legal_actions(s.state(), P1).iter().any(|action| {
            matches!(action, ActionBody::Play { instance_id, targets, .. }
                if *instance_id == crafted.id && targets.as_ref() == Some(&hero_only))
        });
        assert!(offered);

        s.play(&crafted.id, json!({ "zone": 1, "targets": hero_only }));

        assert_eq!(s.state().players.p2.hero.health, 26);
    }
}

mod r113_r122_a_pause_inside_an_answered_step_is_owed_ahead_of_what_was_already_owed {
    use super::*;

    #[test]
    fn r113_r122_r102_a_fused_radiant_mask_mask_finishes_the_answered_first_picks_second_question_before_the_other_masks_first() {
        // #85 Unlicensed Experimentation fuses the Mask p2 plays onto p1's own (R77): built directly here.
        // The fused start-of-turn hook is [ask A's first, ask B's first], and R102 brings each answer back
        // to the Mask that asked, so an answered first pick is [A's pick, ask A's second] — never B's
        // pick as well. Answering the first question pauses that answered step on A's second question — a
        // pause during a resumption, which R113 owes "ahead of everything still owed", B's first question
        // and the rest of the start of turn included. Two Masks, two picks each: four questions.
        let mut s = scenario(json!({
            "p1": {
                "backrow": [{ "def": MASOCHISM_MASK, "radiant": true }],
                "hand": [MASOCHISM_MASK, RENO],
                "library": [TEMPO_TIMMY, RENO],
            },
            "p2": { "hand": [RENO], "field": [MIDRANGE_MENACE] },
        }));
        let kept = must(s.backrow(P1, 1).cloned(), "p1's radiant Mask");
        let ingredient = must(
            s.hand(P1).iter().find(|card| card.def_id == MASOCHISM_MASK).cloned(),
            "the Mask in hand",
        );
        must(
            fuse_in(&mut s, FuseArgs { ingredients: vec![ingredient], target: Some(kept), ..Default::default() }),
            "the fused Mask",
        );

        s.start_turn();
        let mut asked: Vec<&str> = Vec::new();
        for _guard in 0..20 {
            let Some(pending) = s.state().pending.as_ref() else {
                break;
            };
            asked.push(if pending.prompt.contains("(1 of 2)") { "first" } else { "second" });
            s.answer(json!("nothing"));
        }

        assert_eq!(asked, vec!["first", "second", "first", "second"]);
    }
}

const GARY: &str = "core-004";
const STOCKPILE: &str = "core-005";
const HIT_JOB: &str = "core-016";
const CEASELESS_VOID: &str = "core-100";
const CRAFT_A_CARD: &str = "core-099";
const KPOP: &str = "core-050";
const JILLIAX: &str = "core-056";
const RIGHT_HOUSE: &str = "core-003"; // Unit, 1 — 1/1 Taunt, Divine Shield, Reborn
const EXPERIMENTATION: &str = "core-085";
const KEYWORD_LIBRARY: [&str; 6] = [MR_VANILLA; 6];

fn unit_at(g: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    match g.unit(player, lane) {
        Some(card) => card.clone(),
        None => panic!("setup: {player} should hold a unit in lane {lane}"),
    }
}

fn kinds(g: &Scenario, card: &CardInstance) -> Vec<KeywordKind> {
    g.stats(&card.id).keywords.iter().map(Keyword::kind).collect()
}

/// The keyword kinds a definition's base face prints.
fn printed_kinds(g: &Scenario, def_id: &str) -> Vec<KeywordKind> {
    def_of(g.state(), def_id).base.keywords.iter().map(Keyword::kind).collect()
}

mod r41_r77_a_fused_crys_later_part_reads_the_board_its_earlier_parts_left {
    use super::*;

    #[test]
    fn r41_r102_r174_a_crafted_ceaseless_void_carnivorous_cube_whose_void_exiled_the_meal_has_eaten_nothing_so_its_death_summons_nothing() {
        let mut g = scenario(json!({
            "seed": "r6cube-2131", // Craft a Card's Discovers offer Ceaseless Void, then Carnivorous Cube (every set's Units, R380)
            "p1": { "hand": [CRAFT_A_CARD, HIT_JOB, STOCKPILE], "mana": 10, "field": [{ "def": GARY, "lane": 1 }] },
            "p2": { "hand": [STOCKPILE], "field": [{ "def": RENO, "lane": 1 }] },
        }));
        g.play(CRAFT_A_CARD, json!({}));
        g.answer(json!(CEASELESS_VOID));
        g.answer(json!(CARNIVOROUS_CUBE));
        let card = must(
            g.hand(P1).iter().find(|held| held.def_id.starts_with("t-")).cloned(),
            "the crafted card",
        );
        let gary = must(g.unit(P1, 1).cloned(), "Gary");

        // The Void's part exiles every other permanent, Gary included (§8 #100), so the Cube's part has
        // nothing to tribute: the sacrifice fizzles (R174) and nothing is eaten (R41).
        g.play(&card.id, json!({ "zone": 3, "targets": [{ "pick": "instance", "instanceId": gary.id }] }));
        g.expect_in_zone(&gary.id, "exile");
        let crafted = must(g.unit(P1, 3).cloned(), "the crafted unit");
        let before = g.events().len();

        // R41: "nothing eaten → Death does nothing". The crafted card dies, and no Gary comes back.
        g.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": crafted.id }] }));
        g.expect_in_zone(&crafted.id, "graveyard");
        let copies = g.events()[before..]
            .iter()
            .filter(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == GARY))
            .count();
        let units = g.state().players.p1.units.iter().filter(|pile| pile.is_some()).count();
        assert_eq!((copies, units), (0, 0));
    }
}

mod r77_8_68_a_fused_crys_later_part_reads_the_board_its_earlier_parts_left {
    use super::*;

    #[test]
    fn r102_8_68_a_crafted_reno_twisted_sorcerer_reads_the_hero_reno_has_just_set_to_30_so_it_deals_4_not_8() {
        let mut g = scenario(json!({
            "seed": "r6reno-266", // Craft a Card's Discovers offer Reno, then Twisted Sorcerer (every set's Units, R380)
            "p1": { "hand": [CRAFT_A_CARD, STOCKPILE], "mana": 10, "health": 5 },
            "p2": { "hand": [STOCKPILE], "field": [{ "def": RENO, "lane": 1 }] },
        }));
        g.play(CRAFT_A_CARD, json!({}));
        g.answer(json!(RENO));
        g.answer(json!(TWISTED_SORCERER));
        let card = must(
            g.hand(P1).iter().find(|held| held.def_id.starts_with("t-")).cloned(),
            "the crafted card",
        );
        let before = g.events().len();

        // Reno's part sets the hero to 30 (§8 #53), and then the Sorcerer's part resolves: "8 if your
        // hero is below 10", with the threshold "read at resolution" (§8 #68's Engine cell). The hero is
        // at 30 by then, so the enemy hero takes 4.
        g.play(&card.id, json!({ "zone": 1, "targets": [{ "pick": "hero", "player": "p2" }] }));
        g.expect_health(P1, 30);
        let hits: Vec<i32> = g.events()[before..]
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } if target_id == "hero-p2" => Some(*amount),
                _ => None,
            })
            .collect();
        assert_eq!(hits, vec![4]);
        g.expect_health(P2, 26);
    }
}

mod r77_5_2_a_keyword_a_fuse_newly_prints_applies_at_once {
    use super::*;

    #[test]
    fn r77_a_fuse_that_adds_a_printed_divine_shield_gives_a_unit_whose_granted_shield_was_spent_a_shield_again_5_2_10_1() {
        let mut g = scenario(json!({
            "active": "p2",
            "turn": 10,
            "p1": {
                "field": [{ "def": KPOP, "lane": 1 }],
                "backrow": [{ "def": EXPERIMENTATION, "lane": 3 }],
                "hand": [STOCKPILE],
                "library": KEYWORD_LIBRARY,
            },
            "p2": { "hand": [JILLIAX, STOCKPILE], "library": KEYWORD_LIBRARY },
        }));
        let kpop = unit_at(&g, P1, 1);
        // A granted Divine Shield (#63's or #80's pool, R21) that a hit has already spent (§4.4 step 1).
        card_mut(&mut g, &kpop.id).granted_keywords = vec![Keyword::DivineShield];
        card_mut(&mut g, &kpop.id).divine_shield_spent = Some(true);
        assert!(!kinds(&g, &kpop).contains(&KeywordKind::DivineShield));

        // p2 plays Jilliax (Rush, Taunt, Lifesteal, Divine Shield); #85 fuses it onto the Kpop.
        g.play(JILLIAX, json!({ "zone": 1 }));

        let fused = g.card(&kpop.id).clone();
        assert_ne!(fused.def_id, KPOP);
        assert!(g.unit(P2, 1).is_none());
        assert!(printed_kinds(&g, &fused.def_id).contains(&KeywordKind::DivineShield));
        // The fused definition prints Jilliax's Divine Shield, which K-Pop Fanatic's base face never
        // printed: a keyword the card newly gains applies at once (§5.2), exactly as radiant #50's
        // printed shield does after a granted one was spent (`radiant.gainPrintedShield`).
        assert!(kinds(&g, &kpop).contains(&KeywordKind::DivineShield));
    }

    #[test]
    fn r77_r83_a_reborn_body_fused_with_a_card_that_prints_reborn_has_reborn_again_5_2_10_1() {
        let mut g = scenario(json!({
            "active": "p2",
            "turn": 10,
            "p1": {
                "field": [{ "def": KPOP, "lane": 1 }],
                "backrow": [{ "def": EXPERIMENTATION, "lane": 3 }],
                "hand": [STOCKPILE],
                "library": KEYWORD_LIBRARY,
            },
            "p2": { "hand": [RIGHT_HOUSE, STOCKPILE], "library": KEYWORD_LIBRARY },
        }));
        let kpop = unit_at(&g, P1, 1);
        // A Kpop that came back through a granted Reborn: the body has used its Reborn (§4.5 step 4).
        card_mut(&mut g, &kpop.id).reborn_spent = Some(true);

        // p2 plays Right-house defender (Reborn printed); #85 fuses it onto the Kpop's Reborn body.
        g.play(RIGHT_HOUSE, json!({ "zone": 1 }));

        let fused = g.card(&kpop.id).clone();
        assert_ne!(fused.def_id, KPOP);
        assert!(printed_kinds(&g, &fused.def_id).contains(&KeywordKind::Reborn));
        // The defender's printed Reborn is the fused card's text, which the Kpop never printed.
        assert!(kinds(&g, &kpop).contains(&KeywordKind::Reborn));
    }
}

// ---------------------------------------------------------------------------
// Round 7 (lenses "card by card", "keywords and layers" and L2): what each ingredient's text leaves
// behind is its own (R102), and a text fused onto a kept card is had in full (R43, R151).
// ---------------------------------------------------------------------------

const MANA_WELL: &str = "core-006";
const POINTMASTER: &str = "core-020";
const SEVEN_SEVEN: &str = "core-025"; // Unit, 4 — 7/7, Armor 7
/// #45 Deft Duelist on its Radiant face: 8/6, Charge, Armor 1 — the other printed Armor in Core.
fn radiant_duelist() -> Value {
    json!({ "def": "core-045", "radiant": true })
}
const TWINSPELL: &str = "core-079";
const HEROIC_POWER: &str = "core-098";
const RAPID: &str = "core-010";

/// p2 plays its own 4-mana 7/7 into p1's armed #85, which fuses it onto p1's only Unit. `target` is a
/// def id (`json!(SEVEN_SEVEN)`) or a field entry without its lane (`radiant_duelist()`).
fn fuse_seven_seven_onto(target: Value) -> (Scenario, CardInstance) {
    let entry = match target {
        Value::String(def) => json!({ "def": def, "lane": 1 }),
        Value::Object(mut fields) => {
            fields.insert("lane".to_string(), json!(1));
            Value::Object(fields)
        }
        other => panic!("not a field entry: {other}"),
    };
    let mut g = scenario(json!({
        "active": "p2",
        "p1": {
            "backrow": [{ "def": EXPERIMENTATION, "lane": 3 }],
            "field": [entry],
            "hand": [MR_VANILLA],
            "library": KEYWORD_LIBRARY,
        },
        "p2": { "hand": [SEVEN_SEVEN, MR_VANILLA], "library": KEYWORD_LIBRARY },
    }));
    let kept = unit_at(&g, P1, 1);
    g.play(SEVEN_SEVEN, json!({ "zone": 1 }));
    // The fusion happened: the target instance stands, carrying the summed stats (R77).
    assert!(g.events().iter().any(|event| matches!(event, GameEvent::Fused { .. })));
    assert!(g.unit(P2, 1).is_none());
    (g, kept)
}

mod r102_what_a_fused_cards_ingredients_leave_behind_is_each_their_own {
    use super::*;

    #[test]
    fn r102_r77_a_fuse_of_two_armor_7_units_prints_armor_14_as_armor_7_and_armor_1_print_armor_8_6_1_armor_stacks() {
        // Two different Armors already add up on the fused face: the kept Duelist is Radiant, so the
        // fused card runs its Radiant face, the two Radiant faces summed (R77) — 8 + 14 attack.
        let (duelist_g, duelist_kept) = fuse_seven_seven_onto(radiant_duelist());
        assert_eq!(duelist_g.stats(&duelist_kept.id).attack, 22);
        assert_eq!(duelist_g.stats(&duelist_kept.id).armor, 8);

        // Two equal ones add up the same way: each ingredient prints "Armor 7", and Armor stacks from
        // every source (§6.1, §10.4), while the stats beside it sum to 14/14 (R77).
        let (twin_g, twin_kept) = fuse_seven_seven_onto(json!(SEVEN_SEVEN));
        assert_eq!(twin_g.stats(&twin_kept.id).attack, 14);
        assert_eq!(twin_g.stats(&twin_kept.id).max_health, 14);
        assert_eq!(twin_g.stats(&twin_kept.id).armor, 14);
    }

    #[test]
    fn r102_r77_a_masochism_mask_fused_onto_a_masochism_mask_applies_each_start_of_turn_pick_once_not_once_per_ingredient_8_65() {
        // p2 plays a Masochism Mask; p1's Unlicensed Experimentation fuses it onto p1's own Mask, the
        // only Field Spell p1 controls (R61, R77). The fused card carries both Masks' text: "Start of
        // turn: choose one …" twice, so p1 is asked twice and each answer is one pick.
        let mut s = scenario(json!({
            "seed": "r7-card-mask-mask",
            "active": "p2",
            "p1": {
                "backrow": [MASOCHISM_MASK, EXPERIMENTATION],
                "field": [MIDRANGE_MENACE],
                "hand": [STOCKPILE],
                "library": KEYWORD_LIBRARY,
            },
            "p2": { "hand": [MASOCHISM_MASK, STOCKPILE], "field": [MIDRANGE_MENACE], "library": KEYWORD_LIBRARY },
        }));
        s.play(MASOCHISM_MASK, json!({ "zone": 1 }));
        let fused = s.backrow(P1, 1).map(|card| card.def_id.clone());
        assert!(fused.as_deref().is_some_and(|id| is_fused_id(id, "core-065+core-065")));

        s.end_turn();
        assert_eq!(s.state().pending.as_ref().map(|pending| pending.player_id), Some(P1));
        s.answer(json!(["lose 3"]));
        // One Mask's pick: 3 health, not 3 for each ingredient that shares the step name.
        s.expect_health(P1, 27);
        assert_eq!(s.state().pending.as_ref().map(|pending| pending.player_id), Some(P1));
        s.answer(json!(["lose 3"]));
        s.expect_health(P1, 24);
    }

    #[test]
    fn r102_r41_r77_a_crafted_carnivorous_cube_carnivorous_cube_remembers_both_meals_so_its_death_copies_each_8_22() {
        let mut s = scenario(json!({
            "seed": "r7-cube-cube",
            "p1": {
                "mana": 10,
                "hand": [CARNIVOROUS_CUBE, CARNIVOROUS_CUBE, HIT_JOB, RENO],
                "field": [MIDRANGE_MENACE, POINTMASTER],
                "backrow": [MANA_WELL],
                "library": KEYWORD_LIBRARY,
            },
            "p2": { "hand": [STOCKPILE], "field": [MIDRANGE_MENACE], "library": KEYWORD_LIBRARY },
        }));
        // Craft a Card's result, built directly from the two Cubes in hand (R77).
        let cubes: Vec<CardInstance> = s.hand(P1).iter().filter(|card| card.def_id == CARNIVOROUS_CUBE).cloned().collect();
        assert_eq!(cubes.len(), 2);
        let crafted = must(
            fuse_in(&mut s, FuseArgs { ingredients: cubes, to_hand: Some(P1), ..Default::default() }),
            "the crafted card",
        );
        assert!(crafted.def_id.ends_with("core-022+core-022"));

        // Each Cube's Cry tributes one of p1's other Units (R428) and remembers it: Pointmaster for the
        // first, Midrange Menace for the second (R81, R90: one pick per declaration).
        let pointmaster = unit_at(&s, P1, 2);
        let menace = unit_at(&s, P1, 1);
        s.play(
            &crafted.id,
            json!({
                "zone": 3,
                "targets": [
                    { "pick": "instance", "instanceId": pointmaster.id },
                    { "pick": "instance", "instanceId": menace.id },
                ],
            }),
        );
        s.expect_in_zone(&pointmaster.id, "graveyard");
        s.expect_in_zone(&menace.id, "graveyard");
        // The Mana Well beside them is no meal (R428): it stays where it is.
        assert_eq!(s.backrow(P1, 1).map(|card| card.def_id.clone()), Some(MANA_WELL.to_string()));

        // Hit Job destroys the crafted card: each Cube's Death summons 2 copies of ITS remembered card.
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": crafted.id }] }));
        let units: Vec<Option<String>> = (1..=5).map(|lane| s.unit(P1, lane).map(|card| card.def_id.clone())).collect();
        assert_eq!(units.iter().filter(|id| id.as_deref() == Some(POINTMASTER)).count(), 2);
        assert_eq!(units.iter().filter(|id| id.as_deref() == Some(MIDRANGE_MENACE)).count(), 2);
    }

    #[test]
    fn r102_r77_r209_a_twinspell_fused_onto_a_twinspell_gives_the_next_spell_both_echo_1s_8_79() {
        let library: Vec<&str> = vec![MR_VANILLA; 10];
        // p2 plays a Twinspell; p1's Unlicensed Experimentation fuses it onto p1's own Twinspell (R61,
        // R77). The fused Field Spell prints "the next Spell you play gains Echo +1" twice.
        let mut s = scenario(json!({
            "seed": "r7-card-twin-twin",
            "active": "p2",
            "p1": { "backrow": [TWINSPELL, EXPERIMENTATION], "field": [MIDRANGE_MENACE], "hand": [STOCKPILE, RAPID], "library": library },
            "p2": { "hand": [TWINSPELL, STOCKPILE], "field": [MIDRANGE_MENACE], "library": library },
        }));
        s.play(TWINSPELL, json!({ "zone": 1 }));
        let fused = s.backrow(P1, 1).map(|card| card.def_id.clone());
        assert!(fused.as_deref().is_some_and(|id| is_fused_id(id, "core-079+core-079")));
        s.end_turn();

        // Two Twinspells standing apart make the next Spell resolve three times; the one card that
        // carries both texts does the same. Stockpile draws 2 per resolution.
        let from = s.events().len();
        s.play(STOCKPILE, json!({}));
        let draws = s.events()[from..].iter().filter(|event| matches!(event, GameEvent::Drawn { .. })).count();
        assert_eq!(draws, 6);
    }
}

mod r43_r151_r77_a_heroic_powers_text_fused_onto_another_permanent_has_a_power {
    use super::*;

    #[test]
    fn r151_r43_r77_unlicensed_experimentation_fusing_a_played_heroic_power_onto_a_mana_well_leaves_a_card_whose_power_can_be_activated() {
        let mut g = scenario(json!({
            "p1": { "hand": [HEROIC_POWER, STOCKPILE] },
            "p2": {
                "hand": [STOCKPILE],
                "backrow": [
                    { "def": EXPERIMENTATION, "lane": 1 },
                    { "def": MANA_WELL, "lane": 2 },
                ],
            },
        }));
        // The power the Heroic Power rolled in hand (R43): "deal 2 damage to each opposing hero", which
        // asks nothing.
        let power = must(
            g.state().players.p1.hand.iter().find(|card| card.def_id == HEROIC_POWER).cloned(),
            "the live Heroic Power",
        );
        card_mut(&mut g, &power.id).memory.insert(subsystems::POWER_KEY.to_string(), json!("burn"));
        let well = must(g.backrow(P2, 2).cloned(), "p2's Mana Well");

        // p1 plays it (which uses nothing, R752), and after it resolves p2's #85 fuses it onto the Mana Well
        // (R61, R77): the Mana Well's instance is kept and now carries the Heroic Power's text.
        g.play(&power.id, json!({ "zone": 1 }));
        let fused: CardInstance = g.card(&well.id).clone();
        assert!(fused.def_id.starts_with("t-"));
        g.expect_in_zone(&power.id, "gone");

        // R43: each power is an Activate of the card's text (R752), and "one created later rolls when it
        // is created"; R151 has a Heroic Power roll as it arrives anywhere a card can be looked at, so no
        // copy of the text is left "carrying no power … for ever". The fused card has a power, and p2
        // may use it on their own turn.
        assert!(subsystems::power_of(&fused).is_some());
        g.end_turn();
        assert_eq!(g.state().active, P2);
        let ability = must(subsystems::power_ability_of(g.state(), &fused), "the fused card's power").id.clone();
        assert!(subsystems::why_cannot_activate_ability(g.state(), P2, &fused.id, Some(ability.as_str())).is_ok());
    }
}

// ---------------------------------------------------------------------------------------------
// Round 8: a fused card's layers are each ingredient's
// ---------------------------------------------------------------------------------------------

const SUPPRESSIVE_AURA: &str = "core-046"; // Field Spell, 2 embiggen 4
const GOING_LONG: &str = "core-084"; // Field Spell, 2 embiggen 4, Quickdraw — hero Armor 2 (paid 4: 5)
const BIG_FELINOR: &str = "core-043"; // Unit, 3 — 3/10
const PILLOW: &str = "core-065-1"; // Unit token — 0/2 → 0/4, the −2 attack aura
const WEAPONS: &str = "core-014"; // Field Spell, 4 — your units +4 attack, Rush, First Strike
const LAYER_LIBRARY: [&str; 6] = [MR_VANILLA; 6];

fn backrow_at(g: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    match g.backrow(player, lane) {
        Some(card) => card.clone(),
        None => panic!("setup: {player} should hold a backrow card in lane {lane}"),
    }
}

mod r102_a_fused_cards_layers_are_each_ingredients {
    use super::*;

    #[test]
    fn r102_r124_a_going_long_fused_onto_a_going_long_gives_its_hero_armor_4_as_two_going_longs_standing_apart_do() {
        // p2 plays Going Long for 2; p1's Unlicensed Experimentation fuses it onto p1's own Going Long,
        // the only Field Spell p1 controls (R61, R77). The fused card carries both texts, "Your hero has
        // Armor 2" twice: hero Armor from several sources adds up (R124), and a static flag that is an
        // amount of what the text does adds up across a Fuse, as a Twinspell's Echo grant does.
        let mut g = scenario(json!({
            "active": "p2",
            "p1": {
                "backrow": [{ "def": GOING_LONG, "lane": 1 }, { "def": EXPERIMENTATION, "lane": 3 }],
                "hand": [STOCKPILE],
                "library": LAYER_LIBRARY,
            },
            "p2": { "hand": [GOING_LONG, STOCKPILE], "library": LAYER_LIBRARY },
        }));
        assert_eq!(hero_armor_of(g.state(), P1), 2);

        g.play(GOING_LONG, json!({ "zone": 1, "embiggen": false }));
        assert!(g.events().iter().any(|event| matches!(event, GameEvent::Fused { .. })));
        assert!(g.backrow(P2, 1).is_none());
        assert!(is_fused_id(&backrow_at(&g, P1, 1).def_id, "core-084+core-084"));

        assert_eq!(hero_armor_of(g.state(), P1), 4);
    }

    #[test]
    fn r102_r124_a_going_long_paid_4_fused_onto_a_going_long_paid_2_gives_armor_4_and_2_each_at_its_own_cards_price_6_3_embiggen() {
        let mut g = scenario(json!({
            "active": "p2",
            "p1": {
                "backrow": [{ "def": GOING_LONG, "lane": 1 }, { "def": EXPERIMENTATION, "lane": 3 }],
                "hand": [STOCKPILE],
                "library": LAYER_LIBRARY,
            },
            "p2": { "hand": [GOING_LONG, STOCKPILE], "library": LAYER_LIBRARY, "mana": 4 },
        }));
        g.play(GOING_LONG, json!({ "zone": 1, "embiggen": true }));
        assert!(is_fused_id(&backrow_at(&g, P1, 1).def_id, "core-084+core-084"));
        assert_eq!(hero_armor_of(g.state(), P1), 6);
    }

    #[test]
    fn r102_r65_a_suppressive_aura_paid_4_fused_onto_a_mana_well_keeps_its_2_2_6_3_embiggen() {
        // p2 plays Suppressive Aura at its embiggen price, 4: "all Units −2/−2". p1's Unlicensed
        // Experimentation fuses it onto p1's Mana Well. The fused cost already reads that ingredient at
        // the price it was played for (R77: the sum of the printed costs per R65), and its text is the
        // same ingredient's, which §6.3 Embiggen has read the stored choice, so the aura stays −2/−2.
        let mut g = scenario(json!({
            "active": "p2",
            "p1": {
                "backrow": [{ "def": MANA_WELL, "lane": 1 }, { "def": EXPERIMENTATION, "lane": 3 }],
                "field": [{ "def": BIG_FELINOR, "lane": 1 }],
                "hand": [STOCKPILE],
                "library": LAYER_LIBRARY,
            },
            "p2": { "hand": [SUPPRESSIVE_AURA, STOCKPILE], "library": LAYER_LIBRARY },
        }));
        let felinor = unit_at(&g, P1, 1);

        g.play(SUPPRESSIVE_AURA, json!({ "zone": 1, "embiggen": true }));
        assert!(g.events().iter().any(|event| matches!(event, GameEvent::Fused { .. })));
        assert!(is_fused_id(&backrow_at(&g, P1, 1).def_id, "core-046+core-006"));

        // Big Felinor is 3/10: under −2/−2 it is 1/8, under the base price's −1/−1 it would be 2/9.
        g.expect_stats(&felinor.id, json!({ "attack": 1, "maxHealth": 8 }));
    }

    #[test]
    fn r102_a_radiant_spikey_pillow_fused_with_another_card_still_spares_every_spikey_pillow_its_aura_names_7_8_65_1() {
        // p1's radiant Spikey Pillow prints "Aura: your non-Spikey-Pillow units have −2 attack". p2
        // plays Tempo Timmy, and p1's Unlicensed Experimentation fuses it onto the Pillow, p1's only
        // Unit (R61, R77). The fused card carries the Pillow's text in full. On p1's turn its Masochism
        // Mask summons a second, real Spikey Pillow: a Spikey Pillow, so the fused card's aura does not
        // reach it. Jlockeed's Weapons gives it +4 attack and its own base aura −2.
        let mut g = scenario(json!({
            "active": "p2",
            "p1": {
                "field": [{ "def": PILLOW, "radiant": true, "lane": 1 }],
                "backrow": [
                    { "def": MASOCHISM_MASK, "lane": 1 },
                    { "def": WEAPONS, "lane": 2 },
                    { "def": EXPERIMENTATION, "lane": 3 },
                ],
                "hand": [STOCKPILE],
                "library": LAYER_LIBRARY,
            },
            "p2": { "hand": [TEMPO_TIMMY, STOCKPILE], "library": LAYER_LIBRARY },
        }));
        let kept = unit_at(&g, P1, 1);
        g.play(TEMPO_TIMMY, json!({ "zone": 1 }));
        let fused = g.card(&kept.id).def_id.clone();
        assert!(is_fused_id(&fused, "core-011+core-065-1") || is_fused_id(&fused, "core-065-1+core-011"));

        g.end_turn();
        assert_eq!(g.state().pending.as_ref().map(|pending| pending.player_id), Some(P1));
        g.answer(json!(["summon Spikey Pillow"]));
        let pillow = unit_at(&g, P1, 2);
        assert_eq!(pillow.def_id, PILLOW);

        // 0 printed + 4 (Weapons) − 2 (its own base aura), and nothing from the fused radiant aura.
        g.expect_stats(&pillow.id, json!({ "attack": 2 }));
    }
}

mod r77_r102_a_fuse_leaves_the_kept_cards_memory_as_it_was_but_for_the_prices_its_ingredients_read_and_what_its_own_texts_remembered {
    use super::*;

    /// p2 plays Going Long, at its embiggen price or not; p1's Unlicensed Experimentation fuses it onto p1's own, played for 2.
    fn fused_going_longs(embiggen: bool) -> (Scenario, CardInstance, IndexMap<String, Value>) {
        let mut g = scenario(json!({
            "active": "p2",
            "p1": {
                "backrow": [{ "def": GOING_LONG, "lane": 1 }, { "def": EXPERIMENTATION, "lane": 3 }],
                "hand": [STOCKPILE],
                "library": LAYER_LIBRARY,
            },
            "p2": { "hand": [GOING_LONG, STOCKPILE], "library": LAYER_LIBRARY, "mana": 4 },
        }));
        let kept = backrow_at(&g, P1, 1);
        // Something the kept card remembers from before the Fuse, which R77 keeps where it is: no text
        // of the card wrote it through `remember`, so it is none of what moves with the card's texts
        // (re-entry.test.ts's R77 case has a Cube's meal move).
        card_mut(&mut g, &kept.id).memory.insert("r77-before".to_string(), json!("kept"));
        let before = g.card(&kept.id).memory.clone();
        g.play(GOING_LONG, json!({ "zone": 1, "embiggen": embiggen }));
        assert!(is_fused_id(&g.card(&kept.id).def_id, "core-084+core-084"));
        let now = g.card(&kept.id).clone();
        (g, now, before)
    }

    #[test]
    fn r77_a_fuse_whose_ingredients_were_all_played_at_the_kept_cards_price_leaves_its_memory_unchanged() {
        let (_g, kept, before) = fused_going_longs(false);
        assert_eq!(kept.memory, before);
    }

    #[test]
    fn r77_r102_a_fuse_whose_ingredients_were_played_at_different_prices_adds_only_their_prices_to_the_kept_cards_memory_6_3_embiggen() {
        let (_g, kept, before) = fused_going_longs(true);
        let mut rest = kept.memory.clone();
        let prices = rest.shift_remove(INGREDIENTS_KEY);
        // Everything the card remembered before is still there, and the one entry the Fuse added is the
        // price each ingredient was played for: the kept Going Long's 2, and p2's 4.
        assert_eq!(rest, before);
        assert!(prices.is_some());
        let mut paid: Vec<bool> = ingredients_of(&kept).unwrap_or_default().iter().map(|record| record.embiggened).collect();
        paid.sort();
        assert_eq!(paid, vec![false, true]);
    }
}
