//! #22 Carnivorous Cube (SPEC §8.2, R41, R57, R64, R81, R428).
//!
//! Base: "Cry: Tribute one of your other Units and remember it. Death: Summon 2 copies of it."
//! Radiant: "Cry: Tribute one of your other Units and remember it. Death: Fill your board with copies
//! of it." The Radiant face changes the Death clause only, so the Cry — the tribute and the
//! remembering — is the same on both (§8 Conventions).
//!
//! R428 (patch v0.2.0, rewrites R41's "any other permanent, backrow included"): the Cry eats one of
//! your other UNITS only — a Unit acting in a unit zone, never a backrow card. §6.3 Tribute: "a card
//! whose own text tributes (Carnivorous Cube) sacrifices what that text names instead … A tribute
//! written into a card's script is an ordinary Sacrifice of the permanent that script names, where the
//! Sheep Token's 2 never applies." So the meal is picked with the play (R81, a `tribute` target the
//! play action carries, never a prompt) and eaten with `sacrifice`, which bypasses Indestructible and
//! counts as a death.
//!
//! What is remembered is `memory.eaten = { defId, radiant, statsOverride?, armorOverride? }` (§10.1):
//! R41 keeps the eaten card's radiant flag and `statsOverride` (with §7's `armorOverride` beside it)
//! on every copy. R41's two fizzles are one condition each: nothing to tribute → the Cry does nothing
//! and remembers nothing; nothing eaten → Death does nothing. It can never eat itself: the declared
//! target excludes it and the hook re-checks.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-022";

/// `memory.eaten`, plain JSON so the state survives a replay round-trip (§9.3, §10.1).
#[derive(Clone, Debug)]
struct Eaten {
    def_id: String,
    /// R41, R57: a copy of a Radiant meal is Radiant.
    radiant: bool,
    /// R41, R57: a token eaten with §7 stats copies with those stats.
    stats_override: Option<AttackHealth>,
    /// §7: a Bread Token's "Armor X" is the other half of its X/X, so a copy keeps it beside them.
    armor_override: Option<i32>,
}

impl Eaten {
    /// The meal as `memory.eaten` stores it: `{ defId, radiant, statsOverride?, armorOverride? }`.
    fn to_json(&self) -> Value {
        let mut value = json!({ "defId": self.def_id, "radiant": self.radiant });
        if let Some(stats) = self.stats_override {
            value["statsOverride"] = json!({ "attack": stats.attack, "health": stats.health });
        }
        if let Some(armor) = self.armor_override {
            value["armorOverride"] = json!(armor);
        }
        value
    }
}

const EATEN: &str = "eaten";

/// R81: the meal travels in the `play` action, as a `tribute` pick the client shows as one. R428: one
/// of your other Units — the unit row only.
///
/// `min: 1` is R41's "must eat if able", and R90 supplies the "if able": a declaration the board
/// cannot satisfy "does not refuse the play — the play is legal with the answers that exist and the
/// effect fizzles on resolution", which is exactly R41's "nothing to tribute → Cry fizzles".
///
/// Deliberately no `amount`: `playChoices.tributeCostOf` reads a `tribute` declaration's `amount` as
/// §6.3's Tribute *cost*, which is paid with the play action's `tributes` list, counts Sheep Tokens
/// as 2 and refuses the play when the board cannot pay it (#66). §6.3 says the opposite for this card
/// — "a card whose own text tributes (Carnivorous Cube) sacrifices what that text names instead …
/// where the Sheep Token's 2 never applies" — so the meal is a declared target the script sacrifices
/// itself, and the play carries no Tribute cost.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::tribute(
        1,
        1,
        json!({ "side": "ally", "of": ["unit"], "excludeSelf": true }),
    )]
}

/// Read the named permanent off the play's selection; null when there was nothing legal to eat. The
/// pick is read as the engine aims every chosen target (`instanceOf`): on the stay the play chose it
/// on (R174) and acting on the field (§3.2, R13). A crafted Cube + Cube whose two parts name the same
/// Reborn unit eats it once — the second part finds the Reborn body, a new arrival, and remembers
/// nothing — and a meal a Stack play buried under the crafted card is no meal (R41).
fn meal_of(ctx: &EffectContext<'_>) -> Option<Eaten> {
    let selection = ctx.targets.first()?;
    if !matches!(selection, Selection::Instance { .. }) {
        return None;
    }

    let card = instance_of(ctx, &TargetSpec::Chosen { index: None })?;
    // R428: a Unit, acting in a unit zone — never a backrow card.
    if !matches!(card.zone, Zone::Field { row: Row::Units, .. }) {
        return None;
    }
    // "One of your other Units": ally only, and R41's "cannot eat itself".
    if card.controller != ctx.controller {
        return None;
    }
    if ctx.self_.as_ref().is_some_and(|self_| card.id == self_.id) {
        return None;
    }

    Some(Eaten {
        def_id: card.def_id.clone(),
        radiant: card.radiant,
        stats_override: card.stats_override,
        armor_override: card.armor_override,
    })
}

fn eaten_of(ctx: &EffectContext<'_>) -> Option<Eaten> {
    // `recalled` reads what this Cube remembered — on a fused card, this ingredient's own meal (R102).
    let stored = recalled(ctx, EATEN)?;
    let value = stored.as_object()?;
    let def_id = value.get("defId").and_then(Value::as_str)?.to_string();
    Some(Eaten {
        def_id,
        radiant: value.get("radiant").and_then(Value::as_bool) == Some(true),
        stats_override: value
            .get("statsOverride")
            .and_then(|stats| serde_json::from_value::<AttackHealth>(stats.clone()).ok()),
        armor_override: value
            .get("armorOverride")
            .and_then(Value::as_i64)
            .map(|armor| armor as i32),
    })
}

/// The `summon`/`fillBoard` arguments that copy the meal: R57's flags, its §7 stats and Armor.
fn copy_args(eaten: &Eaten) -> Value {
    let mut args = json!({ "defId": eaten.def_id, "radiant": eaten.radiant });
    if let Some(stats) = eaten.stats_override {
        args["statsOverride"] = json!({ "attack": stats.attack, "health": stats.health });
    }
    if let Some(armor) = eaten.armor_override {
        args["armorOverride"] = json!(armor);
    }
    args
}

/// One copy of the meal: R57's flags, and R64 puts it in the leftmost free zone of its own row.
fn copy_of(eaten: &Eaten) -> Effect {
    summon(json_as(copy_args(eaten)))
}

/// Remember the meal, then eat it — the effects apply in this order, so the read happens first.
fn cry() -> Hook {
    hook(|ctx| {
        let Some(meal) = meal_of(ctx) else {
            return vec![]; // R41: nothing to tribute → the Cry fizzles.
        };
        vec![
            remember(json_as(json!({ "key": EATEN, "value": meal.to_json() }))),
            sacrifice(json_as(json!({ "target": { "of": "chosen" } }))),
        ]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: targets(),
        cry: Some(cry()),
        death: Some(hook(|ctx| {
            let Some(eaten) = eaten_of(ctx) else {
                return vec![]; // R41: nothing eaten → Death does nothing.
            };
            vec![copy_of(&eaten), copy_of(&eaten)]
        })),
        ..Script::default()
    };

    let radiant = Script {
        targets: targets(),
        cry: Some(cry()),
        death: Some(hook(|ctx| {
            let Some(eaten) = eaten_of(ctx) else {
                return vec![];
            };
            // R64: "fill your board" summons into every empty, unlocked unit zone left to right.
            if def_of(Some(&*ctx.state), &eaten.def_id).type_ == CardType::Unit {
                return vec![fill_board(json_as(copy_args(&eaten)))];
            }
            // A meal that stood in a unit zone without being a Unit card — a Field Spell or Trap standing
            // there animated (B3.1, R383) — is copied as its own card: `fillBoard` makes Units only, so the
            // board is filled with one laneless `summon` per unit zone, each placed where its own type goes
            // (R64), and the ones with no zone left fizzle.
            (0..UNIT_ZONES).map(|_| copy_of(&eaten)).collect()
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// #22 Carnivorous Cube — SPEC §8.2, BUILD M4-T4 row 22, as patch v0.2.0 rewrites it (R428): "The
// Tribute choice travels in the play action (R81), one of your other Units only, excluding itself;
// chosen Unit sacrificed and remembered; Death → 2 copies (radiant fills board), copies keep
// `statsOverride` (R41); nothing eaten → Death does nothing (R41)". A backrow permanent is no meal.
//
// The base Cube is 4/6, so one 7-attack hit kills it. The radiant Cube is 8/12, so it takes a 7 and
// a 6 in the same turn; Bigot (6/1) dies to the strike-back, which is not what any assertion reads.
//
// HARNESS GAP: `FieldSetup` has no `statsOverride`, so §7 stats cannot be seeded. The one fixture
// that needs an eaten card with them sets the field instance's `statsOverride` itself, the way
// `015-me-and-mr-token.test.ts` sets a radiant flag it cannot seed.
#[cfg(test)]
mod tests {
    use super::script;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const CUBE: &str = "core-022"; // 4/6, radiant 8/12, cost 3.
    const TIMMY: &str = "core-011"; // Tempo Timmy, a plain 3/3 unit.
    const MANA_WELL: &str = "core-006"; // A Field Spell: a backrow permanent.
    const HITTER: &str = "core-025"; // 4-mana 7/7.
    const BIGOT: &str = "core-002"; // 6/1: the second hit that finishes the radiant Cube.
    const FILLER: &str = "core-005"; // A card in hand, so no turn auto-ends mid-fixture.

    /// The controller's unit row as def ids, lane 1 to 5.
    fn row(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        (1..=5)
            .map(|lane| s.unit(player, lane).map(|card| card.def_id.clone()))
            .collect()
    }

    fn backrow(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        (1..=5)
            .map(|lane| s.backrow(player, lane).map(|card| card.def_id.clone()))
            .collect()
    }

    fn graveyard(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.pile(player, "graveyard")
            .iter()
            .map(|card| card.def_id.clone())
            .collect()
    }

    /// A row as the TS tests write it: a def id per lane, or null.
    fn lanes(ids: [Option<&str>; 5]) -> Vec<Option<String>> {
        ids.iter().map(|id| id.map(str::to_string)).collect()
    }

    use crate::matches_object;

    mod n22_carnivorous_cube {
        use super::*;

        #[test]
        fn r81_the_meal_is_a_declared_play_time_choice_that_excludes_the_cube_itself() {
            let scripts = script();
            let base = &scripts.base;
            assert_eq!(base.targets.len(), 1);
            let decl = &base.targets[0];
            assert_eq!(decl.kind, PromptKind::Tribute);
            assert_eq!(decl.min, 1);
            assert_eq!(decl.max, 1);
            let filter = decl.filter.as_ref().expect("the meal's filter");
            assert_eq!(filter.side, Some(FilterSide::Ally));
            // R41: "cannot eat itself"; R428: "one of your other Units", so the unit row only.
            assert_eq!(filter.exclude_self, Some(true));
            assert_eq!(filter.of, Some(vec![FilterOf::Unit]));
            // §6.3: this card's tribute is a Sacrifice its own script performs, not a Tribute *cost*, so
            // the declaration carries no `amount` — `playChoices.tributeCostOf` would read that as a cost
            // paid from the play's `tributes` list, which reaches units only and refuses an unpayable play.
            assert!(decl.amount.is_none());
            // The radiant cell restates the Death clause only, so the Cry's choice is unchanged.
            assert_eq!(scripts.radiant.targets, base.targets);
        }

        mod base {
            use super::*;

            #[test]
            fn r81_cry_sacrifices_the_chosen_permanent_with_no_prompt() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "cube-eat",
                    "p1": { "hand": [CUBE, FILLER], "field": [TIMMY] },
                    "p2": { "hand": [FILLER] },
                }));
                let meal = s.card(TIMMY).clone();
                s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": meal.id }] }));

                // R81: a declared choice never pauses resolution.
                assert!(s.state().pending.is_none());
                assert_eq!(row(&s, P1), lanes([None, Some(CUBE), None, None, None]));
                s.expect_in_zone(&meal, "graveyard");
                s.expect_events(json!(["cardPlayed", "destroyed", "enteredGraveyard"]));
            }

            #[test]
            fn r41_r57_r64_death_summons_2_copies_of_the_remembered_card() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "cube-death",
                    "p1": { "hand": [CUBE, FILLER], "field": [TIMMY] },
                    "p2": { "hand": [FILLER], "field": [HITTER] },
                }));
                let timmy = s.card(TIMMY).id.clone();
                s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": timmy }] }));
                let cube = s.card(CUBE).clone();

                s.end_turn();
                s.attack(HITTER, &cube); // 7 through a 4/6.

                s.expect_in_zone(&cube, "graveyard");
                // R64: laneless summons take the leftmost free zones.
                assert_eq!(row(&s, P1), lanes([Some(TIMMY), Some(TIMMY), None, None, None]));
                let copy = s.unit(P1, 1).expect("expected a copy in lane 1");
                // R57: a copy is a fresh card at full health, not the corpse.
                s.expect_stats(&copy, json!({ "attack": 3, "maxHealth": 3, "health": 3 }));
                assert!(!copy.radiant);
            }

            #[test]
            fn r428_a_backrow_permanent_is_no_meal_naming_one_refuses_the_play() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "cube-backrow",
                    "p1": { "hand": [CUBE, FILLER], "backrow": [MANA_WELL], "field": [TIMMY] },
                    "p2": { "hand": [FILLER] },
                }));

                let well = s.card(MANA_WELL).id.clone();
                s.expect_refused(|s| s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": well }] })));
                // Nothing moved: the Cube is still in hand and the Field Spell still in its zone.
                s.expect_in_zone(CUBE, "hand");
                assert_eq!(backrow(&s, P1), lanes([Some(MANA_WELL), None, None, None, None]));
            }

            #[test]
            fn r428_with_only_a_backrow_permanent_beside_it_the_cube_has_nothing_to_eat_the_cry_fizzles_and_death_does_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "cube-backrow-only",
                    "p1": { "hand": [CUBE, FILLER], "backrow": [MANA_WELL] },
                    "p2": { "hand": [FILLER], "field": [HITTER] },
                }));
                s.play(CUBE, json!({}));
                let cube = s.card(CUBE).clone();

                assert_eq!(backrow(&s, P1), lanes([Some(MANA_WELL), None, None, None, None]));
                assert!(s.card(CUBE).memory.is_empty());

                s.end_turn();
                s.attack(HITTER, &cube);

                s.expect_in_zone(&cube, "graveyard");
                assert_eq!(row(&s, P1), lanes([None, None, None, None, None]));
                assert_eq!(backrow(&s, P1), lanes([Some(MANA_WELL), None, None, None, None]));
            }

            #[test]
            fn r41_r57_copies_keep_the_eaten_card_s_radiant_flag_and_stats_override() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "cube-stats",
                    "p1": { "hand": [CUBE, FILLER], "field": [{ "def": TIMMY, "radiant": true }] },
                    "p2": { "hand": [FILLER], "field": [HITTER] },
                }));
                // HARNESS GAP (see the header): §7 stats, which R41 says the copies inherit.
                let timmy = s.card(TIMMY).id.clone();
                find_instance_mut(s.state_mut(), &timmy)
                    .expect("Timmy on the field")
                    .stats_override = Some(AttackHealth { attack: 5, health: 5 });
                s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": timmy }] }));
                let cube = s.card(CUBE).clone();

                s.end_turn();
                s.attack(HITTER, &cube);

                let copies = [s.unit(P1, 1), s.unit(P1, 2)];
                assert_eq!(
                    copies.iter().map(|copy| copy.as_ref().map(|card| card.def_id.clone())).collect::<Vec<_>>(),
                    vec![Some(TIMMY.to_string()), Some(TIMMY.to_string())]
                );
                for copy in copies {
                    let copy = copy.expect("expected two copies");
                    assert!(copy.radiant);
                    s.expect_stats(&copy, json!({ "attack": 5, "maxHealth": 5 }));
                }
            }

            #[test]
            fn r41_nothing_to_tribute_the_cry_fizzles_and_the_cube_still_enters() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "cube-hungry",
                    "p1": { "hand": [CUBE, FILLER] },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(CUBE, json!({}));

                assert_eq!(row(&s, P1), lanes([Some(CUBE), None, None, None, None]));
                assert!(graveyard(&s, P1).is_empty());
                assert!(s.card(CUBE).memory.is_empty());
            }

            #[test]
            fn r41_nothing_eaten_death_does_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "cube-hungry-death",
                    "p1": { "hand": [CUBE, FILLER] },
                    "p2": { "hand": [FILLER], "field": [HITTER] },
                }));
                s.play(CUBE, json!({}));
                let cube = s.card(CUBE).clone();

                s.end_turn();
                s.attack(HITTER, &cube);

                assert_eq!(row(&s, P1), lanes([None, None, None, None, None]));
                assert_eq!(backrow(&s, P1), lanes([None, None, None, None, None]));
                assert_eq!(graveyard(&s, P1), vec![CUBE.to_string()]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r41_r64_death_fills_your_board_with_copies() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "cube-radiant-fill",
                    "p1": { "hand": [{ "def": CUBE, "radiant": true }, FILLER], "field": [TIMMY] },
                    // 7 + 6 = 13 gets through the radiant Cube's 12 health in one turn.
                    "p2": { "hand": [FILLER], "field": [HITTER, BIGOT] },
                }));
                let timmy = s.card(TIMMY).id.clone();
                s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": timmy }] }));
                let cube = s.card(CUBE).clone();
                s.expect_stats(&cube, json!({ "attack": 8, "maxHealth": 12 }));

                s.end_turn();
                s.attack(HITTER, &cube);
                s.expect_in_zone(&cube, "field"); // 7 of 12.
                s.attack(BIGOT, &cube);

                s.expect_in_zone(&cube, "graveyard");
                // R64: "fill your board" takes every empty, unlocked unit zone, the Cube's own included.
                assert_eq!(
                    row(&s, P1),
                    lanes([Some(TIMMY), Some(TIMMY), Some(TIMMY), Some(TIMMY), Some(TIMMY)])
                );
            }

            #[test]
            fn r428_the_radiant_cube_cannot_eat_a_backrow_permanent_either() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "cube-radiant-backrow",
                    "p1": { "hand": [{ "def": CUBE, "radiant": true }, FILLER], "backrow": [MANA_WELL], "field": [TIMMY] },
                    "p2": { "hand": [FILLER] },
                }));

                let well = s.card(MANA_WELL).id.clone();
                s.expect_refused(|s| s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": well }] })));
                let scripts = script();
                assert_eq!(
                    scripts.radiant.targets.first().and_then(|decl| decl.filter.as_ref()).and_then(|filter| filter.of.clone()),
                    Some(vec![FilterOf::Unit])
                );
            }

            #[test]
            fn r41_nothing_eaten_death_does_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "cube-radiant-hungry",
                    "p1": { "hand": [{ "def": CUBE, "radiant": true }, FILLER] },
                    "p2": { "hand": [FILLER], "field": [HITTER, BIGOT] },
                }));
                s.play(CUBE, json!({}));
                let cube = s.card(CUBE).clone();

                s.end_turn();
                s.attack(HITTER, &cube);
                s.attack(BIGOT, &cube);

                s.expect_in_zone(&cube, "graveyard");
                assert_eq!(row(&s, P1), lanes([None, None, None, None, None]));
                assert_eq!(graveyard(&s, P1), vec![CUBE.to_string()]);
            }

            #[test]
            fn the_cry_is_kept_the_radiant_face_still_eats_and_remembers_s8_conventions() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "cube-radiant-eat",
                    "p1": { "hand": [{ "def": CUBE, "radiant": true }, FILLER], "field": [TIMMY] },
                    "p2": { "hand": [FILLER] },
                }));
                let meal = s.card(TIMMY).clone();
                s.play(CUBE, json!({ "targets": [{ "pick": "instance", "instanceId": meal.id }] }));

                s.expect_in_zone(&meal, "graveyard");
                assert!(matches_object(
                    &json!(s.card(CUBE).memory),
                    &json!({ "eaten": { "defId": TIMMY, "radiant": false } })
                ));
            }
        }
    }
}
