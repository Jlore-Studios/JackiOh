//! Port of `packages/engine/test/layers.test.ts`.
//!
//! SPEC §10.4's stat and keyword layers (BUILD M3-T4). `unitView` is the only reader of a unit's
//! totals — "compute a unit's view on every read, never store totals" — so every assertion here
//! goes through it, one layer at a time, then as the ordered sequence §10.4 names, then over the
//! keyword set. The last test proves nothing is written back into the state.
//!
//! Fixtures: `./fixtures/combat` already carries the aura and keyword bodies these tests want
//! (Big D-fender's Defense-Position Armor, Spikey Pillow's attack drain, the printed-keyword
//! bodies). The three cards §10.4 names that it does not carry get local defs below: Suppressive
//! Aura (#46), Jlockeed's Weapons (#14) and Felinor Fiender (#92) with a Felinor for it to count.
//!
//! (TS wrote through the live instances `put` returned. Here each write goes to the card in the
//! state by id (`edit`), and each read takes the card as the state holds it now (`view`, `live`).)

use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{
    armoured, big_body, big_dfender, indestructible, plain, poisonous, shielded, spikey_pillow, stacker, taunter,
    zero_attack,
};
use crate::rules::fixtures::harness::{new_game, put, slot};

// ---------------------------------------------------------------------------
// Local defs for the three §10.4 cards ./fixtures/combat does not carry. Indices start above 1200
// so they never collide with a fixture catalog or another test file's local defs.
// ---------------------------------------------------------------------------

/// TS's `def(overrides)`: the defaults, then the overrides over them. `index` is the one TS's counter
/// gave the def (1201 up, in the order the defs are made).
fn def(index: u32, overrides: Value) -> CardDef {
    let mut card = json!({
        "index": index.to_string(),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": "" },
        "radiant": { "keywords": [], "text": "" },
    });
    for (key, value) in overrides.as_object().expect("overrides are an object") {
        card[key] = value.clone();
    }
    json_as(card)
}

/// A 2/2 with no keywords: the "2-health unit" Suppressive Aura takes to 0 (BUILD M3-T4).
fn small_body() -> CardDef {
    def(
        1201,
        json!({
            "id": "ly-small-body",
            "name": "Small Body (layers fixture)",
            "base": { "attack": 2, "health": 2, "keywords": [], "text": "2/2, no keywords" },
            "radiant": { "attack": 4, "health": 4, "keywords": [], "text": "4/4, no keywords" },
        }),
    )
}

/// §8 #46 Suppressive Aura: a Field Spell whose aura is all units −2/−2; radiant, enemies −4/−4.
fn suppressive_aura() -> CardDef {
    def(
        1202,
        json!({
            "id": "ly-suppressive-aura",
            "name": "Suppressive Aura (layers fixture)",
            "type": "Field Spell",
            "rarity": "Rare",
            "cost": { "base": 2, "embiggen": 4 },
            "base": { "keywords": [], "text": "Aura: all units -2/-2" },
            "radiant": { "keywords": [], "text": "Aura: enemy units -4/-4" },
        }),
    )
}

/// §8 #14 Jlockeed's Weapons: your units +4 attack, Rush and First Strike (+10 attack radiant).
fn jlockeeds_weapons() -> CardDef {
    def(
        1203,
        json!({
            "id": "ly-jlockeeds-weapons",
            "name": "Jlockeed's Weapons (layers fixture)",
            "type": "Field Spell",
            "cost": 4,
            "base": { "keywords": [], "text": "Aura: your units have +4 attack, Rush, First Strike" },
            "radiant": { "keywords": [], "text": "Aura: your units have +10 attack, Rush, First Strike" },
        }),
    )
}

/// §8 #92 Felinor Fiender, 5/7 → 10/14: Stack, Human, and §10.4's layer-2 set-stat card.
fn felinor_fiender() -> CardDef {
    def(
        1204,
        json!({
            "id": "ly-felinor-fiender",
            "name": "Felinor Fiender (layers fixture)",
            "tags": ["Human"],
            "rarity": "Legendary",
            "cost": 2,
            "base": {
                "attack": 5,
                "health": 7,
                "keywords": [{ "kind": "Stack" }],
                "text": "Stack. Stats = printed plus the combined stats of all your Felinors",
            },
            "radiant": {
                "attack": 10,
                "health": 14,
                "keywords": [{ "kind": "Stack" }, { "kind": "Charge" }],
                "text": "Stack, Charge; same",
            },
        }),
    )
}

/// #92 after a Fuse that unioned the `Felinor` tag onto it (R77) — the case R131 exists for. Same
/// printed stats and the same script; only the tag list differs.
fn fused_fiender() -> CardDef {
    def(
        1205,
        json!({
            "id": "ly-felinor-fiender-fused",
            "name": "Felinor Fiender, fused (layers fixture)",
            "tags": ["Human", "Felinor"],
            "rarity": "Legendary",
            "cost": 2,
            "base": {
                "attack": 5,
                "health": 7,
                "keywords": [{ "kind": "Stack" }],
                "text": "as #92, with the Felinor tag a Fuse gave it",
            },
            "radiant": { "attack": 10, "health": 14, "keywords": [{ "kind": "Stack" }], "text": "same" },
        }),
    )
}

/// A plain Felinor body for #92 to count.
fn felinor() -> CardDef {
    def(
        1206,
        json!({
            "id": "ly-felinor",
            "name": "Felinor (layers fixture)",
            "tags": ["Felinor"],
            "base": { "attack": 2, "health": 3, "keywords": [], "text": "2/3 Felinor" },
            "radiant": { "attack": 4, "health": 6, "keywords": [], "text": "4/6 Felinor" },
        }),
    )
}

/// TS's `"all" | "ally" | "enemy"`.
#[derive(Clone, Copy)]
enum Side {
    All,
    Ally,
    Enemy,
}

/// A layer-5 aura over units on one side of the field, or on both (§10.4 layer 5).
fn units_aura(stat_mod: Value, side: Side) -> AuraHook {
    let stat_mod: StatMod = json_as(stat_mod);
    aura_hook(move |args| {
        let controller = args.self_.controller;
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                matches!(unit.zone, Zone::Field { row: Row::Units, .. })
                    && match side {
                        Side::All => true,
                        Side::Ally => unit.controller == controller,
                        Side::Enemy => unit.controller != controller,
                    }
            }),
            mod_: stat_mod.clone(),
        }]
    })
}

/// §10.4 layer 2 and R39: Felinor Fiender's stats are its printed ones plus the combined layer-4
/// stats of its controller's OTHER Felinors, R13's dormant Stack cards included, never below
/// printed.
///
/// R131: it never counts itself, "matched by instance rather than by tag" — the exclusion below is
/// `unit.id === self.id` and not "the Fiender is a Human, so the tag filter already misses it",
/// because a Fuse that unions in the `Felinor` tag (R77) would otherwise let it feed on its own
/// stats. R131's second half is already in the `faceOf` + `buffs` read: a second Felinor Fiender
/// contributes its printed and buffed stats and never its own layer-2 total, so the layer cannot
/// recurse.
///
/// DISCREPANCY: src/layers.ts has no layer-2 step at all — the comment at its layer-2 slot reads
/// "Layer 2 (set-stat, Felinor Fiender) arrives with M3-T4; no Core card needs it before then" —
/// and `Script` (src/script.ts) declares no set-stat hook, so the only stat-contributing hook a
/// card has is `aura`, which §10.4 numbers as layer 5. The sum therefore rides on the aura hook
/// here. For a purely additive contribution the two are observably equal, but SPEC §10.4 orders
/// the set-stat *before* layer 4, so a later set-stat that had to be seen by a layer-4 buff (or
/// that replaced rather than added) would land in the wrong place.
///
/// `applies` may never call back into `unitView` (it would recurse), so the sum reads `faceOf` plus
/// `buffs`, which is exactly what §10.4 means by "layer-4 stats".
fn felinor_set_stat() -> AuraHook {
    aura_hook(|args| {
        let state = args.state;
        let me = args.self_;
        let mut attack = 0;
        let mut health = 0;
        let mut count = |unit: &CardInstance| {
            // R131: every OTHER Felinor you control, excluded by instance so no granted tag can make it
            // self-feed.
            if unit.id == me.id {
                return;
            }
            if !catalog::def_of(Some(state), &unit.def_id).tags.contains(&Tag::Felinor) {
                return;
            }
            let face = layers::face_of(state, unit);
            attack += face.attack + unit.buffs.attack;
            health += face.health + unit.buffs.health;
        };
        for unit in zones::active_units_of(state, me.controller) {
            count(&unit);
        }
        for unit in zones::dormant_units_of(state, me.controller) {
            count(&unit);
        }
        let id = me.id.clone();
        // R39: "never below printed", so the contribution itself never goes negative.
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| unit.id == id),
            mod_: StatMod { attack: Some(attack.max(0)), max_health: Some(health.max(0)), ..StatMod::default() },
        }]
    })
}

fn both(script: Script) -> CardScripts {
    CardScripts { base: script.clone(), radiant: script }
}

/// R349: a token that prints no Radiant form (the Ghoul Token's shape): printed 0/0 for an X/X, so
/// its catalog Radiant face is the fallback written out — the same face, the 0/0 doubled — and the
/// X doubles at runtime. And an X/X token that prints a Radiant form of its own (the Bread Token's
/// shape), whose X stays put.
fn fallback_token() -> CardDef {
    def(
        1207,
        json!({
            "id": "ly-fallback-token",
            "name": "Fallback Token (layers fixture)",
            "token": true,
            "tags": ["Token"],
            "rarity": "Token",
            "cost": 0,
            "radiantFallback": true,
            "base": { "attack": 0, "health": 0, "keywords": [{ "kind": "Pierce" }], "text": "Pierce." },
            "radiant": { "attack": 0, "health": 0, "keywords": [{ "kind": "Pierce" }], "text": "Pierce." },
        }),
    )
}

fn printed_radiant_token() -> CardDef {
    def(
        1208,
        json!({
            "id": "ly-printed-radiant-token",
            "name": "Printed Radiant Token (layers fixture)",
            "token": true,
            "tags": ["Token"],
            "rarity": "Token",
            "cost": 0,
            "base": { "attack": 0, "health": 0, "keywords": [], "text": "" },
            "radiant": { "attack": 0, "health": 0, "keywords": [{ "kind": "Rush" }], "text": "Rush." },
        }),
    )
}

fn layer_defs() -> Vec<CardDef> {
    vec![
        fallback_token(),
        printed_radiant_token(),
        small_body(),
        suppressive_aura(),
        jlockeeds_weapons(),
        felinor_fiender(),
        fused_fiender(),
        felinor(),
    ]
}

fn layer_scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        suppressive_aura().id,
        CardScripts {
            base: Script { aura: Some(units_aura(json!({ "attack": -2, "maxHealth": -2 }), Side::All)), ..Script::default() },
            radiant: Script {
                aura: Some(units_aura(json!({ "attack": -4, "maxHealth": -4 }), Side::Enemy)),
                ..Script::default()
            },
        },
    );
    scripts.insert(
        jlockeeds_weapons().id,
        CardScripts {
            base: Script {
                aura: Some(units_aura(
                    json!({ "attack": 4, "keywords": [{ "kind": "Rush" }, { "kind": "First Strike" }] }),
                    Side::Ally,
                )),
                ..Script::default()
            },
            radiant: Script {
                aura: Some(units_aura(
                    json!({ "attack": 10, "keywords": [{ "kind": "Rush" }, { "kind": "First Strike" }] }),
                    Side::Ally,
                )),
                ..Script::default()
            },
        },
    );
    scripts.insert(felinor_fiender().id, both(Script { aura: Some(felinor_set_stat()), ..Script::default() }));
    scripts.insert(fused_fiender().id, both(Script { aura: Some(felinor_set_stat()), ..Script::default() }));
    scripts
}

/// A fresh game with the local defs folded in; `newGame` resets the registry, so this runs after.
fn board(seed: &str) -> GameState {
    let state = new_game(seed, None);
    let mut catalog = catalog::registered_catalog().clone();
    for def in layer_defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = scripts::registered_scripts();
    scripts.extend(layer_scripts());
    register_scripts(scripts);
    state
}

/// `put(state, defId, ref, { radiant: true })`: the card is made Radiant before it is placed.
fn put_radiant(state: &mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance {
    let mut card = state::new_instance(state, def_id, at.player, Zone::Hand { player: at.player });
    card.radiant = true;
    let placed = zones::place_on_field(state, &mut card, at, Default::default());
    assert!(placed, "could not place {def_id} in {} {}", at.row, at.lane);
    live(state, &card.id)
}

/// `put` refuses an occupied zone; §3.2's Stack pile needs the `stack` flag.
fn stack_on(state: &mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance {
    let mut card = state::new_instance(state, def_id, at.player, Zone::Hand { player: at.player });
    let placed = zones::place_on_field(state, &mut card, at, zones::PlaceOnFieldOptions { stack: Some(true) });
    assert!(placed, "could not stack {def_id} on {} {}", at.row, at.lane);
    live(state, &card.id)
}

/// The card as the state holds it now (TS read its live object).
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id).expect("the card is in the game").clone()
}

/// A write through TS's live object: to the card in the state, by id.
fn edit(state: &mut GameState, id: &str, change: impl FnOnce(&mut CardInstance)) {
    change(find_instance_mut(state, id).expect("the card is in the game"));
}

/// `unitView(state, card)` on the card as it stands now.
fn view(state: &GameState, id: &str) -> layers::UnitView {
    layers::unit_view(state, &live(state, id))
}

fn has(state: &GameState, id: &str, kind: KeywordKind) -> bool {
    layers::unit_has(state, &live(state, id), kind)
}

fn kinds_of(id: &str, state: &GameState) -> Vec<String> {
    let set: IndexSet<String> = view(state, id).keywords.iter().map(|k| k.kind().as_str().to_string()).collect();
    let mut kinds: Vec<String> = set.into_iter().collect();
    kinds.sort();
    kinds
}

/// `stateCheck(sinkFor(state, events))`: an rng at the state's cursor, not written back (as TS).
fn run_state_check(state: &mut GameState) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    state_check::state_check(&mut sink);
    events
}

fn units(state: &GameState, player: PlayerId) -> Vec<String> {
    zones::active_units_of(state, player).iter().map(|unit| unit.id.clone()).collect()
}

fn dormant(state: &GameState, player: PlayerId) -> Vec<String> {
    zones::dormant_units_of(state, player).iter().map(|unit| unit.id.clone()).collect()
}

fn p1(row: Row, lane: i32) -> ZoneSlot {
    slot(PlayerId::P1, row, lane)
}

mod s10_4_stat_layers {
    use super::*;

    #[test]
    fn s10_4_layer_1_is_the_printed_face_of_the_running_form_with_stats_override_ahead_of_both() {
        let mut state = board("layer-1");
        let base = put(&mut state, &plain().id, p1(Row::Units, 1));
        let radiant = put_radiant(&mut state, &plain().id, p1(Row::Units, 2));
        let token = put(&mut state, &plain().id, p1(Row::Units, 3));
        // §10.4: "or `statsOverride` for tokens summoned with X/X" (Adaptive UI's Rush Token).
        edit(&mut state, &token.id, |c| c.stats_override = Some(AttackHealth { attack: 7, health: 5 }));

        let v = view(&state, &base.id);
        assert_eq!((v.attack, v.max_health, v.health), (3, 3, 3));
        let v = view(&state, &radiant.id);
        assert_eq!((v.attack, v.max_health, v.health), (6, 6, 6));
        let v = view(&state, &token.id);
        assert_eq!((v.attack, v.max_health, v.health), (7, 5, 5));
    }

    #[test]
    fn r39_layer_2_adds_the_sum_of_your_felinors_layer_4_stats_and_r13_counts_the_ones_under_a_stack() {
        let mut state = board("layer-2");
        let fiender = put(&mut state, &felinor_fiender().id, p1(Row::Units, 1));
        let v = view(&state, &fiender.id);
        assert_eq!((v.attack, v.max_health), (5, 7));

        // One Felinor on the board: its layer-4 stats are its printed 2/3 plus its own +1/+1 buff.
        let ally = put(&mut state, &felinor().id, p1(Row::Units, 2));
        edit(&mut state, &ally.id, |c| c.buffs = AttackHealth { attack: 1, health: 1 });
        let v = view(&state, &fiender.id);
        assert_eq!((v.attack, v.max_health), (8, 11));

        // R13: a card under a Stack is not on the field for anything else, and still counts here.
        let dormant_one = put(&mut state, &felinor().id, p1(Row::Units, 3));
        stack_on(&mut state, &stacker().id, p1(Row::Units, 3));
        assert!(!units(&state, PlayerId::P1).contains(&dormant_one.id));
        assert!(dormant(&state, PlayerId::P1).contains(&dormant_one.id));
        let v = view(&state, &fiender.id);
        assert_eq!((v.attack, v.max_health), (10, 14));

        // "All *your* Felinors": the opponent's are not yours (§8 #92).
        put(&mut state, &felinor().id, slot(PlayerId::P2, Row::Units, 1));
        let v = view(&state, &fiender.id);
        assert_eq!((v.attack, v.max_health), (10, 14));
    }

    #[test]
    fn r131_layer_2_never_counts_itself_even_once_a_fuse_has_given_it_the_felinor_tag() {
        let mut state = board("layer-2-self");

        // A Felinor Fiender that IS tagged Felinor: alone on the board it is its printed 5/7, so the
        // sum excluded it by instance. A tag filter on its own would have doubled it to 10/14.
        let fused = put(&mut state, &fused_fiender().id, p1(Row::Units, 1));
        let v = view(&state, &fused.id);
        assert_eq!((v.attack, v.max_health), (5, 7));

        // It still counts every OTHER Felinor, the tag it now carries changing nothing about that.
        let ally = put(&mut state, &felinor().id, p1(Row::Units, 2));
        edit(&mut state, &ally.id, |c| c.buffs = AttackHealth { attack: 1, health: 1 });
        let v = view(&state, &fused.id);
        assert_eq!((v.attack, v.max_health), (8, 11));

        // Two of them: each adds the other's printed-and-buffed stats and never its own layer-2 total,
        // so the layer does not recurse (R131's second half, R116).
        let second = put(&mut state, &fused_fiender().id, p1(Row::Units, 3));
        let v = view(&state, &fused.id);
        assert_eq!((v.attack, v.max_health), (13, 18));
        let v = view(&state, &second.id);
        assert_eq!((v.attack, v.max_health), (13, 18));
    }

    #[test]
    fn s10_4_layer_4_adds_the_instances_permanent_buffs_to_both_stats() {
        let mut state = board("layer-4");
        let unit = put(&mut state, &plain().id, p1(Row::Units, 1));
        let v = view(&state, &unit.id);
        assert_eq!((v.attack, v.max_health), (3, 3));

        edit(&mut state, &unit.id, |c| c.buffs = AttackHealth { attack: 2, health: 4 });
        let v = view(&state, &unit.id);
        assert_eq!((v.attack, v.max_health, v.health), (5, 7, 7));
    }

    #[test]
    fn s10_4_layer_5_auras_apply_while_their_source_is_in_play_and_stop_the_moment_it_leaves_14() {
        let mut state = board("layer-5");
        let unit = put(&mut state, &plain().id, p1(Row::Units, 1));
        let weapons = put(&mut state, &jlockeeds_weapons().id, p1(Row::Backrow, 1));
        assert_eq!(view(&state, &unit.id).attack, 7);

        // §8 #14: "removed when it leaves". Nothing was stored on the unit, so nothing has to be undone.
        let weapons = live(&state, &weapons.id);
        zones::remove_from_field(&mut state, &weapons, Default::default());
        assert_eq!(view(&state, &unit.id).attack, 3);
        assert_eq!(live(&state, &unit.id).buffs, AttackHealth { attack: 0, health: 0 });
    }

    #[test]
    fn r13_a_dormant_stack_card_projects_no_aura() {
        let mut state = board("dormant-aura");
        let ally = put(&mut state, &plain().id, p1(Row::Units, 2));
        edit(&mut state, &ally.id, |c| c.position = Some(Position::Def));
        put(&mut state, &big_dfender().id, p1(Row::Units, 1));
        // Defense Position's own Armor +1 plus Big D-fender's +2 (§4.1, §8 #1).
        assert_eq!(view(&state, &ally.id).armor, 3);

        // §3.2: only the top of a pile is on the field, so only it projects its aura.
        stack_on(&mut state, &stacker().id, p1(Row::Units, 1));
        assert_eq!(view(&state, &ally.id).armor, 1);
    }

    #[test]
    fn s10_4_layer_5_floors_attack_at_0_and_floors_max_health_at_nothing_spikey_pillow() {
        let mut state = board("attack-floor");
        let pillow = put(&mut state, &spikey_pillow().id, p1(Row::Units, 1));
        let small = put(&mut state, &poisonous().id, p1(Row::Units, 2));
        let big = put(&mut state, &plain().id, p1(Row::Units, 3));

        // 1 − 2 and 0 − 2 both floor at 0; 3 − 2 does not.
        assert_eq!(view(&state, &small.id).attack, 0);
        assert_eq!(view(&state, &pillow.id).attack, 0);
        assert_eq!(view(&state, &big.id).attack, 1);
        // §10.4: only attack is floored — max health is left to fall, for the state check to read.
        assert_eq!(view(&state, &small.id).max_health, 1);
    }

    #[test]
    fn s10_4_layer_6_reads_current_health_as_max_health_minus_damage() {
        let mut state = board("layer-6");
        let unit = put(&mut state, &plain().id, p1(Row::Units, 1));
        edit(&mut state, &unit.id, |c| c.damage = 2);
        let v = view(&state, &unit.id);
        assert_eq!((v.max_health, v.health), (3, 1));

        // The damage stays where it is; the health it leaves behind follows the new max.
        edit(&mut state, &unit.id, |c| c.buffs = AttackHealth { attack: 0, health: 5 });
        let v = view(&state, &unit.id);
        assert_eq!((v.max_health, v.health), (8, 6));
    }

    #[test]
    fn s10_4_composes_layers_1_2_4_5_and_6_in_that_order() {
        let mut state = board("layer-order");
        let fiender = put(&mut state, &felinor_fiender().id, p1(Row::Units, 2));
        // 1: the printed face.
        let v = view(&state, &fiender.id);
        assert_eq!((v.attack, v.max_health, v.health), (5, 7, 7));

        // 2: the set-stat, reading the Felinor's layer-4 stats (printed 2/3 plus its +1/+1).
        let ally = put(&mut state, &felinor().id, p1(Row::Units, 3));
        edit(&mut state, &ally.id, |c| c.buffs = AttackHealth { attack: 1, health: 1 });
        let v = view(&state, &fiender.id);
        assert_eq!((v.attack, v.max_health), (8, 11));

        // 4: the Fiender's own buffs, on top of the set-stat.
        edit(&mut state, &fiender.id, |c| c.buffs = AttackHealth { attack: 1, health: 2 });
        let v = view(&state, &fiender.id);
        assert_eq!((v.attack, v.max_health), (9, 13));

        // 5: auras come last, and an aura never changes the layer-4 stats layer 2 summed.
        put(&mut state, &spikey_pillow().id, p1(Row::Units, 1));
        let v = view(&state, &ally.id);
        assert_eq!((v.attack, v.max_health), (1, 4));
        let v = view(&state, &fiender.id);
        assert_eq!((v.attack, v.max_health), (7, 13));

        // 6: current health is the last step, off the max the layers above settled on.
        edit(&mut state, &fiender.id, |c| c.damage = 3);
        let v = view(&state, &fiender.id);
        assert_eq!((v.attack, v.max_health, v.health), (7, 13, 10));
    }

    #[test]
    fn s10_4_applies_layer_4s_buffs_before_layer_5s_auras_and_floors_attack_once_at_the_end() {
        let mut state = board("floor-once");
        let unit = put(&mut state, &zero_attack().id, p1(Row::Units, 2));
        edit(&mut state, &unit.id, |c| c.buffs = AttackHealth { attack: 3, health: 0 });
        put(&mut state, &spikey_pillow().id, p1(Row::Units, 1));

        // 0 + 3 − 2 = 1. A floor taken before the buff — max(0, 0 − 2) = 0, then +3 — would read 3,
        // so the order of layers 4 and 5 against the single final floor is observable here.
        assert_eq!(view(&state, &unit.id).attack, 1);
    }

    #[test]
    fn c46_suppressive_aura_takes_a_2_health_units_max_health_to_0_and_the_next_state_check_kills_it() {
        let mut state = board("suppressive-kill");
        let small = put(&mut state, &small_body().id, p1(Row::Units, 1));
        put(&mut state, &suppressive_aura().id, p1(Row::Backrow, 1));
        let v = view(&state, &small.id);
        assert_eq!((v.attack, v.max_health, v.health), (0, 0, 0));

        let events = run_state_check(&mut state);
        let destroyed: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                GameEvent::Destroyed { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(destroyed, vec![small.id.clone()]);
        assert!(state.players.p1.units[0].is_none());
        assert!(state.players.p1.graveyard.iter().any(|c| c.id == small.id));
    }

    #[test]
    fn c46_removing_suppressive_aura_restores_a_surviving_units_max_health() {
        let mut state = board("suppressive-restore");
        let unit = put(&mut state, &plain().id, p1(Row::Units, 1));
        let aura = put(&mut state, &suppressive_aura().id, p1(Row::Backrow, 1));
        let v = view(&state, &unit.id);
        assert_eq!((v.attack, v.max_health, v.health), (1, 1, 1));

        // 1 health is still above 0, so the state check leaves it alone (§4.5 step 1).
        run_state_check(&mut state);
        assert_eq!(
            state.players.p1.units[0].as_ref().and_then(|pile| pile.first()).map(|c| c.id.clone()),
            Some(unit.id.clone())
        );

        let aura = live(&state, &aura.id);
        zones::remove_from_field(&mut state, &aura, Default::default());
        let v = view(&state, &unit.id);
        assert_eq!((v.attack, v.max_health, v.health), (3, 3, 3));
    }

    #[test]
    fn s10_4_layer_5_an_aura_picks_its_own_side_radiant_suppressive_aura_touches_enemy_units_only() {
        let mut state = board("suppressive-radiant");
        let mine = put(&mut state, &big_body().id, p1(Row::Units, 1));
        let theirs = put(&mut state, &big_body().id, slot(PlayerId::P2, Row::Units, 1));
        put_radiant(&mut state, &suppressive_aura().id, p1(Row::Backrow, 1));

        let v = view(&state, &mine.id);
        assert_eq!((v.attack, v.max_health), (5, 10));
        let v = view(&state, &theirs.id);
        assert_eq!((v.attack, v.max_health), (1, 6));
    }
}

mod s10_4_keyword_set {
    use super::*;

    #[test]
    fn s10_4_the_keyword_set_is_the_union_of_printed_granted_aura_and_position_sources() {
        let mut state = board("keyword-union");
        let unit = put(&mut state, &taunter().id, p1(Row::Units, 1));
        edit(&mut state, &unit.id, |c| {
            c.granted_keywords = vec![Keyword::Lifesteal];
            c.position = Some(Position::Def);
        });
        put(&mut state, &jlockeeds_weapons().id, p1(Row::Backrow, 1));

        // Printed Taunt, granted Lifesteal, the aura's Rush and First Strike, Defense Position's Armor.
        assert_eq!(kinds_of(&unit.id, &state), ["Armor", "First Strike", "Lifesteal", "Rush", "Taunt"]);
        assert_eq!(view(&state, &unit.id).armor, 1);
    }

    #[test]
    fn s6_3_vanilla_strips_printed_keywords_but_not_granted_keywords_and_keeps_stats_buffs_and_damage() {
        let mut state = board("vanilla");
        let unit = put(&mut state, &taunter().id, p1(Row::Units, 1));
        edit(&mut state, &unit.id, |c| {
            c.buffs = AttackHealth { attack: 1, health: 1 };
            c.damage = 2;
            c.granted_keywords = vec![Keyword::Rush];
        });
        assert_eq!(kinds_of(&unit.id, &state), ["Rush", "Taunt"]);

        edit(&mut state, &unit.id, |c| c.vanilla = true);
        let v = view(&state, &unit.id);
        assert_eq!(v.keywords.iter().map(|k| k.kind().as_str()).collect::<Vec<_>>(), ["Rush"]);
        // §6.1 Vanilla: "Clears printed keywords and scripts; keeps stats, buffs and damage".
        assert_eq!((v.attack, v.max_health, v.health), (3, 6, 4));

        // An aura is the board's text, not the unit's, so a Vanilla unit still takes aura grants.
        put(&mut state, &jlockeeds_weapons().id, p1(Row::Backrow, 1));
        assert_eq!(kinds_of(&unit.id, &state), ["First Strike", "Rush"]);
        assert_eq!(view(&state, &unit.id).attack, 7);
    }

    #[test]
    fn s6_3_a_vanilla_units_own_aura_stops_applying_because_vanilla_clears_its_scripts() {
        let mut state = board("vanilla-aura");
        let dfender = put(&mut state, &big_dfender().id, p1(Row::Units, 1));
        let ally = put(&mut state, &plain().id, p1(Row::Units, 2));
        edit(&mut state, &ally.id, |c| c.position = Some(Position::Def));
        assert_eq!(view(&state, &ally.id).armor, 3);

        edit(&mut state, &dfender.id, |c| c.vanilla = true);
        // DISCREPANCY: src/layers.ts does A; SPEC §6.1/§6.3 say B.
        //   A: `auraSources`/`auraMods` read `scriptOf(source).aura` and never look at the source's
        //      `vanilla` flag, so a Vanilla'd Big D-fender keeps projecting its Armor aura (armor 3).
        //   B: Vanilla "clears printed keywords and scripts" (§6.1) — "the card's text stops applying
        //      — its printed keywords and its scripts" (src/effects/transform.ts's own doc comment) —
        //      and `aura` is part of a card's Script (§10.9), so only Defense Position's own Armor +1
        //      should be left.
        assert_eq!(view(&state, &ally.id).armor, 1);
    }

    #[test]
    fn s4_1_a_unit_in_defense_position_gains_taunt_and_armor_1() {
        let mut state = board("defense-position");
        let unit = put(&mut state, &plain().id, p1(Row::Units, 1));
        let v = view(&state, &unit.id);
        assert_eq!((v.position, v.armor), (Position::Atk, 0));
        assert!(!has(&state, &unit.id, KeywordKind::Taunt));

        edit(&mut state, &unit.id, |c| c.position = Some(Position::Def));
        let v = view(&state, &unit.id);
        assert_eq!(v.position, Position::Def);
        assert!(has_keyword(&v.keywords, KeywordKind::Taunt));
        assert_eq!(v.armor, 1);
        // The grant is positional, not a stored keyword: nothing was written onto the instance.
        assert_eq!(live(&state, &unit.id).granted_keywords, Vec::<Keyword>::new());
    }

    #[test]
    fn s10_4_armor_is_summed_across_every_source_printed_granted_defense_position_and_auras() {
        let mut state = board("armor-sum");
        let unit = put(&mut state, &armoured().id, p1(Row::Units, 2));
        assert_eq!(view(&state, &unit.id).armor, 7);

        edit(&mut state, &unit.id, |c| c.granted_keywords = vec![Keyword::Armor { n: 2 }]);
        assert_eq!(view(&state, &unit.id).armor, 9);

        edit(&mut state, &unit.id, |c| c.position = Some(Position::Def));
        assert_eq!(view(&state, &unit.id).armor, 10);

        // §4.1: Big D-fender's aura stacks with printed Armor and with Defense Position's own +1.
        put(&mut state, &big_dfender().id, p1(Row::Units, 1));
        let v = view(&state, &unit.id);
        assert_eq!(v.armor, 12);
        assert_eq!(armor_of(&v.keywords), 12);
    }

    #[test]
    fn r347_an_indestructible_unit_never_has_taunt_printed_granted_or_from_defense_position() {
        let mut state = board("indestructible-no-taunt");
        let unit = put(&mut state, &indestructible().id, p1(Row::Units, 1));
        edit(&mut state, &unit.id, |c| {
            c.granted_keywords = vec![Keyword::Taunt];
            c.position = Some(Position::Def);
        });
        // Granted Taunt and Defense Position's Taunt both give way to Indestructible; the Armor stays.
        assert!(!has(&state, &unit.id, KeywordKind::Taunt));
        assert!(has(&state, &unit.id, KeywordKind::Indestructible));
        assert_eq!(view(&state, &unit.id).armor, 1);
        // A read-time subtraction, not a removal (§10.4): the grant is still on the instance.
        assert_eq!(live(&state, &unit.id).granted_keywords, vec![Keyword::Taunt]);

        // Take the Indestructible away (a Vanilla clears the printed keyword) and the Taunt is back.
        edit(&mut state, &unit.id, |c| c.vanilla = true);
        assert!(!has(&state, &unit.id, KeywordKind::Indestructible));
        assert!(has(&state, &unit.id, KeywordKind::Taunt));
    }

    #[test]
    fn r349_a_unit_with_no_radiant_form_of_its_own_doubles_its_base_stat_layer_when_radiant_a_summons_x_x_included() {
        let mut state = board("radiant-fallback");
        let ghoul = put(&mut state, &fallback_token().id, p1(Row::Units, 1));
        edit(&mut state, &ghoul.id, |c| {
            c.stats_override = Some(AttackHealth { attack: 3, health: 3 });
            c.buffs = AttackHealth { attack: 1, health: 1 };
            c.damage = 2;
        });
        let v = view(&state, &ghoul.id);
        assert_eq!((v.attack, v.max_health, v.health), (4, 4, 2));

        // §5.2: the base-stat layer swaps, buffs and damage are kept — here the swap is R349's doubling.
        edit(&mut state, &ghoul.id, |c| c.radiant = true);
        assert_eq!(
            serde_json::to_value(layers::face_of(&state, &live(&state, &ghoul.id))).expect("a face serialises"),
            json!({ "attack": 6, "health": 6, "keywords": [{ "kind": "Pierce" }] })
        );
        let v = view(&state, &ghoul.id);
        assert_eq!((v.attack, v.max_health, v.health), (7, 7, 5));
        // The X on the instance is the base face's, never rewritten.
        assert_eq!(live(&state, &ghoul.id).stats_override, Some(AttackHealth { attack: 3, health: 3 }));

        // A token that prints a Radiant form keeps its X on both faces, as §7's Bread Token does.
        let bread = put(&mut state, &printed_radiant_token().id, p1(Row::Units, 2));
        edit(&mut state, &bread.id, |c| {
            c.stats_override = Some(AttackHealth { attack: 3, health: 3 });
            c.radiant = true;
        });
        assert_eq!(
            serde_json::to_value(layers::face_of(&state, &live(&state, &bread.id))).expect("a face serialises"),
            json!({ "attack": 3, "health": 3, "keywords": [{ "kind": "Rush" }] })
        );
    }

    #[test]
    fn r46_r347_a_marked_indestructible_unit_stamps_the_turn_which_holds_its_taunt_off_that_turn_only_once_it_is_no_longer_indestructible() {
        let mut state = board("taunt-suppression");
        state.turn = 4;
        let unit = put(&mut state, &indestructible().id, p1(Row::Units, 1));
        edit(&mut state, &unit.id, |c| {
            c.granted_keywords = vec![Keyword::Taunt];
            c.position = Some(Position::Def);
        });
        // R347: while Indestructible it has no Taunt at all.
        assert!(!has(&state, &unit.id, KeywordKind::Taunt));

        // R46: a would-destroy on an Indestructible unit switches it to Attack and stamps the turn.
        edit(&mut state, &unit.id, |c| c.marked_destroyed = Some(true));
        run_state_check(&mut state);
        assert_eq!(live(&state, &unit.id).taunt_suppressed_turn, Some(4));
        assert_eq!(view(&state, &unit.id).position, Position::Atk);
        assert!(!has(&state, &unit.id, KeywordKind::Taunt));

        // A Vanilla the same turn takes its printed Indestructible: R347 no longer holds the granted
        // Taunt off, and R46's stamp still does, for "this turn" and no longer.
        edit(&mut state, &unit.id, |c| c.vanilla = true);
        assert!(!has(&state, &unit.id, KeywordKind::Taunt));
        state.turn = 5;
        assert!(has(&state, &unit.id, KeywordKind::Taunt));
        // And a stamp from an earlier turn suppresses nothing.
        state.turn = 6;
        edit(&mut state, &unit.id, |c| c.taunt_suppressed_turn = Some(5));
        assert!(has(&state, &unit.id, KeywordKind::Taunt));

        // While the stamp does name this turn, even Defense Position's Taunt goes; its Armor stays.
        state.turn = 5;
        edit(&mut state, &unit.id, |c| c.position = Some(Position::Def));
        assert!(!has(&state, &unit.id, KeywordKind::Taunt));
        assert_eq!(view(&state, &unit.id).armor, 1);
    }

    #[test]
    fn s6_1_a_spent_divine_shield_and_a_used_reborn_are_gone_until_they_are_granted_again() {
        let mut state = board("spent-keywords");
        let shield = put(&mut state, &shielded().id, p1(Row::Units, 1));
        assert!(has(&state, &shield.id, KeywordKind::DivineShield));

        // §6.1: "Negate the first damage instance, then lose it" (§10.1's `divineShieldSpent`).
        edit(&mut state, &shield.id, |c| c.divine_shield_spent = Some(true));
        assert!(!has(&state, &shield.id, KeywordKind::DivineShield));

        // §10.1: "gone until granted again". A grant clears the flag (src/effects/buff.ts `grantTo`),
        // and the layers hand the keyword back the moment it is clear.
        edit(&mut state, &shield.id, |c| {
            c.granted_keywords = vec![Keyword::DivineShield];
            c.divine_shield_spent = None;
        });
        assert!(has(&state, &shield.id, KeywordKind::DivineShield));

        let reborn = put(&mut state, &plain().id, p1(Row::Units, 2));
        edit(&mut state, &reborn.id, |c| c.granted_keywords = vec![Keyword::Reborn]);
        assert!(has(&state, &reborn.id, KeywordKind::Reborn));
        // §4.5 step 4: a Reborn body comes back without Reborn.
        edit(&mut state, &reborn.id, |c| c.reborn_spent = Some(true));
        assert!(!has(&state, &reborn.id, KeywordKind::Reborn));
    }
}

mod s10_4_recomputation {
    use super::*;

    #[test]
    fn s10_4_recomputes_the_view_on_every_read_and_stores_no_total_in_the_state() {
        let mut state = board("no-cache");
        let unit = put(&mut state, &plain().id, p1(Row::Units, 2));
        let first = view(&state, &unit.id);
        assert_eq!((first.attack, first.max_health, first.health), (3, 3, 3));

        // Reading writes nothing: §10.1 keeps the state JSON-only, so a round-trip compares exactly.
        let snapshot = serde_json::to_string(&state).expect("a state serialises");
        view(&state, &unit.id);
        view(&state, &unit.id);
        assert_eq!(serde_json::to_string(&state).expect("a state serialises"), snapshot);

        // The instance carries the inputs, never the totals ("never store totals", §10.4).
        let fields = serde_json::to_value(live(&state, &unit.id)).expect("a card serialises");
        let fields = fields.as_object().expect("a card is an object");
        assert!(!fields.contains_key("attack"));
        assert!(!fields.contains_key("maxHealth"));
        assert!(!fields.contains_key("health"));
        assert!(!fields.contains_key("keywords"));
        assert!(!fields.contains_key("armor"));

        // A second read around a mutation sees the new board, and the first view is untouched by it.
        put(&mut state, &spikey_pillow().id, p1(Row::Units, 1));
        edit(&mut state, &unit.id, |c| {
            c.buffs = AttackHealth { attack: 1, health: 1 };
            c.damage = 1;
        });
        let v = view(&state, &unit.id);
        assert_eq!((v.attack, v.max_health, v.health), (2, 4, 3));
        assert_eq!((first.attack, first.max_health, first.health), (3, 3, 3));
    }
}
