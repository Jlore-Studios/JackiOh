//! SPEC §8.12 S+ #121 Toxopid, a card of the Stinkyscape+ set (R1420, R3535000). Built by S13 (#3560).
//! **Float** reduces mana.
//! **Cry:** Summon 3 **Toxopid Legs** in random zones on the field, destroying the Units there
//! **Crater**
//! **Immutable**
//! **Indestructible** while **Toxopid Legs** are on the field
//! 10/10
//! **Rush**
//! **End of turn: Toxic 2** all enemies
//! \~\~\~
//! **Float** reduces mana.
//! **Cry:** Summon 4 **Radiant Toxopid Legs** in random zones on your opponent's field, destroying the Units there
//! **Crater**
//! **Immutable**
//! **Indestructible** while **Toxopid Legs** are on the field
//! 20/20
//! **Rush**
//! **End of turn: Toxic 4** all enemies
//!
//! R5048000: the Legs are Flexible, so the Cry draws its Legs' zones together as it resolves from every
//! unit and backrow zone of both sides (the Radiant face: the opponent's) that holds nothing or only
//! Units, the Toxopid's own excepted, no zone twice. Each drawn zone is crushed (the engine's `crush`:
//! every Unit there destroyed), one state check collects the deaths, and then a Leg is summoned into
//! each drawn zone that is free, standing in a backrow zone (`summon`'s `backrow`); with fewer zones
//! fewer Legs land, and no prompt opens. A Leg
//! belongs to the side it lands on and Craters. R3560006: Indestructible while any Leg stands. Float
//! and Toxic are the engine's. R4783021: a Taunt binds it as it binds any attacker.
//!
//! Issue #5048 made the Legs Flexible and let them crush; before it (`script_before_crushing_legs`,
//! R767000) the Cry landed them in random empty unit zones only (R4154300). Issue #4783 (Stinkyscape+
//! Balance Changes VIII) took "Ignores Taunt" off both faces; through v0.6.13h (`script_v0_6_13h`,
//! R767000) it carried the `ignoresTaunt` flag, so no Taunt bound it (R3560007). Issue #4154 placed the
//! Legs at random and took out its first-hand-arrival Crawler and the discount each friendly Crawler
//! death gave it. Through v0.6.6h (`script_v0_6_6h`, R767000) its first hand arrival each game added a
//! Crawler (R3560000), each friendly Crawler death took 1 off it in hand and deck (R3560004), and its
//! Cry opened one Mode prompt per Leg, each Leg exiling the pile below (R3560005).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-121";

/// The Crawler its arrival added through v0.6.6h.
const CRAWLER: &str = "stinkyscapeplus-120";
/// The Leg its Cry summons.
const LEG: &str = "stinkyscapeplus-121-1";
/// R3560000: the once-a-game hand-arrival mark (through v0.6.6h).
const HAND_KEY: &str = "stinkyscapeplus-121:hand";
/// R3560004: what each friendly Crawler death took off (through v0.6.6h).
const DISCOUNT: i32 = 1;
/// The v0.6.6h Cry's resume step.
const LEG_STEP: &str = "toxopidLeg";
/// The v0.6.6h Cry's data keys.
const LEFT_KEY: &str = "legsLeft";
const TAKEN_KEY: &str = "legsTaken";
/// A unit zone's key, `own-<lane>` or `enemy-<lane>`: the v0.6.6h Cry's Mode options and the zones
/// R4154300's draw names. R5048000's zones add their row: `own-units-<lane>`, `enemy-backrow-<lane>`.
const OWN: &str = "own";
const ENEMY: &str = "enemy";
/// R5048000: a zone key's row.
const UNITS: &str = "units";
const BACKROW: &str = "backrow";
/// R5048000: the Cry's part keys: each zone it crushes, then the zones its Legs land in.
const CRUSH: &str = "crush:";
const LAND: &str = "land:";

/// Which Cry a face runs: R5048000's, which crushes; R4154300's, into random empty unit zones (before
/// issue #5048); or R3560005's prompts (through v0.6.6h), with its hand and deck hooks.
#[derive(Clone, Copy, PartialEq)]
enum Era {
    Crushing,
    EmptyZones,
    Prompts,
}

/// The sides a Leg may land on, each with its zone keys' prefix: both on the base face, the opponent's
/// on the Radiant face.
fn sides(ctx: &EffectContext<'_>) -> Vec<(PlayerId, &'static str)> {
    let enemy = opponent_of(ctx.controller);
    if ctx.radiant {
        vec![(enemy, ENEMY)]
    } else {
        vec![(ctx.controller, OWN), (enemy, ENEMY)]
    }
}

/// R5048000: the zones a Leg may land in, each keyed by its side, row and lane: every unit and backrow
/// zone of `sides` a summon may land in, holding nothing or only Units (a GIGA covering it among them,
/// the engine's `crushed_in`), never one the Toxopid itself fills.
fn landing_zones(ctx: &EffectContext<'_>) -> Vec<String> {
    let Some(me) = ctx.live_self().cloned().or_else(|| ctx.self_.clone()) else {
        return vec![];
    };
    let mut zones: Vec<String> = Vec::new();
    for (side, prefix) in sides(ctx) {
        for (row, row_key) in [(Row::Units, UNITS), (Row::Backrow, BACKROW)] {
            for lane in 1..=row_size(row) {
                let slot = ZoneSlot {
                    player: side,
                    row,
                    lane,
                };
                if !exiling_summon_lands(ctx.state, &me, slot) {
                    continue;
                }
                let units = crushed_in(ctx.state, slot);
                let holds_only_units = zone_contents(ctx.state, slot).len() <= units.len();
                if holds_only_units && units.iter().all(|card| card.id != me.id) {
                    zones.push(format!("{prefix}-{row_key}-{lane}"));
                }
            }
        }
    }
    zones
}

/// A zone key's side, row and lane (`own-units-<lane>`, `enemy-backrow-<lane>`, …).
fn zone_parts(zone: &str) -> (&'static str, Row, i32) {
    let mut parts = zone.splitn(3, '-');
    let side = if parts.next() == Some(OWN) { "self" } else { "enemy" };
    let row = if parts.next() == Some(BACKROW) {
        Row::Backrow
    } else {
        Row::Units
    };
    let lane = parts.next().and_then(|lane| lane.parse::<i32>().ok()).unwrap_or(0);
    (side, row, lane)
}

/// R5048000: a Leg into the zone `zone` names, standing there in a backrow zone, Radiant on the Radiant
/// face.
fn leg_onto(zone: &str, radiant: bool) -> Effect {
    let (side, row, lane) = zone_parts(zone);
    summon(json_as(json!({
        "defId": LEG,
        "lane": lane,
        "player": side,
        "radiant": radiant,
        "backrow": row == Row::Backrow,
    })))
}

/// R5048000: one key of the Cry's part: crush the zone a `crush:` key names (the engine's `crush`), or,
/// once the state check has collected every crush, land a Leg in each zone the `land:` key names.
fn crush_or_land(key: &str, radiant: bool) -> Effect {
    if let Some(zone) = key.strip_prefix(CRUSH) {
        let (side, row, lane) = zone_parts(zone);
        return crush(json_as(json!({ "player": side, "row": row, "lane": lane })));
    }
    let zones: Vec<String> = key
        .strip_prefix(LAND)
        .unwrap_or_default()
        .split(',')
        .filter(|zone| !zone.is_empty())
        .map(str::to_string)
        .collect();
    after_state_check(move |_| zones.iter().map(|zone| leg_onto(zone, radiant)).collect())
}

/// R5048000: `legs` different zones of `landing_zones`, drawn together as the Cry reaches them; each is
/// crushed (the deaths collected together, R59), and after the state check a Leg lands in each.
fn crush_and_land(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let legs = param(ctx, "legs").max(0) as usize;
    let radiant = ctx.radiant;
    vec![for_each_card(ForEachCardArgs {
        cards: Arc::new(move |c: &mut EffectContext<'_>| -> Vec<String> {
            let zones = landing_zones(c);
            let drawn: Vec<String> = c.sink.rng.shuffle(&zones).into_iter().take(legs).collect();
            if drawn.is_empty() {
                return vec![];
            }
            let mut keys: Vec<String> = drawn.iter().map(|zone| format!("{CRUSH}{zone}")).collect();
            keys.push(format!("{LAND}{}", drawn.join(",")));
            keys
        }),
        each: Arc::new(move |key: &str| crush_or_land(key, radiant)),
    })]
}

/// R4154300 (before issue #5048): the empty unit zones a Leg may land in, each keyed by its side and
/// lane: both sides' on the base face, the opponent's on the Radiant face.
fn empty_zones(ctx: &EffectContext<'_>) -> Vec<String> {
    let Some(me) = ctx.live_self().cloned().or_else(|| ctx.self_.clone()) else {
        return vec![];
    };
    let mut zones: Vec<String> = Vec::new();
    for (side, prefix) in sides(ctx) {
        for lane in 1..=UNIT_ZONES {
            let slot = ZoneSlot {
                player: side,
                row: Row::Units,
                lane,
            };
            if jackioh_engine::zones::is_empty(ctx.state, slot) && exiling_summon_lands(ctx.state, &me, slot) {
                zones.push(format!("{prefix}-{lane}"));
            }
        }
    }
    zones
}

/// R4154300 (before issue #5048): a Leg into the zone `zone` names (`own-<lane>` or `enemy-<lane>`),
/// Radiant on the Radiant face.
fn leg_into(zone: &str, radiant: bool) -> Effect {
    let (side, lane) = zone.split_once('-').unwrap_or((ENEMY, "0"));
    let player = if side == OWN { "self" } else { "enemy" };
    summon(json_as(json!({
        "defId": LEG,
        "lane": lane.parse::<i32>().unwrap_or(0),
        "player": player,
        "radiant": radiant,
    })))
}

/// R4154300 (before issue #5048): `legs` different empty unit zones, drawn together as the Cry reaches
/// them, a Leg into each.
fn summon_legs(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let legs = param(ctx, "legs").max(0) as usize;
    let radiant = ctx.radiant;
    vec![for_each_card(ForEachCardArgs {
        cards: Arc::new(move |c: &mut EffectContext<'_>| -> Vec<String> {
            let zones = empty_zones(c);
            c.sink.rng.shuffle(&zones).into_iter().take(legs).collect()
        }),
        each: Arc::new(move |zone: &str| leg_into(zone, radiant)),
    })]
}

/// R3560004 (through v0.6.6h): a friendly Crawler death cheapens this in hand and deck.
fn crawler_died(id: &str) -> TriggerDef {
    TriggerDef::new(id, &[GameEventType::Destroyed], move |ctx, event| {
        let GameEvent::Destroyed {
            def_id, controller, ..
        } = event
        else {
            return vec![];
        };
        if *def_id != CRAWLER || *controller != ctx.controller {
            return vec![];
        }
        vec![set_cost_mod(json_as(json!({
            "target": { "of": "self" },
            "amount": -DISCOUNT,
        })))]
    })
}

/// R3560005 (through v0.6.6h): the prompt for one more Leg, `left` to go, `taken` already chosen.
fn ask_leg(ctx: &EffectContext<'_>, left: i32, taken: Vec<String>) -> Vec<Effect> {
    if left <= 0 {
        return vec![];
    }
    let Some(me) = ctx.live_self().cloned().or_else(|| ctx.self_.clone()) else {
        return vec![];
    };
    let mut options: Vec<String> = Vec::new();
    let mut labels: IndexMap<String, String> = IndexMap::new();
    for (side, prefix) in sides(ctx) {
        for lane in 1..=UNIT_ZONES {
            let slot = ZoneSlot {
                player: side,
                row: Row::Units,
                lane,
            };
            let key = format!("{prefix}-{lane}");
            if taken.contains(&key) {
                continue;
            }
            if let Some(found) = occupant_at(ctx.state, slot).cloned()
                && found.id == me.id
            {
                continue;
            }
            if !exiling_summon_lands(ctx.state, &me, slot) {
                continue;
            }
            let label = if side == ctx.controller {
                format!("Your lane {lane}")
            } else {
                format!("Enemy lane {lane}")
            };
            labels.insert(key.clone(), label);
            options.push(key);
        }
    }
    if options.is_empty() {
        return vec![];
    }
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(LEFT_KEY.to_string(), json!(left));
    data.insert(TAKEN_KEY.to_string(), json!(taken));
    vec![choose_mode(ChooseModeArgs {
        options,
        step: LEG_STEP.into(),
        prompt: Some("Choose where a Toxopid Leg lands".into()),
        data: Some(data),
        by: None,
        labels: Some(labels),
    })]
}

/// R3560007 (through v0.6.13h): "Ignores Taunt", the attacker's flag no Taunt binds.
fn ignoring_taunt(mut script: Script) -> Script {
    script.static_flags = Some(StaticFlags {
        ignores_taunt: Some(true),
        ..StaticFlags::default()
    });
    script
}

/// The face, its Cry `era`'s: today's crushes (R5048000), `EmptyZones` ran before issue #5048 and
/// `Prompts` through v0.6.6h; none ignores Taunt (`ignoring_taunt` gives the kept scripts through
/// v0.6.13h the flag).
fn toxopid(era: Era) -> Script {
    let mut script = Script {
        hand_triggers: vec![float_reduces_cost("toxopid-float")],
        conditional_keywords: Some(read_hook(|args| {
            if def_on_field(args.state, LEG) {
                vec![Keyword::Indestructible]
            } else {
                vec![]
            }
        })),
        end_of_turn: Some(hook(|ctx| {
            vec![toxic_all(json_as(json!({
                "amount": param(ctx, "toxic"),
                "side": "enemy",
                "heroes": true,
            })))]
        })),
        cry: Some(hook(if era == Era::Crushing {
            crush_and_land
        } else {
            summon_legs
        })),
        ..Script::default()
    };
    if era != Era::Prompts {
        return script;
    }
    script.enters_hand = Some(hook(move |ctx| {
        if once_a_game_used(ctx.state, ctx.controller, HAND_KEY) {
            return vec![];
        }
        vec![
            mark_once_a_game(json_as(json!({ "key": HAND_KEY }))),
            add_to_hand(json_as(json!({ "defId": CRAWLER }))),
        ]
    }));
    script.hand_triggers.push(crawler_died("toxopid-crawler-hand"));
    script.deck_triggers = vec![crawler_died("toxopid-crawler-deck")];
    script.cry = Some(hook(|ctx| ask_leg(ctx, param(ctx, "legs"), vec![])));
    script.resume = IndexMap::from([(
        LEG_STEP,
        hook(|ctx| {
            let Some(choice) = chosen_options(ctx).first().cloned() else {
                return vec![];
            };
            let (prefix, lane) = match choice.split_once('-') {
                Some((prefix, lane)) => (prefix, lane.parse::<i32>().unwrap_or(0)),
                None => return vec![],
            };
            let player = if prefix == OWN { "self" } else { "enemy" };
            let left = ctx.data.get(LEFT_KEY).and_then(Value::as_i64).unwrap_or(0) as i32;
            let mut taken: Vec<String> = ctx
                .data
                .get(TAKEN_KEY)
                .and_then(|value| serde_json::from_value::<Vec<String>>(value.clone()).ok())
                .unwrap_or_default();
            taken.push(choice);
            let mut out = vec![summon(json_as(json!({
                "defId": LEG,
                "lane": lane,
                "player": player,
                "exileBelow": true,
                "radiant": ctx.radiant,
            })))];
            out.extend(ask_leg(ctx, left - 1, taken));
            out
        }),
    )]);
    script
}

pub fn script() -> CardScripts {
    CardScripts {
        base: toxopid(Era::Crushing),
        radiant: toxopid(Era::Crushing),
    }
}

/// R767000: what it ran before issue #5048: its Cry landed the Legs in random empty unit zones only
/// (R4154300), crushing nothing.
pub fn script_before_crushing_legs() -> CardScripts {
    CardScripts {
        base: toxopid(Era::EmptyZones),
        radiant: toxopid(Era::EmptyZones),
    }
}

/// R767000: what it ran through v0.6.13h, before issue #4783: the Cry before issue #5048's, ignoring
/// Taunt (R3560007).
pub fn script_v0_6_13h() -> CardScripts {
    CardScripts {
        base: ignoring_taunt(toxopid(Era::EmptyZones)),
        radiant: ignoring_taunt(toxopid(Era::EmptyZones)),
    }
}

/// R767000: what it ran through v0.6.6h, before issue #4154: the first-hand-arrival Crawler, the
/// Crawler-death discount and the Leg prompts, ignoring Taunt (R3560007).
pub fn script_v0_6_6h() -> CardScripts {
    CardScripts {
        base: ignoring_taunt(toxopid(Era::Prompts)),
        radiant: ignoring_taunt(toxopid(Era::Prompts)),
    }
}

// S+ #121 Toxopid — SPEC §8.12 row 121.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::query::{hero_toxic_of, toxic_on};
    use jackioh_engine::subsystems::ai_policy::policy_actions;
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CRAWLER_DEF: &str = "stinkyscapeplus-120";
    const TOUGH: &str = "stand-in (1) 2/30";
    const FILLER: &str = "core-005";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const BODY: &str = "stand-in (0) 1/1";
    const TAUNT: &str = "stand-in (1) 2/30 Taunt";
    const TRAP: &str = "stand-in (1) Trap";
    /// How many seeds the random placement is watched over.
    const SEEDS: usize = 16;

    fn crawler_count(s: &Scenario) -> usize {
        s.hand(P1)
            .iter()
            .filter(|card| card.def_id == CRAWLER_DEF)
            .count()
    }

    /// The lanes of `player`'s side a Leg stands in.
    fn legs_of(s: &Scenario, player: PlayerId) -> Vec<i32> {
        (1..=UNIT_ZONES)
            .filter(|lane| s.unit(player, *lane).is_some_and(|unit| unit.def_id == LEG))
            .collect()
    }

    /// The lanes of `player`'s backrow a Leg stands in (R5048000).
    fn backrow_legs_of(s: &Scenario, player: PlayerId) -> Vec<i32> {
        (1..=BACKROW_ZONES)
            .filter(|lane| s.backrow(player, *lane).is_some_and(|card| card.def_id == LEG))
            .collect()
    }

    /// Every Leg on the field, either side, either row.
    fn all_legs(s: &Scenario) -> usize {
        [P1, P2]
            .iter()
            .map(|player| legs_of(s, *player).len() + backrow_legs_of(s, *player).len())
            .sum()
    }

    /// `player`'s units in `lanes`, each `def` (R5048000's boards).
    fn units_in(def: &str, lanes: std::ops::RangeInclusive<i32>) -> Value {
        json!(lanes.map(|lane| json!({ "def": def, "lane": lane })).collect::<Vec<_>>())
    }

    #[test]
    fn r5048000_the_cry_crushes_the_units_where_its_legs_land_on_either_side_and_row() {
        let (mut own, mut theirs, mut backrow, mut crushed) = (0, 0, 0, 0);
        for n in 0..SEEDS {
            let mut s = weapon_board(
                &format!("s121-crush-{n}"),
                json!({ "mana": 10, "hand": [ID], "field": units_in(BODY, 2..=3) }),
                json!({ "field": units_in(VANILLA, 1..=5) }),
            );
            let before: Vec<(PlayerId, i32, String)> = [(P1, 2..=3), (P2, 1..=5)]
                .into_iter()
                .flat_map(|(player, lanes)| lanes.map(move |lane| (player, lane)))
                .map(|(player, lane)| (player, lane, s.unit_at(player, lane).id))
                .collect();
            s.play(ID, json!({ "zone": 1 }));
            assert!(s.state().pending.is_none(), "seed {n}: no prompt opens");
            assert_eq!(s.unit_at(P1, 1).def_id, ID, "seed {n}: never its own zone");
            assert_eq!(all_legs(&s), 3, "seed {n}: three Legs");
            // A Leg stands where a Unit stood only once that Unit was destroyed; every other Unit stands.
            for (player, lane, id) in &before {
                if legs_of(&s, *player).contains(lane) {
                    s.expect_in_zone(id.as_str(), "graveyard");
                    crushed += 1;
                } else {
                    assert_eq!(s.unit_at(*player, *lane).id, *id, "seed {n}");
                }
            }
            assert!(s.pile(P1, "exile").is_empty() && s.pile(P2, "exile").is_empty(), "seed {n}: destroyed");
            // A Leg in the backrow stands there as a Unit (it is Flexible) and Craters that zone.
            for player in [P1, P2] {
                for lane in backrow_legs_of(&s, player) {
                    let leg = s.backrow(player, lane).expect("a Leg");
                    assert!(!leg.radiant, "seed {n}: base Legs");
                    assert!(in_units_row_or_standing(s.state(), &leg), "seed {n}: standing");
                    let slot = ZoneSlot {
                        player,
                        row: Row::Backrow,
                        lane,
                    };
                    assert!(is_locked(s.state(), slot), "seed {n}: Crater");
                }
            }
            own += legs_of(&s, P1).len() + backrow_legs_of(&s, P1).len();
            theirs += legs_of(&s, P2).len() + backrow_legs_of(&s, P2).len();
            backrow += backrow_legs_of(&s, P1).len() + backrow_legs_of(&s, P2).len();
        }
        assert!(own > 0 && theirs > 0, "both sides take Legs over the seeds: {own} own, {theirs} enemy");
        assert!(backrow > 0, "a Leg stands in a backrow zone on some seed");
        assert!(crushed > 0, "a Leg crushes a Unit on some seed");
    }

    #[test]
    fn r5048000_radiant_crushes_only_the_opponents_units_and_passes_over_zones_holding_traps() {
        for n in 0..SEEDS {
            let mut s = weapon_board(
                &format!("s121-radiant-crush-{n}"),
                json!({
                    "mana": 10,
                    "hand": [{ "def": ID, "radiant": true }],
                    "field": [{ "def": BODY, "lane": 2 }],
                }),
                json!({ "field": units_in(VANILLA, 1..=5), "backrow": units_in(TRAP, 1..=5) }),
            );
            let traps: Vec<String> = (1..=BACKROW_ZONES)
                .map(|lane| s.backrow(P2, lane).expect("a Trap").id)
                .collect();
            let body = s.unit_at(P1, 2).id;
            s.play(ID, json!({ "zone": 1 }));
            assert!(s.state().pending.is_none());
            // Every enemy backrow zone holds a Trap, so only its five unit zones may be drawn: four of them.
            assert!(legs_of(&s, P1).is_empty() && backrow_legs_of(&s, P1).is_empty(), "seed {n}");
            assert!(backrow_legs_of(&s, P2).is_empty(), "seed {n}: no Trap is crushed");
            let legs = legs_of(&s, P2);
            assert_eq!(legs.len(), 4, "seed {n}");
            assert!(legs.iter().all(|lane| s.unit_at(P2, *lane).radiant), "seed {n}: Radiant Legs");
            let left = (1..=UNIT_ZONES)
                .filter(|lane| s.unit_at(P2, *lane).def_id == VANILLA)
                .count();
            assert_eq!(left, 1, "seed {n}: one enemy Unit was not drawn");
            assert_eq!(s.unit_at(P1, 2).id, body, "seed {n}: its own side untouched");
            for (lane, trap) in (1..=BACKROW_ZONES).zip(&traps) {
                assert_eq!(s.backrow(P2, lane).expect("the Trap").id, *trap, "seed {n}");
            }
        }
    }

    #[test]
    fn r5048000_a_unit_its_crush_cannot_destroy_keeps_its_zone_and_no_leg_lands_there() {
        const WALL: &str = "stand-in (1) 2/30 Indestructible";
        let mut blocked = 0;
        for n in 0..SEEDS {
            let mut s = weapon_board(
                &format!("s121-wall-{n}"),
                json!({ "mana": 10, "hand": [{ "def": ID, "radiant": true }] }),
                json!({
                    "field": [
                        { "def": WALL, "lane": 1 },
                        { "def": VANILLA, "lane": 2 },
                        { "def": VANILLA, "lane": 3 },
                        { "def": VANILLA, "lane": 4 },
                        { "def": VANILLA, "lane": 5 },
                    ],
                    "backrow": units_in(TRAP, 1..=5),
                }),
            );
            let vanillas: Vec<String> = (2..=UNIT_ZONES).map(|lane| s.unit_at(P2, lane).id).collect();
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(s.unit_at(P2, 1).def_id, WALL, "seed {n}: the Indestructible Unit stands");
            let legs = legs_of(&s, P2);
            assert!(!legs.contains(&1), "seed {n}: no Leg lands on it");
            let destroyed = vanillas
                .iter()
                .filter(|id| s.card(id.as_str()).zone.z() == ZoneName::Graveyard)
                .count();
            assert_eq!(legs.len(), destroyed, "seed {n}: a Leg where each crush freed the zone");
            if legs.len() == 3 {
                blocked += 1;
            } else {
                assert_eq!(legs.len(), 4, "seed {n}");
            }
        }
        assert!(blocked > 0, "the Indestructible Unit's zone is drawn on some seed");
    }

    #[test]
    fn r5048000_a_backrow_summon_stands_a_flexible_unit_and_nothing_else() {
        let mut s = weapon_board(
            "s121-backrow-summon",
            json!({ "backrow": [{ "def": TRAP, "lane": 3 }] }),
            json!({}),
        );
        let summon_at = |def: &str, lane: i32| {
            summon(json_as(json!({ "defId": def, "lane": lane, "player": "self", "backrow": true })))
        };
        crate::run_as(&mut s, summon_at(LEG, 2), P1);
        let leg = s.backrow(P1, 2).expect("the Flexible Leg stands in backrow lane 2");
        assert_eq!(leg.def_id, LEG);
        assert!(in_units_row_or_standing(s.state(), &leg));
        let slot = ZoneSlot {
            player: P1,
            row: Row::Backrow,
            lane: 2,
        };
        assert!(is_locked(s.state(), slot), "it Craters its backrow zone");
        // A Unit that is not Flexible lands nowhere, nor does a Leg on the Trap's zone.
        crate::run_as(&mut s, summon_at(BODY, 4), P1);
        assert!(s.backrow(P1, 4).is_none() && s.unit(P1, 4).is_none());
        crate::run_as(&mut s, summon_at(LEG, 3), P1);
        assert_eq!(s.backrow(P1, 3).expect("the Trap").def_id, TRAP);
        assert_eq!(all_legs(&s), 1);
        // Flying is Flexible innately.
        crate::run_as(&mut s, summon_at("stand-in (1) 2/2 Flying", 5), P1);
        assert!(s.backrow(P1, 5).is_some(), "a Flying Unit stands there too");
    }

    #[test]
    fn r4154300_its_hand_arrival_adds_no_crawler_and_a_crawler_death_leaves_its_cost() {
        let mut s = weapon_board(
            "s121-no-crawler",
            json!({ "hand": [FILLER], "field": [{ "def": CRAWLER_DEF, "lane": 1 }], "library": filler(4) }),
            json!({ "field": [VANILLA], "library": filler(4) }),
        );
        crate::run_as(
            &mut s,
            jackioh_engine::effects::add_to_hand(json_as(json!({ "defId": ID }))),
            P1,
        );
        assert_eq!(crawler_count(&s), 0, "no Crawler comes with it");
        let toxopid = s
            .hand(P1)
            .into_iter()
            .find(|card| card.def_id == ID)
            .expect("in hand");
        // A friendly Crawler dies on the enemy's turn: the Crawler's own Death may cheapen it (R4154301),
        // but the Toxopid itself takes nothing off for it.
        crate::next_turn_of(&mut s, P2);
        let before = s.card(&toxopid.id).cost_mod;
        let killer = s.unit_at(P2, 1).id;
        let victim = s.unit_at(P1, 1).id;
        s.attack(killer.as_str(), victim.as_str());
        s.expect_in_zone(victim.as_str(), "graveyard");
        let reduce = param_value(s.state(), Some(s.card(&victim)), "reduce", ParamValueOptions::default());
        assert_eq!(s.card(&toxopid.id).cost_mod, before - reduce, "the Crawler's reduction alone");
    }

    #[test]
    fn float_reduces_its_cost() {
        let mut s = weapon_board(
            "s121-float",
            json!({ "mana": 5, "hand": [ID], "library": filler(4) }),
            json!({ "library": filler(4) }),
        );
        let before = play_cost(s.state(), s.card(ID));
        s.end_turn();
        s.end_turn();
        let after = play_cost(s.state(), s.card(ID));
        assert!(after < before, "a Float took something off");
    }

    #[test]
    fn r3560006_indestructible_while_a_leg_stands() {
        let mut s = weapon_board(
            "s121-indestructible",
            json!({ "mana": 10, "hand": [ID], "library": filler(4) }),
            json!({ "hand": [FILLER], "field": [TOUGH], "library": filler(4) }),
        );
        s.play(ID, json!({ "zone": 1 }));
        assert_eq!(all_legs(&s), 3);
        assert!(unit_has(s.state(), s.card(ID), KeywordKind::Indestructible));
        // Without a Leg it is destructible: a fresh board has none.
        let s = weapon_board("s121-mortal", json!({ "field": [ID] }), json!({}));
        assert!(!unit_has(s.state(), s.card(ID), KeywordKind::Indestructible));
    }

    /// A Toxopid of `radiant`'s face against a Taunt in lane 1 and a plain Unit beside it:
    /// whether it may attack the hero, the plain Unit and the Taunt.
    fn past_taunt(seed: &str, radiant: bool) -> (bool, bool, bool) {
        let s = weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": [{ "def": TAUNT, "lane": 1 }, VANILLA] }),
        );
        let toxopid = s.unit_at(P1, 1);
        let (taunt, plain) = (s.unit_at(P2, 1), s.unit_at(P2, 2));
        (
            can_attack(s.state(), &toxopid, &DamageTarget::Hero { player: P2 }),
            can_attack(s.state(), &toxopid, &DamageTarget::Unit { instance: plain }),
            can_attack(s.state(), &toxopid, &DamageTarget::Unit { instance: taunt }),
        )
    }

    #[test]
    fn r4783021_a_taunt_binds_it_on_both_faces() {
        // Neither the hero nor the plain Unit: the Taunt alone.
        for radiant in [false, true] {
            assert_eq!(past_taunt("s121-taunt", radiant), (false, false, true), "radiant {radiant}");
        }
    }

    #[test]
    fn end_of_turn_gives_every_enemy_toxic() {
        for radiant in [false, true] {
            let toxic = if radiant { 4 } else { 2 };
            let mut s = weapon_board(
                "s121-toxic",
                json!({ "field": [{ "def": ID, "radiant": radiant }] }),
                // A sturdy enemy: Toxic 4 ticks at its owner's turn start and would kill a 4/4.
                json!({ "field": ["stand-in (1) 10/10 R20/20"], "library": filler(4) }),
            );
            s.end_turn();
            let enemy = s.unit_at(P2, 1).id;
            assert_eq!(toxic_on(s.card(&enemy)), toxic, "radiant {radiant}");
            assert_eq!(hero_toxic_of(s.state(), P2), toxic, "radiant {radiant}");
        }
    }

    #[test]
    fn r5117000_a_spell_running_its_cry_crushes_no_unit_immune_to_spells_and_lands_no_leg_there() {
        const IMMUNE: &str = "stand-in (1) 4/4 Immune to Spells";
        const KEPT: &str = "stinkyscape-126";
        let (mut spared, mut crushed) = (0, 0);
        for n in 0..SEEDS {
            let mut s = weapon_board(
                &format!("s121-spell-{n}"),
                json!({
                    "mana": 10,
                    "hand": [KEPT],
                    "field": units_in(IMMUNE, 1..=5),
                    "backrow": units_in(TRAP, 1..=5),
                }),
                json!({ "field": units_in(VANILLA, 1..=5), "backrow": units_in(TRAP, 1..=5) }),
            );
            // The issue's fusion: the Toxopid fused into a Spell in hand, which keeps the Spell's type (R77).
            let spell = s.card(KEPT).id.clone();
            let fuse = fuse_cards(json_as(json!({ "defIds": [ID], "intoInstanceId": spell })));
            crate::run_as(&mut s, fuse, P1);
            assert_eq!(card_type_of(s.state(), s.card(spell.as_str())), CardType::Spell, "seed {n}");
            let immune: Vec<String> = (1..=UNIT_ZONES).map(|lane| s.unit_at(P1, lane).id).collect();
            let vanillas: Vec<String> = (1..=UNIT_ZONES).map(|lane| s.unit_at(P2, lane).id).collect();
            s.play(spell.as_str(), json!({}));
            assert!(s.state().pending.is_none(), "seed {n}: no prompt opens");
            for (lane, id) in (1..=UNIT_ZONES).zip(&immune) {
                assert_eq!(s.unit_at(P1, lane).id, *id, "seed {n}: the Spell passes it by");
            }
            assert!(legs_of(&s, P1).is_empty() && backrow_legs_of(&s, P1).is_empty(), "seed {n}");
            let destroyed = vanillas
                .iter()
                .filter(|id| s.card(id.as_str()).zone.z() == ZoneName::Graveyard)
                .count();
            assert_eq!(legs_of(&s, P2).len(), destroyed, "seed {n}: a Leg where each crush freed the zone");
            spared += 3 - destroyed;
            crushed += destroyed;
        }
        assert!(spared > 0, "an immune Unit's zone is drawn on some seed");
        assert!(crushed > 0, "a Unit not immune is crushed on some seed");
    }

    /// R767000: the script kept for the replays from before issue #5048, whose Cry landed its Legs in
    /// random empty unit zones only (R4154300).
    mod kept_before_crushing_legs {
        use super::*;

        #[test]
        fn r767000_r4154300_the_cry_landed_three_legs_in_empty_zones_of_either_side_with_no_prompt() {
            crate::under_kept(ID, script_before_crushing_legs(), || {
                let (mut own, mut theirs) = (0, 0);
                for n in 0..SEEDS {
                    let mut s = weapon_board(
                        &format!("s121-random-{n}"),
                        json!({ "mana": 10, "hand": [ID], "library": filler(4) }),
                        json!({ "field": [{ "def": VANILLA, "lane": 2 }], "library": filler(4) }),
                    );
                    let vanilla = s.unit_at(P2, 2).id;
                    s.play(ID, json!({ "zone": 1 }));
                    assert!(s.state().pending.is_none(), "seed {n}: no prompt opens");
                    let (mine, enemy) = (legs_of(&s, P1), legs_of(&s, P2));
                    assert_eq!(mine.len() + enemy.len(), 3, "seed {n}: three Legs");
                    // Only empty zones: never the Toxopid's own, never the Vanilla's, which still stands.
                    assert!(!mine.contains(&1) && !enemy.contains(&2), "seed {n}");
                    assert_eq!(s.unit_at(P1, 1).def_id, ID);
                    assert_eq!(s.unit_at(P2, 2).id, vanilla, "seed {n}: nothing exiled");
                    assert!(s.pile(P2, "exile").is_empty(), "seed {n}");
                    let legs: Vec<CardInstance> = mine
                        .iter()
                        .map(|lane| s.unit_at(P1, *lane))
                        .chain(enemy.iter().map(|lane| s.unit_at(P2, *lane)))
                        .collect();
                    assert!(legs.iter().all(|leg| !leg.radiant), "seed {n}: base Legs");
                    own += mine.len();
                    theirs += enemy.len();
                }
                assert!(own > 0 && theirs > 0, "both sides take Legs over the seeds: {own} own, {theirs} enemy");
            });
        }

        #[test]
        fn r767000_r4154300_radiant_landed_four_radiant_legs_in_the_opponents_empty_zones_only() {
            crate::under_kept(ID, script_before_crushing_legs(), || {
                for n in 0..SEEDS {
                    let mut s = weapon_board(
                        &format!("s121-radiant-{n}"),
                        json!({ "mana": 10, "hand": [{ "def": ID, "radiant": true }], "library": filler(4) }),
                        json!({ "field": [{ "def": VANILLA, "lane": 1 }], "library": filler(4) }),
                    );
                    s.play(ID, json!({ "zone": 1 }));
                    assert!(s.state().pending.is_none());
                    assert!(legs_of(&s, P1).is_empty(), "seed {n}: none on its own side");
                    // Four empty zones on the opponent's side, so all four take one.
                    assert_eq!(legs_of(&s, P2), vec![2, 3, 4, 5], "seed {n}");
                    for lane in 2..=5 {
                        assert!(s.unit_at(P2, lane).radiant, "seed {n}: Radiant Legs");
                    }
                    assert_eq!(s.unit_at(P2, 1).def_id, VANILLA);
                }
            });
        }

        #[test]
        fn r767000_r4154300_fewer_empty_zones_landed_fewer_legs() {
            crate::under_kept(ID, script_before_crushing_legs(), || {
                let mut s = weapon_board(
                    "s121-crowded",
                    json!({
                        "mana": 10,
                        "hand": [ID],
                        "field": [
                            { "def": BODY, "lane": 2 },
                            { "def": BODY, "lane": 3 },
                            { "def": BODY, "lane": 4 },
                            { "def": BODY, "lane": 5 },
                        ],
                        "library": filler(4),
                    }),
                    json!({
                        "field": [
                            { "def": BODY, "lane": 1 },
                            { "def": BODY, "lane": 2 },
                            { "def": BODY, "lane": 3 },
                            { "def": BODY, "lane": 4 },
                        ],
                        "library": filler(4),
                    }),
                );
                s.play(ID, json!({ "zone": 1 }));
                assert!(s.state().pending.is_none());
                // One empty zone on the whole field: one Leg, the other two land nowhere.
                assert!(legs_of(&s, P1).is_empty());
                assert_eq!(legs_of(&s, P2), vec![5]);
                assert!(s.pile(P1, "exile").is_empty() && s.pile(P2, "exile").is_empty());
            });
        }
    }

    /// R767000: the script kept for the replays of v0.6.13h and before (issue #4783).
    mod kept_v0_6_13h {
        use super::*;

        #[test]
        fn r767000_r3560007_it_ignored_taunt_on_both_faces() {
            crate::under_kept(ID, script_v0_6_13h(), || {
                for radiant in [false, true] {
                    assert_eq!(past_taunt("s121-kept-taunt", radiant), (true, true, true), "radiant {radiant}");
                }
            });
        }

        #[test]
        fn r767000_r4154300_its_cry_still_landed_its_legs_at_random() {
            crate::under_kept(ID, script_v0_6_13h(), || {
                let mut s = weapon_board(
                    "s121-kept-cry",
                    json!({ "mana": 10, "hand": [ID], "library": filler(4) }),
                    json!({ "library": filler(4) }),
                );
                s.play(ID, json!({ "zone": 1 }));
                assert!(s.state().pending.is_none(), "no prompt opens");
                assert_eq!(legs_of(&s, P1).len() + legs_of(&s, P2).len(), 3);
            });
        }
    }

    /// R767000: the script kept for the replays of v0.6.6h and before (issue #4154).
    mod kept_v0_6_6h {
        use super::*;

        #[test]
        fn r767000_r3560007_it_ignored_taunt_through_v0_6_6h_too() {
            crate::under_kept(ID, script_v0_6_6h(), || {
                assert_eq!(past_taunt("s121-kept-6h-taunt", false), (true, true, true));
            });
        }

        #[test]
        fn r767000_r3560000_its_first_hand_arrival_added_a_crawler_once() {
            crate::under_kept(ID, script_v0_6_6h(), || {
                for radiant in [false, true] {
                    let mut s = weapon_board(
                        "s121-hand",
                        json!({ "hand": [{ "def": ID, "radiant": radiant }], "library": filler(4) }),
                        json!({ "library": filler(4) }),
                    );
                    // A scenario's hand is set up, not dealt: it arrives nothing, so no Crawler yet.
                    assert_eq!(crawler_count(&s), 0, "radiant {radiant}: a set-up hand is no arrival");
                    let add = || {
                        jackioh_engine::effects::add_to_hand(json_as(json!({ "defId": ID, "radiant": radiant })))
                    };
                    crate::run_as(&mut s, add(), P1);
                    assert_eq!(crawler_count(&s), 1, "radiant {radiant}: its first arrival adds one");
                    crate::run_as(&mut s, add(), P1);
                    assert_eq!(crawler_count(&s), 1, "radiant {radiant}: once per game");
                }
            });
        }

        #[test]
        fn r767000_r3560004_a_friendly_crawler_dying_cheapened_it_in_the_deck() {
            crate::under_kept(ID, script_v0_6_6h(), || {
                let mut s = weapon_board(
                    "s121-discount",
                    json!({
                        "hand": [FILLER],
                        "field": [{ "def": CRAWLER_DEF, "lane": 1 }],
                        "library": [{ "def": ID }, FILLER, FILLER, FILLER],
                    }),
                    json!({ "field": [VANILLA], "library": filler(4) }),
                );
                crate::next_turn_of(&mut s, P2);
                let in_deck = |s: &Scenario| {
                    s.pile(P1, "library")
                        .into_iter()
                        .find(|card| card.def_id == ID)
                        .expect("in deck")
                };
                let before = play_cost(s.state(), &in_deck(&s));
                let killer = s.unit_at(P2, 1).id;
                let victim = s.unit_at(P1, 1).id;
                s.attack(killer.as_str(), victim.as_str());
                s.expect_in_zone(victim.as_str(), "graveyard");
                assert_eq!(play_cost(s.state(), &in_deck(&s)), (before - DISCOUNT).max(0));
            });
        }

        #[test]
        fn r767000_r3560005_its_cry_asked_where_each_leg_lands() {
            crate::under_kept(ID, script_v0_6_6h(), || {
                let mut s = weapon_board(
                    "s121-cry",
                    json!({ "mana": 10, "hand": [ID], "library": filler(4) }),
                    json!({ "field": [{ "def": VANILLA, "lane": 4 }], "library": filler(4) }),
                );
                s.play(ID, json!({ "zone": 1 }));
                let pending = s.state().pending.clone().expect("a Leg prompt");
                let offered: Vec<String> = pending
                    .options
                    .iter()
                    .map(|option| option.key.trim_start_matches("mode:").to_string())
                    .collect();
                assert!(!offered.contains(&"own-1".to_string()), "its own zone");
                assert!(
                    policy_actions(s.state(), P1, Default::default())
                        .iter()
                        .any(|action| matches!(action, ActionBody::Answer { .. })),
                    "the AI policy answers"
                );
                s.answer(json!(["enemy-4"]));
                s.answer(json!(["own-2"]));
                s.answer(json!(["own-3"]));
                assert!(s.state().pending.is_none());
                assert_eq!(legs_of(&s, P1), vec![2, 3]);
                assert_eq!(legs_of(&s, P2), vec![4]);
                // The enemy's Leg exiled the Unit below it.
                assert!(s.pile(P2, "exile").iter().any(|card| card.def_id == VANILLA));
            });
        }
    }
}
