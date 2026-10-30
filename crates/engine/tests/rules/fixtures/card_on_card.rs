//! Test-only cards for #1850's card-on-card verbs (R1850000–R1850006).
//!
//! No catalog card uses these verbs until the Stinkyscape set is built (#1836), so these are the
//! engine's proof. Ids are prefixed `coc-` and indexed from 18500 up, clear of every other fixture file
//! (BUILD §0).

use std::sync::LazyLock;

use jackioh_engine::effects;
use jackioh_engine::reduce::begin_game;
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use super::harness::{act_ok, both, in_hand, new_game, spread};

/// The keywords Rod's fixtures turn Radiant at (R1850005).
pub const ROD_KEYWORDS: i32 = 3;
/// Legend's declared number: what its end-of-turn hook and its Death deal (base face).
pub const LEGEND_PING: i32 = 1;
pub const LEGEND_RADIANT_PING: i32 = 2;
/// Boost's +N/+N on its base face.
pub const BOOST: i32 = 2;
pub const RADIANT_BOOST: i32 = 4;
/// The exile aura's cost cap: "permanents that cost (2) or less" (Stinkyscape #113's shape).
pub const EXILE_CAP: i32 = 2;

/// A Core Common card whose faces both print `keywords`, with `extra`'s keys over it.
fn def_with(index: u32, name: &str, type_: &str, cost: i32, keywords: Value, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("coc-{name}"),
            "index": index.to_string(),
            "name": format!("{name} (#1850)"),
            "set": "Core",
            "type": type_,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": cost,
            "base": { "keywords": keywords, "text": name },
            "radiant": { "keywords": keywords, "text": name },
        }),
        extra,
    ))
}

fn def(index: u32, name: &str, type_: &str, cost: i32, keywords: Value) -> CardDef {
    def_with(index, name, type_, cost, keywords, json!({}))
}

/// A Unit whose faces both print `keywords`; the Radiant face doubles the stats.
fn unit_with(index: u32, name: &str, cost: i32, stats: (i32, i32), keywords: Value, extra: Value) -> CardDef {
    let (attack, health) = stats;
    let faces = json!({
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": name },
        "radiant": { "attack": attack * 2, "health": health * 2, "keywords": keywords, "text": name },
    });
    def_with(index, name, "Unit", cost, keywords, spread(faces, extra))
}

fn unit(index: u32, name: &str, cost: i32, stats: (i32, i32), keywords: Value) -> CardDef {
    unit_with(index, name, cost, stats, keywords, json!({}))
}

// Copy a Unit's text (R1850000)

pub fn copier() -> CardDef {
    unit(18500, "copier", 1, (2, 2), json!([]))
}

pub fn radiant_copier() -> CardDef {
    unit(18501, "radiant-copier", 1, (2, 2), json!([]))
}

/// A Legendary 3/3 with Taunt: "Aura: your other Units have +1 Attack. Whenever you play a card, End of
/// turn, Death and Activate: deal {ping} to the enemy hero."
pub fn legend() -> CardDef {
    unit_with(
        18502,
        "legend",
        1,
        (3, 3),
        json!([{ "kind": "Taunt" }]),
        json!({
            "rarity": "Legendary",
            "radiant": {
                "attack": 6,
                "health": 6,
                "keywords": [{ "kind": "Taunt" }, { "kind": "Lifesteal" }],
                "text": "legend",
            },
            "params": [{
                "key": "ping",
                "base": LEGEND_PING,
                "radiant": LEGEND_RADIANT_PING,
                "better": "up",
                "step": 1,
                "min": 1,
            }],
        }),
    )
}

/// An Immutable 2/2 copier (§6.1): its text can't be changed, so it copies nothing.
pub fn immutable_copier() -> CardDef {
    unit(
        18503,
        "immutable-copier",
        1,
        (2, 2),
        json!([{ "kind": "Immutable" }]),
    )
}

// Bodies the verbs act on

pub fn cheap() -> CardDef {
    unit(18504, "cheap", 1, (1, 1), json!([]))
}

pub fn dear() -> CardDef {
    unit(18505, "dear", 3, (3, 3), json!([]))
}

pub fn cheap_field() -> CardDef {
    def(18506, "cheap-field", "Field Spell", 1, json!([]))
}

pub fn cheap_trap() -> CardDef {
    def(18507, "cheap-trap", "Trap", 1, json!([]))
}

pub fn human() -> CardDef {
    unit_with(18508, "human", 1, (1, 1), json!([]), json!({ "tags": ["Human"] }))
}

pub fn crier() -> CardDef {
    unit(18509, "crier", 1, (1, 1), json!([]))
}

pub fn token() -> CardDef {
    unit_with(18510, "token", 1, (1, 1), json!([]), json!({ "token": true }))
}

// Duo and Squad (R1850001)

/// A 2/2: "Cry: Duo with a permanent and make it Radiant next turn"; Radiant: "Cry: Squad with all
/// your permanents and make them Radiant next turn" (Stinkyscape #110 iKwzki's shape).
pub fn duo() -> CardDef {
    unit(18511, "duo", 1, (2, 2), json!([]))
}

// Exile auras (R1850002)

/// A (2) 4/4: "Aura: all permanents that cost (2) or less are exiled"; Radiant: all enemy ones
/// (Stinkyscape #113 A Very Dandy Lad's shape, at a cost its own aura reaches).
pub fn cheap_exiler() -> CardDef {
    unit(18512, "cheap-exiler", EXILE_CAP, (4, 4), json!([]))
}

/// A (4) Human 4/4: "Aura: all Units that are neither Human, Starship Piece nor Starship are exiled"
/// (Stinkyscape #115 Scornboi's shape).
pub fn tag_exiler() -> CardDef {
    unit_with(
        18513,
        "tag-exiler",
        4,
        (4, 4),
        json!([]),
        json!({ "tags": ["Human"] }),
    )
}

// Hold beneath (R1850003)

/// A (2) Field Spell: "Cry: stack a permanent under this. Death: return it to its controller"
/// (Stinkyscape #9 Ticketed's shape).
pub fn ticket() -> CardDef {
    def(18514, "ticket", "Field Spell", 2, json!([]))
}

/// A (1) Spell: "Stuff an enemy Unit in a 0/4 Sack"; Radiant: "… in a 0/4 Sack for yourself"
/// (Stinkyscape #135 Sacked's shape, as a Spell).
pub fn sacker() -> CardDef {
    def(18515, "sacker", "Spell", 1, json!([]))
}

/// The Sack token, 0/4: "Death: summon the Unit stuffed into this" (Stinkyscape #135.1's shape).
pub fn sack() -> CardDef {
    unit_with(18516, "sack", 1, (0, 4), json!([]), json!({ "token": true }))
}

// Cast it for yourself (R1850004)

/// A (2) Trap: "Reveals when your opponent casts a beneficial Spell on themselves or a permanent they
/// control. Cast it beneficially for yourself instead"; Radiant: "Cast its Radiant version …"
/// (Stinkyscape #139 Wormhole's shape).
pub fn wormhole() -> CardDef {
    def(18517, "wormhole", "Trap", 2, json!([]))
}

/// A (1) Spell: "Give a Unit +{boost}/+{boost}", aimed in its caster's favor.
pub fn boost() -> CardDef {
    def_with(
        18518,
        "boost",
        "Spell",
        1,
        json!([]),
        json!({
            "params": [{
                "key": "boost",
                "base": BOOST,
                "radiant": RADIANT_BOOST,
                "better": "up",
                "step": 1,
                "min": 1,
            }],
        }),
    )
}

/// A (1) Spell with no target: "Deal 1 to the enemy hero".
pub fn bolt() -> CardDef {
    def(18519, "bolt", "Spell", 1, json!([]))
}

// Radiant on reaching N keywords (R1850005)

/// A 1/1 with Taunt and Rush: "When this has {ROD_KEYWORDS} or more keywords, make it Radiant"
/// (Stinkyscape #16 Rod of Ages' 10, made small).
pub fn rod() -> CardDef {
    unit(
        18520,
        "rod",
        1,
        (1, 1),
        json!([{ "kind": "Taunt" }, { "kind": "Rush" }]),
    )
}

/// A Field Spell with Immutable and Crater and Rod's text.
pub fn rod_field() -> CardDef {
    def(
        18521,
        "rod-field",
        "Field Spell",
        1,
        json!([{ "kind": "Immutable" }, { "kind": "Crater" }]),
    )
}

pub static COC_DEFS: LazyLock<Vec<CardDef>> = LazyLock::new(|| {
    vec![
        copier(),
        radiant_copier(),
        legend(),
        immutable_copier(),
        cheap(),
        dear(),
        cheap_field(),
        cheap_trap(),
        human(),
        crier(),
        token(),
        duo(),
        cheap_exiler(),
        tag_exiler(),
        ticket(),
        sacker(),
        sack(),
        wormhole(),
        boost(),
        bolt(),
        rod(),
        rod_field(),
    ]
});

fn ping(amount: i32) -> Effect {
    effects::damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))
}

/// "Aura: your other Units have +1 Attack." Instance data only, as an aura's `applies` must read.
fn lend_attack() -> AuraHook {
    aura_hook(|a| {
        let me = a.self_.id.clone();
        let controller = a.self_.controller;
        vec![AuraEntry {
            applies: Box::new(move |card: &CardInstance| {
                card.id != me
                    && card.controller == controller
                    && matches!(card.zone, Zone::Field { row: Row::Units, .. })
            }),
            mod_: StatMod {
                attack: Some(1),
                ..StatMod::default()
            },
        }]
    })
}

/// A copier's Cry: copy the chosen Unit's text, its Radiant text when `radiant`.
fn copier_script(radiant: bool) -> CardScripts {
    both(Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "of": ["unit"], "excludeSelf": true }),
        )],
        cry: Some(hook(move |_ctx| {
            vec![effects::copy_unit_text(effects::CopyUnitTextArgs {
                target: None,
                radiant: if radiant { Some(true) } else { None },
            })]
        })),
        ..Script::default()
    })
}

/// Wormhole's answer: the opponent's beneficial Spell, countered into this player's hand and cast by
/// them for their own side — made Radiant first on the Radiant face.
fn wormhole_trigger(radiant: bool) -> TriggerDef {
    TriggerDef::new(
        "coc-wormhole",
        &[GameEventType::CardAnnounced],
        move |ctx, event| {
            let Some(spell) = beneficial_on_own_side(&*ctx.state, event) else {
                return vec![];
            };
            let card = json!({ "of": "instance", "instanceId": spell });
            let mut out = vec![effects::counter_play(json_as(
                json!({ "to": "thief", "target": card.clone() }),
            ))];
            if radiant {
                out.push(effects::set_radiant(json_as(json!({ "instanceId": spell }))));
            }
            out.push(effects::cast(json_as(
                json!({ "target": card, "friendsOnly": true }),
            )));
            out
        },
    )
    .with_when(|ctx, event| {
        matches!(event, GameEvent::CardAnnounced { player, .. } if *player != ctx.controller)
            && beneficial_on_own_side(&*ctx.state, event).is_some()
    })
}

fn rod_script() -> CardScripts {
    both(Script {
        radiant_at_keywords: Some(ROD_KEYWORDS),
        ..Script::default()
    })
}

pub static COC_SCRIPTS: LazyLock<IndexMap<String, CardScripts>> = LazyLock::new(|| {
    let mut table: IndexMap<String, CardScripts> = IndexMap::new();
    for card in [
        immutable_copier(),
        cheap(),
        dear(),
        cheap_field(),
        cheap_trap(),
        human(),
        token(),
    ] {
        table.insert(card.id, both(Script::default()));
    }
    table.insert(copier().id, copier_script(false));
    table.insert(radiant_copier().id, copier_script(true));
    let legend_ping = || hook(|ctx| vec![ping(param(&*ctx, "ping"))]);
    table.insert(
        legend().id,
        both(Script {
            aura: Some(lend_attack()),
            triggers: vec![
                TriggerDef::new("coc-legend-played", &[GameEventType::CardPlayed], |ctx, _event| {
                    vec![ping(param(&*ctx, "ping"))]
                })
                .with_when(|ctx, event| {
                    matches!(event, GameEvent::CardPlayed { player, .. } if *player == ctx.controller)
                }),
            ],
            end_of_turn: Some(legend_ping()),
            death: Some(legend_ping()),
            activations: vec![ActivationDecl {
                id: "ping".to_string(),
                label: "Deal {ping} to the enemy hero".to_string(),
                uses: ActivationUses::Count(1),
                cost: None,
                targets: vec![],
                modes: vec![],
                can_activate: None,
                has: None,
                run: legend_ping(),
            }],
            ..Script::default()
        }),
    );
    table.insert(
        crier().id,
        both(Script {
            cry: Some(hook(|_ctx| vec![ping(1)])),
            ..Script::default()
        }),
    );
    let any_permanent = json!({ "of": ["unit", "backrow"], "excludeSelf": true });
    table.insert(
        duo().id,
        CardScripts {
            base: Script {
                targets: vec![TargetDecl::target(1, 1, any_permanent.clone())],
                cry: Some(hook(|_ctx| {
                    vec![effects::duo_radiant(effects::DuoRadiantArgs::default())]
                })),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(hook(|_ctx| vec![effects::squad_radiant()])),
                ..Script::default()
            },
        },
    );
    let cheap_permanents = |side: &str| ExileAura {
        scope: json_as(json!({ "side": side, "rows": ["units", "backrow"] })),
        cost: Some(json_as(json!({ "maxCost": EXILE_CAP }))),
        not_keywords: Vec::new(),
    };
    table.insert(
        cheap_exiler().id,
        CardScripts {
            base: Script {
                exile_auras: vec![cheap_permanents("any")],
                ..Script::default()
            },
            radiant: Script {
                exile_auras: vec![cheap_permanents("enemy")],
                ..Script::default()
            },
        },
    );
    table.insert(
        tag_exiler().id,
        both(Script {
            exile_auras: vec![ExileAura {
                scope: json_as(json!({ "side": "any", "notTags": ["Human"] })),
                cost: None,
                not_keywords: vec![KeywordKind::StarshipPiece, KeywordKind::Starship],
            }],
            ..Script::default()
        }),
    );
    table.insert(
        ticket().id,
        both(Script {
            targets: vec![TargetDecl::target(1, 1, any_permanent)],
            cry: Some(hook(|_ctx| {
                vec![effects::hold_beneath(effects::HoldArgs::default())]
            })),
            ..Script::default()
        }),
    );
    let sack_for = |player: &str| {
        let player = player.to_string();
        Script {
            targets: vec![TargetDecl::target(
                1,
                1,
                json!({ "side": "enemy", "of": ["unit"] }),
            )],
            cry: Some(hook(move |_ctx| {
                vec![effects::hold_beneath(json_as(json!({
                    "release": "holdersController",
                    "holder": { "defId": sack().id, "player": player },
                })))]
            })),
            ..Script::default()
        }
    };
    table.insert(
        sacker().id,
        CardScripts {
            base: sack_for("enemy"),
            radiant: sack_for("self"),
        },
    );
    table.insert(sack().id, both(Script::default()));
    table.insert(
        wormhole().id,
        CardScripts {
            base: Script {
                triggers: vec![wormhole_trigger(false)],
                ..Script::default()
            },
            radiant: Script {
                triggers: vec![wormhole_trigger(true)],
                ..Script::default()
            },
        },
    );
    table.insert(
        boost().id,
        both(Script {
            targets: vec![TargetDecl {
                aim: Some(TargetAim::Help),
                ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))
            }],
            cry: Some(hook(|ctx| {
                let amount = param(&*ctx, "boost");
                vec![effects::buff(json_as(json!({
                    "target": { "of": "chosen" },
                    "attack": amount,
                    "health": amount,
                })))]
            })),
            ..Script::default()
        }),
    );
    table.insert(
        bolt().id,
        both(Script {
            cry: Some(hook(|_ctx| vec![ping(1)])),
            ..Script::default()
        }),
    );
    table.insert(rod().id, rod_script());
    table.insert(rod_field().id, rod_script());
    table
});

/// Merge this file's cards into whatever catalog and scripts the test registered first.
pub fn register_card_on_card() {
    let mut defs = registered_catalog().clone();
    for entry in COC_DEFS.iter() {
        defs.insert(entry.id.clone(), entry.clone());
    }
    register_catalog(defs);
    let mut all_scripts = registered_scripts().clone();
    all_scripts.extend(COC_SCRIPTS.clone());
    register_scripts(all_scripts);
}

// The verbs' tests' board helpers, shared by `card_on_card.rs` and `unit_text.rs` (two binaries)

/// p1's first main phase, both mulligans kept, ten mana a side and #1850's cards registered.
pub fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(&format!("coc-{seed}"), None)).state;
    register_card_on_card();
    for player in [P1, P2] {
        let keep: Vec<String> = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        state = act_ok(
            &state,
            json!({ "type": "mulligan", "keep": keep, "playerId": player }),
        )
        .state;
    }
    for player in [P1, P2] {
        state.players[player].auto_end_turn = Some(false);
        state.players[player].mana.current = 10;
        state.players[player].mana.max = 10;
    }
    state
}

/// Put `def_id` in `player`'s hand and play it, `extra` merged into the action.
pub fn play(state: &GameState, player: PlayerId, def_id: &str, radiant: bool, extra: Value) -> ReduceResult {
    let mut state = state.clone();
    let mut card = in_hand(&mut state, def_id, player, 1)
        .into_iter()
        .next()
        .expect("a card to play");
    if radiant {
        card.radiant = true;
        if let Some(live) = find_instance_mut(&mut state, &card.id) {
            live.radiant = true;
        }
    }
    let mut body = json!({ "type": "play", "instanceId": card.id, "playerId": player });
    if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            body.insert(key.clone(), value.clone());
        }
    }
    act_ok(&state, body)
}

pub fn at(row: &str, lane: i32, target: &str) -> Value {
    json!({
        "zone": { "row": row, "lane": lane },
        "targets": [{ "pick": "instance", "instanceId": target }],
    })
}

pub fn pass(state: &GameState) -> ReduceResult {
    act_ok(state, json!({ "type": "endTurn", "playerId": state.active }))
}

/// Apply one effect as `self_id`'s (none: no card) for `controller`, then settle, as a resolving
/// card's would.
pub fn apply(
    state: &mut GameState,
    self_id: Option<&str>,
    controller: PlayerId,
    effect: Effect,
) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed.clone(), state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        {
            let me = self_id.and_then(|id| find_instance(sink.state, id).cloned());
            let mut ctx = make_context(
                &mut sink,
                me.as_ref(),
                HookOptions {
                    controller: Some(controller),
                    ..Default::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        settle(&mut sink, SettleOptions::default());
    }
    state.rng_cursor = rng.cursor();
    events
}

pub fn instance(id: &str) -> Value {
    json!({ "of": "instance", "instanceId": id })
}

pub fn destroy(state: &mut GameState, id: &str) -> Vec<GameEvent> {
    apply(
        state,
        None,
        P1,
        effects::destroy(json_as(json!({ "target": instance(id) }))),
    )
}

pub fn top(state: &GameState, player: PlayerId, lane: i32) -> Option<CardInstance> {
    state.players[player]
        .units
        .get((lane - 1) as usize)?
        .as_ref()?
        .first()
        .cloned()
}

pub fn on_field(state: &GameState, id: &str) -> bool {
    find_instance(state, id).is_some_and(|card| matches!(card.zone, Zone::Field { .. }))
}

pub fn has(state: &GameState, id: &str, kind: KeywordKind) -> bool {
    let card = find_instance(state, id).cloned().expect("the card");
    unit_has(state, &card, kind)
}
