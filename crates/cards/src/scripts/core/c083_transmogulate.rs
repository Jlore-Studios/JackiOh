//! #83 Transmogulate (SPEC §8.4 row 83): Spell, cost 2, Legendary.
//!   Base:    "Replace every card in your hand, deck, board, GY and exile with a random Legendary."
//!   Radiant: "... with a random Radiant Legendary." — the same five zones with `radiant: true` on
//!            every replacement (§8 Conventions).
//!
//! FIVE ZONES. Patch v0.1.1 added the hand to the library, board, GY and exile that R35 named; the
//! hand is an "other zone" of R35's like the rest (any card from the pool, same count), R365.
//!
//! R35 IS THE WHOLE CARD:
//!   "Board cards: same-type replacement in place, Field Trap counts as Trap, Immutable cards stay
//!    (R23). Other zones: any card from the pool, same counts. Replaced cards cease to exist.
//!    Pool: the §8 Legendary-rarity cards except #83, which is #52, #85, #87, #92, #93, #95".
//! Clause by clause:
//!   - "the pool" is `catalog.pool("83", { rarity: "Legendary" })` — §5.1's one query function with
//!     this card's own index excluded, which `test/query.test.ts` pins to exactly those six. This
//!     file never lists the six by hand: two sources of one pool is the bug that file prevents.
//!   - "same-type replacement in place" is one `transform` per board card, narrowed to the
//!     Legendaries of that card's type: a Unit becomes #52 or #92, a Field Spell becomes #93, and a
//!     Trap becomes #85. "Field Trap counts as Trap" is `TRAP_TYPES` (both types in one query), so a
//!     Field Trap also becomes #85 Unlicensed Experimentation — BUILD M4-T4's own example. The pool
//!     holds no Legendary Field Trap, which is why the type match has to be read this way rather
//!     than as an exact `type` equality.
//!   - "in place" and "replaced cards cease to exist" are `effects/transform.ts`: `replaceOnField`
//!     keeps the zone, the position and the Stack pile beneath, `replaceOffField` keeps the pile
//!     index (so a library keeps its order), and both leave the old card in no pile at all. That is
//!     also why the counts per zone are preserved without this card counting anything: every
//!     replacement is one card for one card.
//!   - "Immutable cards stay (R23), since this is a Transform" needs no check here either:
//!     `transform` refuses an Immutable target and does nothing, so that card keeps its place and
//!     the zone count still holds.
//!
//! R13 "Stack dormancy": cards under a Stack are not on the field, so "your board" is the top of
//! each unit pile plus the backrow, which is what `cardAt` returns per zone (§3.2). Transforming a
//! dormant card would also be wrong mechanically — `placeOnField(…, { stack: true })` puts the
//! replacement on TOP of the pile, which would promote it past the card that is actually acting.
//!
//! R11: a unit token cannot sit in a graveyard or an exile pile. The pool holds no tokens, so no
//! replacement can vanish on arrival and thin a zone (`replaceOffField` guards it anyway).
//!
//! RANDOMNESS (CLAUDE.md rule 4, §9.3). Every pick is `ctx.rng`, never `Math.random`, and the picks
//! happen while the effect list is being built — the pool is a definition list, and `transform` takes
//! one `defId`, so there is no "transform into a random X" effect to defer them into. That is
//! deterministic here because this hook runs exactly once: the card opens no prompt, so nothing can
//! re-enter it and `applyResumable` never replays part of the list (`prompts.ts`).
//!
//! THE OFF-FIELD READ. `zones.ts` exposes `slotsOf`/`cardAt` for the field but owns no reader for
//! the off-field piles, so enumerating "every card in your library, GY and exile" used to mean
//! naming the fields of `PlayerState` — which BUILD M3-T1's acceptance greps for, and which made
//! this card one of fifteen files that would have to be edited if that shape ever changed. The
//! engine's read-only board surface now answers it:
//!     zoneCards(state, player, zone): readonly CardInstance[]     // engine/src/query.ts
//! one pile as a copy, which is also what this card needs mechanically — see `pileCards` below.
//! Reading is not mutation either way (CLAUDE.md rule 5 bans writing, and nothing here writes).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-083";

/// R35's "other zones", in the order this card walks them; the hand is one of them (R365).
const OFF_FIELD_ZONES: [OffFieldZone; 4] = [
    OffFieldZone::Hand,
    OffFieldZone::Library,
    OffFieldZone::Graveyard,
    OffFieldZone::Exile,
];

/// R35's pool: every non-token Legendary but this card (R387), of every set (R380) — in Core #52, #85,
/// #87, #92, #93, #95, proved by `test/query.test.ts`, not listed here.
///
/// `type_` is TS's `CardType | CardType[]`, as the JSON the query takes.
fn legendaries(type_: Option<Value>) -> Vec<CardDef> {
    let mut args = json!({ "rarity": "Legendary" });
    if let Some(type_) = type_ {
        args["type"] = type_;
    }
    crate::query::pool(ID, json_as(args))
}

/// R35 on the board: same type, with Field Trap counting as Trap in both directions.
fn same_type_legendaries(type_: CardType) -> Vec<CardDef> {
    legendaries(Some(if type_ == CardType::Trap || type_ == CardType::FieldTrap {
        json!(crate::query::TRAP_TYPES)
    } else {
        json!(type_)
    }))
}

/// §3.2 and R13: "your board" is the card acting in each zone — the top of a pile, not the pile.
/// R35 and R23: an Immutable board card stays, since on the field a Replace is a Transform, so it is
/// left out here rather than handed to `transform` to refuse: R129 has an effect that finds nothing
/// to do draw no random number, and a pick rolled for a card that stays would be exactly that.
fn board_cards(ctx: &EffectContext<'_>) -> Vec<CardInstance> {
    [Row::Units, Row::Backrow]
        .into_iter()
        .flat_map(|row| {
            slots_of(ctx.controller, row)
                .into_iter()
                .filter_map(|slot| {
                    let card = card_at(&ctx.state, &slot)?;
                    if unit_has(&ctx.state, &card, KeywordKind::Immutable) {
                        None
                    } else {
                        Some(card.clone())
                    }
                })
                .collect::<Vec<CardInstance>>()
        })
        .collect()
}

/// One off-field pile of the controller's. `zoneCards` hands back a copy, which matters here: the
/// `transform`s this card builds replace every card in the pile being walked, so iterating the live
/// array would be iterating a list the effects are rewriting.
fn pile_cards(ctx: &EffectContext<'_>, zone: OffFieldZone) -> Vec<CardInstance> {
    let cards: Vec<CardInstance> = zone_cards(&ctx.state, ctx.controller, zone).to_vec();
    // R223: each replacement takes a new id, and walked top down the library's would be one run of
    // numbers in library order, so any of them the owner is later shown (#51's reveal, a Recruit)
    // would say where it lies. The library is walked in an order of the seed's own instead; each
    // replacement still takes its card's place (R35).
    if zone == OffFieldZone::Library {
        numbering_order(&ctx.state, &cards)
    } else {
        cards
    }
}

/// §6.3 Replace: one card, one random Legendary from its pool, named by instance (R81 does not apply).
fn replace(ctx: &mut EffectContext<'_>, card: &CardInstance, pool: &[CardDef], radiant_result: bool) -> Vec<Effect> {
    let Some(pick) = ctx.rng.pick(pool) else {
        return vec![];
    };
    vec![transform(json_as(json!({
        "instanceId": card.id,
        "defId": pick.id,
        "radiant": radiant_result,
    })))]
}

/// The faces differ only in whether the cards that arrive are Radiant (§5.2, R74).
fn transmogulate(radiant_result: bool) -> Script {
    Script {
        // A Spell's script hangs off `cry`: that is its on-resolve hook (§10.9).
        cry: Some(hook(move |ctx| {
            let mut effects: Vec<Effect> = Vec::new();
            // The board first, each card by its own type (R35); lane order, so the rng draws are fixed.
            for card in board_cards(ctx) {
                let type_ = def_of(&ctx.state, &card.def_id).type_;
                let pool = same_type_legendaries(type_);
                effects.extend(replace(ctx, &card, &pool, radiant_result));
            }
            // Then hand (R365), library (in `pileCards` order, R223), graveyard and exile: "any card from
            // the pool, same counts".
            for zone in OFF_FIELD_ZONES {
                for card in pile_cards(ctx, zone) {
                    let pool = legendaries(None);
                    effects.extend(replace(ctx, &card, &pool, radiant_result));
                }
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = transmogulate(false);
    // "Random Radiant Legendaries": the same five zones, every replacement Radiant.
    let radiant = transmogulate(true);
    CardScripts { base, radiant }
}

// #83 Transmogulate (SPEC §8.4 row 83; R11, R23, R35, R365).
//
// BUILD M4-T4's must-pass row: "Zone counts preserved; board cards replaced by same-type
// Legendaries in place; pool is exactly #52, #85, #87, #92, #93, #95, and a Field Trap becomes
// Unlicensed Experimentation (R35); the hand is replaced too (R365); radiant gives radiant cards" —
// and since patch v0.2.0 (R380) the pool is every set's non-token Legendaries but #83.
//
// The board fixture covers every permanent type at once: a Unit, an Immutable Unit (R23), a Field
// Spell, a Trap and a Field Trap. The pool is read from the catalog and checked to hold the six ids
// R35 names, so if `query`, the rarities or the exclusion of #83 ever drift, these tests say so.
//
// One count to keep in mind: Transmogulate is a Spell, so it reaches its owner's graveyard AFTER
// its own script has run (§10.5). The graveyard therefore ends one card larger than it started —
// the replacement for what was there, plus Transmogulate itself, which was in `resolving` while the
// replacements happened and so was never replaced by one of them.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TRANSMOGULATE: &str = "core-083";

    /// An engine value as the JSON the TS test compares it with.
    fn js<T: serde::Serialize>(v: &T) -> Value {
        serde_json::to_value(v).expect("serialises")
    }

    /// TS `catalog.pool(TRANSMOGULATE, args).map((def) => def.id)`.
    fn pool_of(args: Value) -> Vec<String> {
        crate::query::pool(TRANSMOGULATE, json_as(args))
            .into_iter()
            .map(|def| def.id.clone())
            .collect()
    }

    /// R35's pool: "every non-token Legendary except #83" — of every set since patch v0.2.0 (R380). In
    /// Core that is the six R35 names, #52, #85, #87, #92, #93, #95. (TS `POOL`.)
    fn pool_ids() -> Vec<String> {
        pool_of(json!({ "rarity": "Legendary" }))
    }

    const CORE_SIX: [&str; 6] = ["core-052", "core-085", "core-087", "core-092", "core-093", "core-095"];

    /// The Legendary Units in the pool, every set's (Core #52 Silly Silas and #92 Felinor Fiender among
    /// them). (TS `LEGENDARY_UNITS`.)
    fn legendary_units() -> Vec<String> {
        pool_of(json!({ "rarity": "Legendary", "type": "Unit" }))
    }

    /// The Legendary Field Spells: Core #93 Combo-Index, Classic #4, #7, Classic+ #78. (TS
    /// `LEGENDARY_FIELD_SPELLS`.)
    fn legendary_field_spells() -> Vec<String> {
        pool_of(json!({ "rarity": "Legendary", "type": "Field Spell" }))
    }

    /// The Legendary Trap this seed draws: #85 Unlicensed Experimentation — and "Field Trap counts as
    /// Trap". Since patch v0.2.9 (issue #44) there are two Legendary Traps of any set (#85 and Classic
    /// #9 Income Tax); the "transmogulate-1" seed draws #85 for both trap slots.
    const LEGENDARY_TRAP: &str = "core-085";

    /// Board fixtures: #11 Tempo Timmy (Unit), a Radiant #19 Midrange Menace (Immutable Unit), #73
    ///  (Field Spell), #41 Sheepish (Trap), #18 Bread and Butter (Field Trap).
    const IMMUTABLE: &str = "core-019";

    fn board(radiant_face: bool) -> Scenario {
        let mut s = scenario(json!({
            // Pinned after patch v0.2.9 added Classic #9 to the Legendary Trap pool: this seed draws #85
            // for both trap slots and no replacement casts (a Spell Tyrant draw cascades under other seeds).
            "seed": "transmogulate-1",
            "p1": {
                "hand": [TRANSMOGULATE, "core-056"],
                "field": ["core-011", { "def": IMMUTABLE, "radiant": true }],
                "backrow": [
                    { "def": "core-073", "lane": 1 },
                    { "def": "core-041", "lane": 2 },
                    { "def": "core-018", "lane": 3 },
                ],
                "library": ["core-020", "core-025"],
                "graveyard": ["core-056"],
                "exile": ["core-011"],
            },
            "p2": { "field": ["core-011"], "library": ["core-020"] },
        }));
        // Stands in for a missing `{ def, radiant }` form on SideSetup.hand (harness request).
        if radiant_face {
            let id = s.card(TRANSMOGULATE).id.clone();
            find_instance_mut(s.state_mut(), &id).expect("Transmogulate in hand").radiant = true;
        }
        s
    }

    /// The def id of the unit acting in a lane, or "" for an empty lane (TS `s.unit(p, lane)?.defId`).
    fn unit_def(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane).map(|card| card.def_id.clone()).unwrap_or_default()
    }

    /// The def id of the backrow card in a lane, or "" for an empty lane.
    fn backrow_def(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.backrow(player, lane).map(|card| card.def_id.clone()).unwrap_or_default()
    }

    mod n83_transmogulate_base {
        use super::*;

        #[test]
        fn r35_board_cards_are_replaced_in_place_by_a_legendary_of_the_same_type() {
            crate::register_all();
            let mut s = board(false);
            s.play(TRANSMOGULATE, json!({}));

            // A Unit becomes a Legendary Unit, in its own lane.
            assert!(legendary_units().contains(&unit_def(&s, P1, 1)));
            // A Field Spell becomes the Legendary Field Spell; a Trap becomes the Legendary Trap.
            assert!(legendary_field_spells().contains(&backrow_def(&s, P1, 1)));
            assert_eq!(backrow_def(&s, P1, 2), LEGENDARY_TRAP);
            s.expect_events(json!(["cardPlayed", "transformed"]));
        }

        #[test]
        fn r35_field_trap_counts_as_trap_a_field_trap_becomes_unlicensed_experimentation() {
            crate::register_all();
            let mut s = board(false);
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(backrow_def(&s, P1, 3), LEGENDARY_TRAP);
        }

        #[test]
        fn r23_an_immutable_board_card_stays_since_this_is_a_transform() {
            crate::register_all();
            let mut s = board(false);
            let immutable = s.card(IMMUTABLE).id.clone();
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(s.unit(P1, 2).map(|card| card.id.clone()), Some(immutable));
            assert_eq!(unit_def(&s, P1, 2), IMMUTABLE);
        }

        #[test]
        fn r35_r365_same_counts_per_zone_in_hand_library_graveyard_and_exile() {
            crate::register_all();
            let mut s = board(false);
            let hand = s.hand(Some(P1)).len() - 1;
            let library = s.pile(P1, "library").len();
            let graveyard = s.pile(P1, "graveyard").len();
            let exile = s.pile(P1, "exile").len();
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(s.hand(Some(P1)).len(), hand);
            assert_eq!(s.pile(P1, "library").len(), library);
            assert_eq!(s.pile(P1, "exile").len(), exile);
            // Plus Transmogulate itself, which was resolving while the replacements happened.
            assert_eq!(s.pile(P1, "graveyard").len(), graveyard + 1);
            assert!(s
                .pile(P1, "graveyard")
                .iter()
                .any(|card| card.def_id == TRANSMOGULATE));
        }

        #[test]
        fn r35_r380_the_pool_is_every_set_s_non_token_legendaries_core_s_n52_n85_n87_n92_n93_n95_among_them_never_transmogulate_itself(
        ) {
            crate::register_all();
            let pool = pool_ids();
            for id in CORE_SIX {
                assert!(pool.contains(&id.to_string()));
            }
            assert!(!pool.contains(&TRANSMOGULATE.to_string()));
            assert!(pool.iter().any(|id| id.starts_with("classic")));
            let mut s = board(false);
            let spell = s.card(TRANSMOGULATE).id.clone();
            s.play(TRANSMOGULATE, json!({}));

            let mut replaced: Vec<CardInstance> = Vec::new();
            replaced.extend(s.hand(Some(P1)));
            replaced.extend(s.pile(P1, "library"));
            replaced.extend(s.pile(P1, "exile"));
            replaced.extend(s.pile(P1, "graveyard").into_iter().filter(|card| card.id != spell));
            for lane in 1..=5 {
                if let Some(unit) = s.unit(P1, lane) {
                    replaced.push(unit.clone());
                }
                if let Some(back) = s.backrow(P1, lane) {
                    replaced.push(back.clone());
                }
            }

            for card in &replaced {
                // The Immutable Menace is the one card R23 left alone.
                if card.def_id == IMMUTABLE {
                    continue;
                }
                assert!(pool.contains(&card.def_id), "{} is not in R35's pool", card.def_id);
            }
        }

        #[test]
        fn r35_replaced_cards_cease_to_exist_they_are_in_no_pile_at_all() {
            crate::register_all();
            let mut s = board(false);
            let old_unit = s.card("core-011").id.clone();
            let old_library: Vec<String> = s.pile(P1, "library").into_iter().map(|card| card.id).collect();
            let old_graveyard: Vec<String> = s.pile(P1, "graveyard").into_iter().map(|card| card.id).collect();
            s.play(TRANSMOGULATE, json!({}));

            s.expect_in_zone(old_unit.as_str(), "gone");
            for id in old_library.iter().chain(old_graveyard.iter()) {
                s.expect_in_zone(id.as_str(), "gone");
            }
        }

        #[test]
        fn r365_your_hand_is_replaced_too_the_card_in_it_ceases_to_exist_and_a_pool_card_takes_its_place() {
            crate::register_all();
            let mut s = board(false);
            let spare = s
                .hand(Some(P1))
                .into_iter()
                .find(|card| card.def_id == "core-056")
                .map(|card| card.id)
                .unwrap_or_default();
            s.play(TRANSMOGULATE, json!({}));

            s.expect_in_zone(spare.as_str(), "gone");
            assert_eq!(s.hand(Some(P1)).len(), 1);
            let first = s.hand(Some(P1)).first().cloned();
            assert!(pool_ids().contains(&first.as_ref().map(|card| card.def_id.clone()).unwrap_or_default()));
            assert_eq!(first.map(|card| card.radiant), Some(false));
        }

        #[test]
        fn r365_the_opponent_never_learns_what_the_hand_became_r177() {
            crate::register_all();
            let mut s = board(false);
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(js(&s.view(Some(P2)).opponent.hand), json!({ "count": 1 }));
            let seen = serde_json::to_string(&s.view(Some(P2))).expect("serialises");
            let held = s
                .hand(Some(P1))
                .first()
                .map(|card| card.id.clone())
                .unwrap_or_else(|| "no-card".to_string());
            assert!(!seen.contains(&held));
        }

        #[test]
        fn s8_your_library_and_board_the_opponent_keeps_everything() {
            crate::register_all();
            let mut s = board(false);
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(unit_def(&s, P2, 1), "core-011");
            assert_eq!(
                s.pile(P2, "library").into_iter().map(|card| card.def_id).collect::<Vec<_>>(),
                vec!["core-020".to_string()]
            );
        }

        #[test]
        fn base_gives_non_radiant_cards() {
            crate::register_all();
            let mut s = board(false);
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(
                s.pile(P1, "library").into_iter().map(|card| card.radiant).collect::<Vec<_>>(),
                vec![false, false]
            );
            assert_eq!(s.unit(P1, 1).map(|card| card.radiant), Some(false));
            assert_eq!(s.backrow(P1, 1).map(|card| card.radiant), Some(false));
        }

        #[test]
        fn r13_a_card_dormant_under_a_stack_is_not_on_your_board_only_the_top_of_the_pile_is_replaced() {
            crate::register_all();
            // #92 Felinor Fiender (Stack) on top of #43 Big Felinor in lane 1; the Big Felinor is dormant.
            let mut s = scenario(json!({
                "seed": "transmogulate-stack",
                "p1": {
                    "hand": [TRANSMOGULATE],
                    "field": ["core-043", { "def": "core-092", "stack": true }],
                    "library": ["core-020"],
                },
                "p2": { "field": ["core-011"], "library": ["core-020"] },
            }));
            let buried = s.card("core-043").clone();
            let top = s.card("core-092").clone();
            assert_eq!(
                s.unit(P1, 1).map(|card| card.id.clone()),
                Some(top.id.clone()),
                "the Fiender is on top of lane 1"
            );

            s.play(TRANSMOGULATE, json!({}));

            // The acting top card was replaced in place by a Legendary Unit.
            let transformed: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Transformed { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect();
            assert!(transformed.contains(&top.id));
            assert!(legendary_units().contains(&unit_def(&s, P1, 1)));
            // The dormant card is untouched: still on the field beneath it, still a Big Felinor.
            assert!(!transformed.contains(&buried.id));
            s.expect_in_zone(&buried, "field");
            assert_eq!(s.card(&buried).def_id, "core-043");
        }
    }

    mod n83_transmogulate_radiant {
        use super::*;

        #[test]
        fn random_radiant_legendaries_every_replacement_is_radiant_in_every_zone() {
            crate::register_all();
            let mut s = board(true);
            let spell = s.card(TRANSMOGULATE).id.clone();
            s.play(TRANSMOGULATE, json!({}));

            assert_eq!(s.unit(P1, 1).map(|card| card.radiant), Some(true));
            assert_eq!(s.backrow(P1, 1).map(|card| card.radiant), Some(true));
            assert_eq!(s.backrow(P1, 2).map(|card| card.radiant), Some(true));
            assert_eq!(s.backrow(P1, 3).map(|card| card.radiant), Some(true));
            assert_eq!(
                s.pile(P1, "library").into_iter().map(|card| card.radiant).collect::<Vec<_>>(),
                vec![true, true]
            );
            assert_eq!(
                s.pile(P1, "exile").into_iter().map(|card| card.radiant).collect::<Vec<_>>(),
                vec![true]
            );
            assert_eq!(
                s.hand(Some(P1)).into_iter().map(|card| card.radiant).collect::<Vec<_>>(),
                vec![true]
            );
            assert_eq!(
                s.pile(P1, "graveyard")
                    .into_iter()
                    .filter(|card| card.id != spell)
                    .map(|card| card.radiant)
                    .collect::<Vec<_>>(),
                vec![true]
            );
        }

        #[test]
        fn the_radiant_face_keeps_r35_s_pool_and_its_same_type_board_rule() {
            crate::register_all();
            let mut s = board(true);
            s.play(TRANSMOGULATE, json!({}));

            assert!(legendary_units().contains(&unit_def(&s, P1, 1)));
            assert!(legendary_field_spells().contains(&backrow_def(&s, P1, 1)));
            assert_eq!(backrow_def(&s, P1, 3), LEGENDARY_TRAP);
            let pool = pool_ids();
            for card in s.pile(P1, "library") {
                assert!(pool.contains(&card.def_id));
            }
        }

        #[test]
        fn r23_an_immutable_board_card_still_stays_on_the_radiant_face() {
            crate::register_all();
            let mut s = board(true);
            s.play(TRANSMOGULATE, json!({}));

            let immutable = s.unit(P1, 2).cloned();
            assert_eq!(immutable.as_ref().map(|card| card.def_id.as_str()), Some(IMMUTABLE));
            // A refused Transform changes nothing about the card: it is the Menace it was.
            assert_eq!(immutable.map(|card| card.radiant), Some(true));
        }
    }

    mod n83_transmogulate_r312_the_owner_s_library_list {
        use super::*;

        #[test]
        fn r312_every_library_replacement_is_a_card_its_owner_was_never_shown_so_the_list_counts_them_unknown() {
            crate::register_all();
            let mut s = board(false);
            // Before: p1's own two cards, by printed cost (R310): Pointmaster (2), then the 7/7 (4).
            assert_eq!(
                js(&s.view(Some(P1)).you.own_library),
                json!({
                    "cards": [
                        { "defId": "core-020", "radiant": false, "count": 1 },
                        { "defId": "core-025", "radiant": false, "count": 1 },
                    ],
                    "unknown": 0,
                })
            );

            s.play(TRANSMOGULATE, json!({}));

            // The same count, none of it named: the Legendaries it rolled stay unread, even by p1.
            assert_eq!(js(&s.view(Some(P1)).you.own_library), json!({ "cards": [], "unknown": 2 }));
            let mine = serde_json::to_string(&s.view(Some(P1))).expect("serialises");
            for card in s.pile(P1, "library") {
                assert!(!mine.contains(&format!("\"{}\"", card.id)));
            }
            // p2's library is untouched and still fully known to p2.
            assert_eq!(
                js(&s.view(Some(P2)).you.own_library),
                json!({ "cards": [{ "defId": "core-020", "radiant": false, "count": 1 }], "unknown": 0 })
            );
        }
    }
}
