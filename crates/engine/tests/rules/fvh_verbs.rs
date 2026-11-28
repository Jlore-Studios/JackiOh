//! Felinors vs Humans, E3 (#5238): the play, board and hand verbs, each proved through a fixture
//! script that calls it (docs/felinors-vs-humans-set.md FV2, FV3): Temporary N on an added card
//! (R5238000), the secret grant (R5238001), a play onto either side (R5238002), the crush beneath a card
//! (R5238003), enchanted shuffles (R5238004), a recruit by set (R5238005), a replaced hand that never
//! keeps a card's own def (R5238006), a discard of named library cards (R5238007), the Allure of one Unit
//! (R5238008), the death watcher (R5238009), a cheaper Transform (R5238010), Woozy gained (R5238011), a
//! forced attack on an ally (R5238012) and the swap that comes back (R5238013).
//!
//! The file registers its own catalog on each test's thread (`install`): script-less (0) Spells for the
//! scenario's filler deck and every library, then the test's own cards. Ids are `fvhv-…`.

use jackioh_engine::effects;
use jackioh_engine::prelude::*;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

/// The script-less (0) Spell every library holds, so no turn ends by itself and no draw fatigues.
const FILLER: &str = "fvhv-filler-0";
/// A vanilla Unit that plays for (0).
const PAWN: &str = "stand-in (0) 1/1";
/// A sturdier vanilla Unit for the field.
const WALL: &str = "stand-in (0) 1/5";
/// A script-less (0) Spell the verbs move about.
const KEEPSAKE: &str = "fvhv-keepsake";

/// The Temporary count the added cards carry.
const TEMPORARY_COUNT: i32 = 2;
/// How many copies the shuffles put in.
const SHUFFLED: i32 = 2;
/// The Woozy a gain adds.
const WOOZY_GAIN: i32 = 3;
/// What the death watcher's step deals the enemy hero.
const WATCH_DAMAGE: i32 = 1;
/// The bolt's hit, enough for any fixture Unit.
const BOLT_DAMAGE: i32 = 10;

/// A Core Common fixture definition of `set`, its Radiant face the same as its base.
fn def_in(
    set: &str,
    id: &str,
    index: u32,
    type_: &str,
    cost: i32,
    stats: Option<(i32, i32)>,
    tags: Vec<Tag>,
) -> CardDef {
    let face = match stats {
        Some((attack, health)) => {
            json!({ "attack": attack, "health": health, "keywords": [], "text": id })
        }
        None => json!({ "keywords": [], "text": id }),
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (FvH verbs fixture)"),
        "set": set,
        "type": type_,
        "tags": tags,
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": face,
        "radiant": face,
    }))
}

fn unit(id: &str, index: u32, cost: i32, stats: (i32, i32), tags: Vec<Tag>) -> CardDef {
    def_in("Core", id, index, "Unit", cost, Some(stats), tags)
}

fn spell(id: &str, index: u32) -> CardDef {
    def_in("Core", id, index, "Spell", 0, None, Vec::new())
}

/// This file's catalog on the calling test's thread: `DECK_SIZE` script-less (0) Spells (the filler
/// deck and the libraries), then `cards`, each with its script on both faces.
fn install(cards: Vec<(CardDef, Script)>) {
    let mut catalog: CardDefs = IndexMap::new();
    for n in 0..DECK_SIZE {
        let filler = spell(&format!("fvhv-filler-{n}"), 524_900 + n as u32);
        catalog.insert(filler.id.clone(), filler);
    }
    let mut scripts: IndexMap<String, CardScripts> = IndexMap::new();
    for (card, script) in cards {
        scripts.insert(
            card.id.clone(),
            CardScripts {
                base: script.clone(),
                radiant: script,
            },
        );
        catalog.insert(card.id.clone(), card);
    }
    register_catalog(catalog);
    register_scripts(scripts);
}

/// p1's turn 9, its main phase; each side as given, with fillers to draw, and no turn that ends by itself.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([FILLER, FILLER, FILLER, FILLER, FILLER, FILLER]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn lane_id(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit_at(player, lane).id
}

fn hero_health(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player].hero.health
}

/// The ids of `player`'s pile (`hand`, `library`, `graveyard`) of `def_id`.
fn pile_of(s: &Scenario, player: PlayerId, zone: &str, def_id: &str) -> Vec<String> {
    s.pile(player, zone)
        .into_iter()
        .filter(|card| card.def_id == def_id)
        .map(|card| card.id)
        .collect()
}

fn hand_holds(s: &Scenario, player: PlayerId, id: &str) -> bool {
    s.hand(player).iter().any(|card| card.id == id)
}

/// The instance ids a play chose, in order.
fn chosen_ids(ctx: &EffectContext<'_>) -> Vec<String> {
    ctx.targets
        .iter()
        .filter_map(|selection| match selection {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

/// A Spell whose Cry is `effect`, made fresh at each play.
fn cast(effect: impl Fn() -> Effect + Send + Sync + 'static) -> Script {
    Script {
        cry: Some(hook(move |_ctx| vec![effect()])),
        ..Script::default()
    }
}

/// A Spell aimed at `count` Units of either side, its Cry built from what it chose.
fn aimed(count: i32, effect: impl Fn(&EffectContext<'_>) -> Effect + Send + Sync + 'static) -> Script {
    Script {
        targets: vec![TargetDecl::target(count, count, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(move |ctx| vec![effect(ctx)])),
        ..Script::default()
    }
}

/// Two picks, in order, as a play's targets.
fn two_picks(first: &str, second: &str) -> Value {
    json!({ "targets": [pick::id(first)[0], pick::id(second)[0]] })
}

mod r5238000_temporary_count {
    use super::*;

    const ADD_TEMPORARY: &str = "fvhv-add-temporary";
    const ADD_RANDOM_TEMPORARY: &str = "fvhv-add-random-temporary";
    const ADD_PLAIN: &str = "fvhv-add-plain";

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (spell(KEEPSAKE, 523_800), Script::default()),
            (
                spell(ADD_TEMPORARY, 523_801),
                cast(|| {
                    let mut args: AddToHandArgs = json_as(json!({ "defId": KEEPSAKE }));
                    args.temporary_count = Some(TEMPORARY_COUNT);
                    effects::add_to_hand(args)
                }),
            ),
            (
                spell(ADD_RANDOM_TEMPORARY, 523_802),
                cast(|| {
                    let mut args: AddRandomFromCatalogArgs =
                        json_as(json!({ "query": { "defId": [KEEPSAKE] } }));
                    args.temporary_count = Some(TEMPORARY_COUNT);
                    effects::add_random_from_catalog(args)
                }),
            ),
            (
                spell(ADD_PLAIN, 523_803),
                cast(|| effects::add_to_hand(json_as(json!({ "defId": KEEPSAKE })))),
            ),
        ]
    }

    /// Plays `adder` and follows the card it added through p1's ends of turn: Temporary 2 outlasts the
    /// first and is discarded at the second.
    fn outlasts_one_end(seed: &str, adder: &str) {
        install(cards());
        let mut s = board(
            seed,
            json!({ "hand": [adder, FILLER] }),
            json!({ "hand": [FILLER] }),
        );
        s.play(adder, json!({}));
        let added = must(pile_of(&s, P1, "hand", KEEPSAKE).pop(), "the added card");
        assert!(is_temporary_card(s.state(), s.card(added.as_str())));
        s.end_turn();
        assert!(hand_holds(&s, P1, &added), "Temporary 2 outlasts its owner's first end of turn");
        s.end_turn();
        assert!(hand_holds(&s, P1, &added), "the opponent's end of turn counts nothing");
        s.end_turn();
        s.expect_in_zone(added.as_str(), "graveyard");
    }

    #[test]
    fn r5238000_add_to_hand_adds_the_card_with_temporary_n() {
        outlasts_one_end("r5238000-add", ADD_TEMPORARY);
    }

    #[test]
    fn r5238000_add_random_from_catalog_adds_the_card_with_temporary_n() {
        outlasts_one_end("r5238000-random", ADD_RANDOM_TEMPORARY);
    }

    #[test]
    fn r5238000_absent_the_card_is_not_temporary() {
        install(cards());
        let mut s = board(
            "r5238000-plain",
            json!({ "hand": [ADD_PLAIN, FILLER] }),
            json!({ "hand": [FILLER] }),
        );
        s.play(ADD_PLAIN, json!({}));
        let added = must(pile_of(&s, P1, "hand", KEEPSAKE).pop(), "the added card");
        assert!(!is_temporary_card(s.state(), s.card(added.as_str())));
        s.end_turn();
        assert!(hand_holds(&s, P1, &added));
    }
}

mod r5238001_secret_grant {
    use super::*;

    /// "Choose two of your Units: the first gains Taunt in secret, the second is its decoy."
    const SECRET: &str = "fvhv-secret-taunt";

    fn cards() -> Vec<(CardDef, Script)> {
        vec![(
            spell(SECRET, 523_810),
            aimed(2, |ctx| {
                let decoys: Vec<String> = chosen_ids(ctx).into_iter().skip(1).collect();
                effects::secret_grant::grant_keyword_secretly(SecretGrantArgs {
                    target: TargetSpec::Chosen { index: Some(0) },
                    decoys,
                    keyword: Keyword::Taunt,
                })
            }),
        )]
    }

    /// The keywords `viewer`'s view shows on the Unit `id`, on either side.
    fn shown(s: &Scenario, viewer: PlayerId, id: &str) -> Vec<KeywordKind> {
        let view = view_for(s.state(), viewer);
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

    fn granted(seed: &str) -> (Scenario, String, String) {
        install(cards());
        let mut s = board(
            seed,
            json!({ "hand": [SECRET], "field": [WALL, WALL] }),
            json!({ "hand": [FILLER] }),
        );
        let (target, decoy) = (lane_id(&s, P1, 1), lane_id(&s, P1, 2));
        s.play(SECRET, two_picks(&target, &decoy));
        (s, target, decoy)
    }

    #[test]
    fn r5238001_the_target_has_it_and_the_other_seat_sees_it_on_the_target_and_every_decoy() {
        let (s, target, decoy) = granted("r5238001-shown");
        assert!(unit_has(s.state(), s.card(target.as_str()), KeywordKind::Taunt));
        assert!(shown(&s, P2, &target).contains(&KeywordKind::Taunt));
        assert!(shown(&s, P2, &decoy).contains(&KeywordKind::Taunt), "the decoy shows it too");
        assert!(shown(&s, P1, &target).contains(&KeywordKind::Taunt));
    }

    #[test]
    fn r5238001_a_decoy_never_has_it_and_its_controller_sees_the_truth() {
        let (s, _target, decoy) = granted("r5238001-truth");
        assert!(!unit_has(s.state(), s.card(decoy.as_str()), KeywordKind::Taunt));
        assert!(!shown(&s, P1, &decoy).contains(&KeywordKind::Taunt));
    }
}

mod r5238002_plays_on_either_side {
    use super::*;

    const EITHER: &str = "fvhv-either-side";

    fn cards() -> Vec<(CardDef, Script)> {
        vec![(
            unit(EITHER, 523_820, 0, (2, 2), Vec::new()),
            Script {
                static_flags: Some(StaticFlags {
                    plays_on_either_side: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            },
        )]
    }

    /// p1's play of the hand card `id` into `side`'s unit zone in `lane`.
    fn play_onto<'a>(s: &'a mut Scenario, id: &str, side: PlayerId, lane: i32) -> &'a mut Scenario {
        let mut body: ActionBody = json_as(json!({ "type": "play", "instanceId": id }));
        if let ActionBody::Play { zone, .. } = &mut body {
            *zone = Some(ZoneChoice {
                row: Row::Units,
                lane,
                player: Some(side),
            });
        }
        s.act(body, P1, "a play onto a chosen side")
    }

    /// The side each offered play of the card names for its zone.
    fn sides_offered(s: &Scenario, id: &str) -> Vec<Option<PlayerId>> {
        s.plays_of(id)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { zone: Some(zone), .. } => Some(zone.player),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r5238002_it_may_be_played_into_the_opponents_open_zone() {
        install(cards());
        let mut s = board("r5238002-theirs", json!({ "hand": [EITHER] }), json!({}));
        let either = s.card(EITHER).id.clone();
        play_onto(&mut s, &either, P2, 3);
        assert_eq!(s.unit_at(P2, 3).id, either, "it stands on the opponent's side");
        assert!(s.unit(P1, 3).is_none());
    }

    #[test]
    fn r5238002_it_may_be_played_into_its_own_side_too_and_both_sides_are_offered() {
        install(cards());
        let mut s = board("r5238002-own", json!({ "hand": [EITHER] }), json!({}));
        let either = s.card(EITHER).id.clone();
        let sides = sides_offered(&s, &either);
        assert!(sides.contains(&Some(P2)), "the opponent's zones are offered: {sides:?}");
        assert!(
            sides.iter().any(|side| *side != Some(P2)),
            "and its own: {sides:?}"
        );
        play_onto(&mut s, &either, P1, 2);
        assert_eq!(s.unit_at(P1, 2).id, either);
    }

    #[test]
    fn r5238002_any_other_card_is_refused_an_enemy_zone() {
        install(cards());
        let mut s = board("r5238002-plain", json!({ "hand": [PAWN] }), json!({}));
        let pawn = s.card(PAWN).id.clone();
        s.expect_refused(|s| play_onto(s, &pawn, P2, 3));
        assert!(s.unit(P2, 3).is_none());
        s.expect_in_zone(pawn.as_str(), "hand");
    }

    #[test]
    fn r5238002_any_other_card_is_offered_no_enemy_zone() {
        install(cards());
        let s = board("r5238002-offers", json!({ "hand": [PAWN] }), json!({}));
        let pawn = s.card(PAWN).id.clone();
        let sides = sides_offered(&s, &pawn);
        assert!(!sides.is_empty(), "its own zones are offered");
        assert!(!sides.contains(&Some(P2)), "{sides:?}");
    }
}

mod r5238003_crush_beneath_only {
    use super::*;

    /// "End of turn: crush the cards beneath this", the top of a pile in lane 2.
    const CRUSH_BENEATH: &str = "fvhv-crush-beneath";
    /// The same crush of its lane with no `beneath_only`.
    const CRUSH_LANE: &str = "fvhv-crush-lane";
    const LANE: i32 = 2;

    fn crusher(id: &'static str, index: u32, beneath: bool) -> (CardDef, Script) {
        let script = Script {
            end_of_turn: Some(hook(move |_ctx| {
                let mut args: CrushArgs = json_as(json!({ "player": "self", "lane": LANE }));
                args.beneath_only = beneath.then_some(true);
                vec![effects::crush(args)]
            })),
            ..Script::default()
        };
        (unit(id, index, 0, (2, 2), Vec::new()), script)
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            crusher(CRUSH_BENEATH, 523_830, true),
            crusher(CRUSH_LANE, 523_831, false),
        ]
    }

    /// p1's pile in `LANE`: a vanilla Unit beneath `top`. Ends p1's turn; returns (beneath, top).
    fn crushed(seed: &str, top: &str) -> (Scenario, String, String) {
        install(cards());
        let mut s = board(
            seed,
            json!({
                "hand": [FILLER],
                "field": [{ "def": WALL, "lane": LANE }, { "def": top, "lane": LANE, "stack": true }],
            }),
            json!({ "hand": [FILLER] }),
        );
        let top_id = s.card(top).id.clone();
        let beneath = s.card(WALL).id.clone();
        s.end_turn();
        (s, beneath, top_id)
    }

    #[test]
    fn r5238003_destroys_only_the_cards_beneath_the_running_card() {
        let (mut s, beneath, top) = crushed("r5238003-beneath", CRUSH_BENEATH);
        s.expect_in_zone(beneath.as_str(), "graveyard");
        s.expect_in_zone(top.as_str(), "field");
    }

    #[test]
    fn r5238003_absent_the_crush_destroys_the_whole_zone() {
        let (mut s, beneath, top) = crushed("r5238003-lane", CRUSH_LANE);
        s.expect_in_zone(beneath.as_str(), "graveyard");
        s.expect_in_zone(top.as_str(), "graveyard");
    }
}

mod r5238004_shuffle_enchanted {
    use super::*;

    const SHUFFLE_ENCHANTED: &str = "fvhv-shuffle-cast-on-draw";
    const SHUFFLE_PLAIN: &str = "fvhv-shuffle-plain";

    fn shuffler(enchanted: bool) -> Script {
        cast(move || {
            let mut args: ShuffleIntoArgs = json_as(json!({ "defId": KEEPSAKE, "count": SHUFFLED }));
            if enchanted {
                args.enchantments = Some(vec![Enchantment::CastOnDraw]);
            }
            effects::shuffle_into(args)
        })
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (spell(KEEPSAKE, 523_840), Script::default()),
            (spell(SHUFFLE_ENCHANTED, 523_841), shuffler(true)),
            (spell(SHUFFLE_PLAIN, 523_842), shuffler(false)),
        ]
    }

    fn casts_on_draw(card: &CardInstance) -> bool {
        card.enchantments
            .as_ref()
            .is_some_and(|list| list.contains(&Enchantment::CastOnDraw))
    }

    fn shuffled(seed: &str, shuffler: &str) -> Scenario {
        install(cards());
        let mut s = board(seed, json!({ "hand": [shuffler] }), json!({}));
        s.play(shuffler, json!({}));
        s
    }

    #[test]
    fn r5238004_each_shuffled_card_carries_the_enchantments() {
        let s = shuffled("r5238004-enchanted", SHUFFLE_ENCHANTED);
        let library = s.pile(P1, "library");
        let fresh: Vec<&CardInstance> = library.iter().filter(|card| card.def_id == KEEPSAKE).collect();
        assert_eq!(fresh.len(), SHUFFLED as usize);
        assert!(fresh.iter().all(|card| casts_on_draw(card)));
    }

    #[test]
    fn r5238004_the_cards_already_there_carry_nothing() {
        let s = shuffled("r5238004-others", SHUFFLE_ENCHANTED);
        let library = s.pile(P1, "library");
        let others: Vec<&CardInstance> = library.iter().filter(|card| card.def_id == FILLER).collect();
        assert!(!others.is_empty());
        assert!(others.iter().all(|card| !casts_on_draw(card)));
    }

    #[test]
    fn r5238004_absent_the_shuffled_cards_carry_nothing() {
        let s = shuffled("r5238004-plain", SHUFFLE_PLAIN);
        let library = s.pile(P1, "library");
        let fresh: Vec<&CardInstance> = library.iter().filter(|card| card.def_id == KEEPSAKE).collect();
        assert_eq!(fresh.len(), SHUFFLED as usize);
        assert!(fresh.iter().all(|card| !casts_on_draw(card)));
    }
}

mod r5238005_recruit_by_set {
    use super::*;

    const CORE_UNIT: &str = "fvhv-core-unit";
    const CLASSIC_UNIT: &str = "fvhv-classic-unit";
    const RECRUIT_CLASSIC: &str = "fvhv-recruit-classic";
    const RECRUIT_MEDITATIVE: &str = "fvhv-recruit-meditative";

    fn recruiter(set: SetName) -> Script {
        cast(move || {
            let mut filter: RecruitFilter = json_as(json!({}));
            filter.set = Some(OneOrMany::One(set));
            let mut args: RecruitArgs = json_as(json!({}));
            args.filter = Some(filter);
            effects::recruit(args)
        })
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (unit(CORE_UNIT, 523_850, 1, (1, 1), Vec::new()), Script::default()),
            (
                def_in("Classic", CLASSIC_UNIT, 523_851, "Unit", 1, Some((1, 1)), Vec::new()),
                Script::default(),
            ),
            (spell(RECRUIT_CLASSIC, 523_852), recruiter(SetName::Classic)),
            (spell(RECRUIT_MEDITATIVE, 523_853), recruiter(SetName::Meditative)),
        ]
    }

    fn recruiting(seed: &str, recruiter: &str) -> Scenario {
        install(cards());
        let mut s = board(
            seed,
            json!({ "hand": [recruiter], "library": [CORE_UNIT, CLASSIC_UNIT, FILLER, FILLER] }),
            json!({}),
        );
        s.play(recruiter, json!({}));
        s
    }

    #[test]
    fn r5238005_recruits_only_a_card_of_the_named_set() {
        let s = recruiting("r5238005-classic", RECRUIT_CLASSIC);
        assert_eq!(s.unit_at(P1, 1).def_id, CLASSIC_UNIT);
        assert_eq!(pile_of(&s, P1, "library", CORE_UNIT).len(), 1, "the Core Unit stays");
        assert!(pile_of(&s, P1, "library", CLASSIC_UNIT).is_empty());
    }

    #[test]
    fn r5238005_a_set_no_library_card_is_of_recruits_nothing() {
        let s = recruiting("r5238005-none", RECRUIT_MEDITATIVE);
        assert!(s.unit(P1, 1).is_none());
        assert_eq!(pile_of(&s, P1, "library", CORE_UNIT).len(), 1);
        assert_eq!(pile_of(&s, P1, "library", CLASSIC_UNIT).len(), 1);
    }
}

mod r5238006_exclude_replaced {
    use super::*;

    const APPLE: &str = "fvhv-apple";
    const BANANA: &str = "fvhv-banana";
    const CHERRY: &str = "fvhv-cherry";
    const REPLACE_THREE: &str = "fvhv-replace-three";
    const REPLACE_TWO: &str = "fvhv-replace-two";

    fn replacer(pool: &'static [&'static str]) -> Script {
        cast(move || {
            let mut args: ReplaceHandWithRandomArgs = json_as(json!({ "query": { "defId": pool } }));
            args.exclude_replaced = Some(true);
            effects::replace_hand_with_random(args)
        })
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (spell(APPLE, 523_860), Script::default()),
            (spell(BANANA, 523_861), Script::default()),
            (spell(CHERRY, 523_862), Script::default()),
            (spell(REPLACE_THREE, 523_863), replacer(&[APPLE, BANANA, CHERRY])),
            (spell(REPLACE_TWO, 523_864), replacer(&[APPLE, BANANA])),
        ]
    }

    fn hand_defs(s: &Scenario) -> Vec<String> {
        s.hand(P1).into_iter().map(|card| card.def_id).collect()
    }

    #[test]
    fn r5238006_a_card_whose_own_def_is_the_only_other_choice_takes_the_other() {
        install(cards());
        let mut s = board("r5238006-two", json!({ "hand": [REPLACE_TWO, APPLE] }), json!({}));
        s.play(REPLACE_TWO, json!({}));
        assert_eq!(hand_defs(&s), vec![BANANA.to_string()]);
    }

    #[test]
    fn r5238006_never_replaces_a_hand_card_with_its_own_def() {
        install(cards());
        let mut s = board(
            "r5238006-three",
            json!({ "hand": [REPLACE_THREE, APPLE, APPLE] }),
            json!({}),
        );
        s.play(REPLACE_THREE, json!({}));
        let defs = hand_defs(&s);
        assert_eq!(defs.len(), 2, "each card is replaced: {defs:?}");
        assert!(!defs.contains(&APPLE.to_string()), "{defs:?}");
        assert!(
            defs.iter().all(|def| def == BANANA || def == CHERRY),
            "{defs:?}"
        );
    }
}

mod r5238007_discard_named_library_cards {
    use super::*;

    const MARKED: &str = "fvhv-marked";
    /// Discards p1's library cards of MARKED by id.
    const DISCARD_MARKED: &str = "fvhv-discard-marked";
    /// Names p1's hand cards of MARKED instead.
    const DISCARD_HAND_IDS: &str = "fvhv-discard-hand-ids";
    const DISCARD_ALL: &str = "fvhv-discard-all";

    fn discarder(from_hand: bool) -> Script {
        Script {
            cry: Some(hook(move |ctx| {
                let side = &ctx.state.players[ctx.controller];
                let pile = if from_hand { &side.hand } else { &side.library };
                let named: Vec<String> = pile
                    .iter()
                    .filter(|card| card.def_id == MARKED)
                    .map(|card| card.id.clone())
                    .collect();
                let mut args: DiscardLibraryArgs = json_as(json!({}));
                args.instance_ids = Some(named);
                vec![effects::discard_library(args)]
            })),
            ..Script::default()
        }
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (spell(MARKED, 523_870), Script::default()),
            (spell(DISCARD_MARKED, 523_871), discarder(false)),
            (spell(DISCARD_HAND_IDS, 523_872), discarder(true)),
            (
                spell(DISCARD_ALL, 523_873),
                cast(|| effects::discard_library(json_as(json!({})))),
            ),
        ]
    }

    fn library() -> Value {
        json!([MARKED, FILLER, MARKED, FILLER])
    }

    #[test]
    fn r5238007_discards_only_the_named_library_cards() {
        install(cards());
        let mut s = board(
            "r5238007-named",
            json!({ "hand": [DISCARD_MARKED], "library": library() }),
            json!({}),
        );
        let marked = pile_of(&s, P1, "library", MARKED);
        s.play(DISCARD_MARKED, json!({}));
        for id in nonempty(marked, "the marked library cards") {
            s.expect_in_zone(id.as_str(), "graveyard");
        }
        assert_eq!(pile_of(&s, P1, "library", FILLER).len(), 2, "the rest stay");
    }

    #[test]
    fn r5238007_absent_the_whole_library_is_discarded() {
        install(cards());
        let mut s = board(
            "r5238007-all",
            json!({ "hand": [DISCARD_ALL], "library": library() }),
            json!({}),
        );
        s.play(DISCARD_ALL, json!({}));
        assert!(s.pile(P1, "library").is_empty());
    }

    #[test]
    fn r5238007_an_id_of_no_library_card_discards_nothing() {
        install(cards());
        let mut s = board(
            "r5238007-hand",
            json!({ "hand": [DISCARD_HAND_IDS, MARKED], "library": library() }),
            json!({}),
        );
        let in_hand = pile_of(&s, P1, "hand", MARKED);
        s.play(DISCARD_HAND_IDS, json!({}));
        for id in nonempty(in_hand, "the hand's marked card") {
            assert!(hand_holds(&s, P1, &id), "a hand card is no library card");
        }
        assert_eq!(s.pile(P1, "library").len(), 4, "the library is untouched");
    }
}

mod r5238008_allure_unit {
    use super::*;

    const ALLURE_NAMED: &str = "fvhv-allure-named";
    const ALLURE_RANDOM: &str = "fvhv-allure-random";

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (
                spell(ALLURE_NAMED, 523_880),
                aimed(1, |ctx| {
                    effects::delay::allure_unit(AllureUnitArgs {
                        instance_id: chosen_ids(ctx).into_iter().next(),
                        random_enemy: None,
                    })
                }),
            ),
            (
                spell(ALLURE_RANDOM, 523_881),
                cast(|| {
                    effects::delay::allure_unit(AllureUnitArgs {
                        instance_id: None,
                        random_enemy: Some(true),
                    })
                }),
            ),
        ]
    }

    /// p1 plays `allure` (`targets` as given), then the turn comes round to p1's next start.
    fn allured(seed: &str, allure: &str, targets: impl Fn(&Scenario) -> Value) -> (Scenario, String, String) {
        install(cards());
        let mut s = board(
            seed,
            json!({ "hand": [allure, FILLER] }),
            json!({ "hand": [FILLER], "field": [PAWN, WALL] }),
        );
        let (pawn, wall) = (lane_id(&s, P2, 1), lane_id(&s, P2, 2));
        let picks = targets(&s);
        s.play(allure, picks);
        s.end_turn().end_turn();
        (s, pawn, wall)
    }

    #[test]
    fn r5238008_allures_the_named_unit() {
        let (s, _pawn, wall) = allured("r5238008-named", ALLURE_NAMED, |s| {
            pick::targets(pick::id(&s.unit_at(P2, 2).id))
        });
        assert_eq!(s.card(wall.as_str()).controller, P1, "stolen at the Allurer's next start");
    }

    #[test]
    fn r5238008_no_other_unit_is_allured_by_a_named_allure() {
        let (s, pawn, _wall) = allured("r5238008-other", ALLURE_NAMED, |s| {
            pick::targets(pick::id(&s.unit_at(P2, 2).id))
        });
        assert_eq!(s.card(pawn.as_str()).controller, P2);
    }

    #[test]
    fn r5238008_random_enemy_allures_exactly_one_enemy_unit() {
        let (s, pawn, wall) = allured("r5238008-random", ALLURE_RANDOM, |_s| json!({}));
        let stolen = [pawn, wall]
            .iter()
            .filter(|id| s.card(id.as_str()).controller == P1)
            .count();
        assert_eq!(stolen, 1);
    }
}

mod r5238009_watch_deaths {
    use super::*;

    const HUMAN: &str = "fvhv-human";
    const WATCH_HUMANS: &str = "fvhv-watch-human-deaths";
    const BOLT: &str = "fvhv-bolt";
    const DIED_STEP: &str = "died";

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (unit(HUMAN, 523_890, 0, (1, 1), vec![Tag::Human]), Script::default()),
            (
                spell(WATCH_HUMANS, 523_891),
                Script {
                    cry: Some(hook(|_ctx| {
                        vec![effects::delay::watch_deaths(WatchDeathsArgs {
                            tag: Tag::Human,
                            step: DIED_STEP.to_string(),
                            label: "fvh death watch".to_string(),
                            hook: None,
                            data: None,
                        })]
                    })),
                    resume: IndexMap::from([(
                        DIED_STEP,
                        hook(|_ctx| {
                            vec![effects::damage(json_as(
                                json!({ "to": { "of": "enemyHero" }, "amount": WATCH_DAMAGE }),
                            ))]
                        }),
                    )]),
                    ..Script::default()
                },
            ),
            (
                spell(BOLT, 523_892),
                aimed(1, |_ctx| {
                    effects::damage(json_as(json!({ "to": { "of": "chosen" }, "amount": BOLT_DAMAGE })))
                }),
            ),
        ]
    }

    #[test]
    fn r5238009_fires_after_each_death_of_a_card_with_the_tag_for_the_rest_of_the_game() {
        install(cards());
        let mut s = board(
            "r5238009-each",
            json!({ "hand": [WATCH_HUMANS, BOLT, BOLT], "field": [HUMAN, HUMAN] }),
            json!({ "hand": [FILLER] }),
        );
        let (first, second) = (lane_id(&s, P1, 1), lane_id(&s, P1, 2));
        s.play(WATCH_HUMANS, json!({}));
        let health = hero_health(&s, P2);
        s.play(BOLT, pick::targets(pick::id(&first)));
        s.expect_in_zone(first.as_str(), "graveyard");
        assert_eq!(hero_health(&s, P2), health - WATCH_DAMAGE);
        s.end_turn().end_turn();
        s.play(BOLT, pick::targets(pick::id(&second)));
        assert_eq!(hero_health(&s, P2), health - 2 * WATCH_DAMAGE, "a turn later it still fires");
    }

    #[test]
    fn r5238009_the_death_of_a_card_without_the_tag_fires_nothing() {
        install(cards());
        let mut s = board(
            "r5238009-untagged",
            json!({ "hand": [WATCH_HUMANS, BOLT], "field": [PAWN] }),
            json!({}),
        );
        let pawn = lane_id(&s, P1, 1);
        s.play(WATCH_HUMANS, json!({}));
        let health = hero_health(&s, P2);
        s.play(BOLT, pick::targets(pick::id(&pawn)));
        s.expect_in_zone(pawn.as_str(), "graveyard");
        assert_eq!(hero_health(&s, P2), health);
    }
}

mod r5238010_cheaper_transform {
    use super::*;

    const ONE: &str = "fvhv-unit-1";
    const THREE: &str = "fvhv-unit-3";
    const EIGHT: &str = "fvhv-unit-8";
    const DOWN_ONE: &str = "fvhv-devolve-1";
    const DOWN_TWO: &str = "fvhv-devolve-2";
    const UP_ONE: &str = "fvhv-evolve-1";

    fn stepper(step: i32) -> Script {
        aimed(1, move |_ctx| {
            effects::transform_cost_step(json_as(json!({ "target": { "of": "chosen" }, "step": step })))
        })
    }

    /// The only Unit pool: one Unit each at (1), (3) and (8), a hole from (4) to (7).
    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (unit(ONE, 523_900, 1, (1, 1), Vec::new()), Script::default()),
            (unit(THREE, 523_901, 3, (3, 3), Vec::new()), Script::default()),
            (unit(EIGHT, 523_902, 8, (8, 8), Vec::new()), Script::default()),
            (spell(DOWN_ONE, 523_903), stepper(-1)),
            (spell(DOWN_TWO, 523_904), stepper(-2)),
            (spell(UP_ONE, 523_905), stepper(1)),
        ]
    }

    /// p1 plays `stepper` at p2's lane-1 Unit of `from`; the def standing there after.
    fn stepped(seed: &str, stepper: &str, from: &str) -> String {
        install(cards());
        let mut s = board(seed, json!({ "hand": [stepper] }), json!({ "field": [from] }));
        let target = lane_id(&s, P2, 1);
        s.play(stepper, pick::targets(pick::id(&target)));
        s.unit_at(P2, 1).def_id
    }

    #[test]
    fn r5238010_a_negative_step_takes_the_highest_price_at_or_below_start_plus_step() {
        assert_eq!(
            stepped("r5238010-hole", DOWN_ONE, EIGHT),
            THREE,
            "(7) has none: the highest below is (3)"
        );
        assert_eq!(stepped("r5238010-two", DOWN_TWO, EIGHT), THREE);
        assert_eq!(stepped("r5238010-exact", DOWN_ONE, THREE), ONE);
    }

    #[test]
    fn r5238010_with_no_price_at_or_below_the_card_stays() {
        assert_eq!(stepped("r5238010-floor", DOWN_ONE, ONE), ONE);
    }

    #[test]
    fn r5238010_a_positive_step_still_takes_the_closest_higher_price() {
        assert_eq!(stepped("r5238010-up", UP_ONE, ONE), THREE);
    }
}

mod r5238011_gain_woozy {
    use super::*;

    const WOOZY: &str = "fvhv-woozy";
    const WOOZY_NONE: &str = "fvhv-woozy-none";
    const WOOZY_THEIRS: &str = "fvhv-woozy-theirs";

    fn gainer(amount: i32, player: Option<PlayerSpec>) -> Script {
        cast(move || effects::player_mods::gain_woozy(GainWoozyArgs { player, amount }))
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (spell(WOOZY, 523_910), gainer(WOOZY_GAIN, None)),
            (spell(WOOZY_NONE, 523_911), gainer(0, None)),
            (
                spell(WOOZY_THEIRS, 523_912),
                gainer(WOOZY_GAIN, Some(PlayerSpec::Enemy)),
            ),
        ]
    }

    fn woozy_of(s: &Scenario, player: PlayerId) -> Option<i32> {
        s.state().players[player].woozy.map(|counter| counter.count)
    }

    fn woozy_events(s: &Scenario) -> Vec<(String, i32)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CounterChanged {
                    instance_id,
                    counter: CounterKind::Woozy,
                    value,
                    ..
                } => Some((instance_id.clone(), *value)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r5238011_raises_the_players_woozy_counter() {
        install(cards());
        let mut s = board("r5238011-raise", json!({ "hand": [WOOZY, WOOZY] }), json!({}));
        s.play(WOOZY, json!({}));
        assert_eq!(woozy_of(&s, P1), Some(WOOZY_GAIN));
        assert_eq!(woozy_events(&s), vec![("hero-p1".to_string(), WOOZY_GAIN)]);
        s.play(WOOZY, json!({}));
        assert_eq!(woozy_of(&s, P1), Some(2 * WOOZY_GAIN), "a second gain adds to it");
    }

    #[test]
    fn r5238011_a_gain_of_0_changes_nothing() {
        install(cards());
        let mut s = board("r5238011-zero", json!({ "hand": [WOOZY_NONE] }), json!({}));
        s.play(WOOZY_NONE, json!({}));
        assert_eq!(woozy_of(&s, P1), None);
        assert!(woozy_events(&s).is_empty());
    }

    #[test]
    fn r5238011_the_enemy_option_raises_the_opponents_counter_alone() {
        install(cards());
        let mut s = board("r5238011-theirs", json!({ "hand": [WOOZY_THEIRS] }), json!({}));
        s.play(WOOZY_THEIRS, json!({}));
        assert_eq!(woozy_of(&s, P2), Some(WOOZY_GAIN));
        assert_eq!(woozy_of(&s, P1), None);
    }
}

mod r5238012_forced_attack_on_an_ally {
    use super::*;

    const FORCE_ALLY: &str = "fvhv-force-ally";
    const FORCE_PLAIN: &str = "fvhv-force-plain";
    /// The attacker: 3/9.
    const STRIKER: &str = "stand-in (0) 3/9";
    /// The ally it attacks: 2/9.
    const ALLY: &str = "stand-in (0) 2/9";

    /// "Choose two Units: the first attacks the second", `allies` when `allies`.
    fn forcer(allies: bool) -> Script {
        aimed(2, move |_ctx| {
            let mut args: ForcedAttackArgs = json_as(json!({
                "attacker": { "of": "chosen", "index": 0 },
                "target": { "of": "chosen", "index": 1 },
            }));
            args.allies = allies.then_some(true);
            effects::forced_attack(args)
        })
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (spell(FORCE_ALLY, 523_920), forcer(true)),
            (spell(FORCE_PLAIN, 523_921), forcer(false)),
        ]
    }

    fn forced(seed: &str, forcer: &str) -> (Scenario, String, String) {
        install(cards());
        let mut s = board(
            seed,
            json!({ "hand": [forcer], "field": [STRIKER, ALLY] }),
            json!({}),
        );
        let (striker, ally) = (lane_id(&s, P1, 1), lane_id(&s, P1, 2));
        s.play(forcer, two_picks(&striker, &ally));
        (s, striker, ally)
    }

    #[test]
    fn r5238012_with_allies_it_attacks_a_unit_on_its_own_side() {
        let (mut s, striker, ally) = forced("r5238012-ally", FORCE_ALLY);
        s.expect_stats(ally.as_str(), json!({ "health": 9 - 3 }));
        s.expect_stats(striker.as_str(), json!({ "health": 9 - 2 }));
    }

    #[test]
    fn r5238012_absent_no_attack_on_its_own_side_happens() {
        let (mut s, striker, ally) = forced("r5238012-plain", FORCE_PLAIN);
        s.expect_stats(ally.as_str(), json!({ "health": 9 }));
        s.expect_stats(striker.as_str(), json!({ "health": 9 }));
    }
}

mod r5238013_swap_until_end_of_turn {
    use super::*;

    const SWAP_BACK: &str = "fvhv-swap-back";
    const SWAP_KEEP: &str = "fvhv-swap-keep";

    fn swapper(back: bool) -> Script {
        cast(move || {
            let mut args: SwapHandsArgs = json_as(json!({}));
            args.until_end_of_turn = back.then_some(true);
            effects::swap_hands(args)
        })
    }

    fn cards() -> Vec<(CardDef, Script)> {
        vec![
            (spell(SWAP_BACK, 523_930), swapper(true)),
            (spell(SWAP_KEEP, 523_931), swapper(false)),
        ]
    }

    /// p1 holds `swapper` and two fillers, p2 two (0) Units; p1 plays `swapper`. Returns p1's and
    /// p2's cards as they were before.
    fn swapped(seed: &str, swapper: &str) -> (Scenario, Vec<String>, Vec<String>) {
        install(cards());
        let mut s = board(
            seed,
            json!({ "hand": [swapper, FILLER, FILLER] }),
            json!({ "hand": [PAWN, PAWN] }),
        );
        let mine = pile_of(&s, P1, "hand", FILLER);
        let theirs = pile_of(&s, P2, "hand", PAWN);
        s.play(swapper, json!({}));
        for id in &theirs {
            assert!(hand_holds(&s, P1, id), "the hands swapped");
        }
        (s, mine, theirs)
    }

    #[test]
    fn r5238013_every_swapped_card_still_in_a_hand_goes_back_at_the_swappers_end_of_turn() {
        let (mut s, mine, theirs) = swapped("r5238013-back", SWAP_BACK);
        s.end_turn();
        for id in &mine {
            assert!(hand_holds(&s, P1, id), "p1's own card came back");
        }
        for id in &theirs {
            assert!(hand_holds(&s, P2, id), "p2's card went back");
            assert!(!hand_holds(&s, P1, id));
        }
    }

    #[test]
    fn r5238013_a_swapped_card_played_before_the_end_does_not_come_back() {
        let (mut s, _mine, theirs) = swapped("r5238013-played", SWAP_BACK);
        let played = theirs[0].clone();
        s.play(played.as_str(), json!({}));
        s.end_turn();
        s.expect_in_zone(played.as_str(), "field");
        assert!(!hand_holds(&s, P2, &played));
        assert!(hand_holds(&s, P2, &theirs[1]), "the one still in a hand went back");
    }

    #[test]
    fn r5238013_absent_the_swap_stays() {
        let (mut s, mine, theirs) = swapped("r5238013-keep", SWAP_KEEP);
        s.end_turn();
        for id in &theirs {
            assert!(!hand_holds(&s, P2, id), "nothing goes back");
        }
        for id in &mine {
            assert!(!hand_holds(&s, P1, id));
        }
    }
}
