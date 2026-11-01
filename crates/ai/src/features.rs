//! The learned evaluation's inputs (#466, #1006; `docs/learned-eval.md` D3, D5, D10): integer
//! feature vectors derived from `redact(state, seat)` (R185, R1006000).
//! Dense features include life, mana, unit stats through §10.4 layers (`unit_view`), keywords,
//! and hand distribution with Radiant counts (R1012004, #1143).
//! Sparse features track board, hand, and readable backrow card counts.
//! `FEATURES_VERSION` tracks the layout version. Pure (CLAUDE.md rule 4).

use jackioh_engine::{
    GameState, Keyword, KeywordKind, PlayerId, Position, active_units_of, backrow_is_public, cannot_attack,
    effective_life, find_def, has_keyword, hero_armor_of, owed_mana_of, query_cost, unit_view,
};
use serde::{Deserialize, Serialize};

use crate::evaluate::face_threat;
use crate::observe::redact;

/// The layout `features` writes. Bump it with any change to what a feature means or where it sits.
/// 2: a Radiant card in the seat's own hand (R1012004).
pub const FEATURES_VERSION: u32 = 2;

/// The highest own-hand cost bin: a card costing this or more counts in the last bin.
pub const HAND_COST_BINS: i32 = 8;

/// The dense fields of the whole position, before the sides'.
const GLOBAL_FIELDS: &[&str] = &["turn", "seat_acts", "seat_is_p1"];

/// Each side's dense fields, before its keyword counts. `side_fields` writes them in this order.
const SIDE_FIELDS: &[&str] = &[
    "life",
    "armor",
    "mana",
    "crystals",
    "owed",
    "hand",
    "library",
    "graveyard",
    "backrow",
    "backrow_readable",
    "backrow_hidden",
    "units",
    "units_atk",
    "units_def",
    "units_can_swing",
    "attack",
    "attack_ready",
    "health",
    "max_health",
    "unit_armor",
    "damaged",
    "radiant_units",
    "face_threat",
    "jade",
    "extra_turns",
];

/// The two sides, the seat's own first.
const SIDES: [&str; 2] = ["own", "enemy"];

/// The keyword kinds no count reads: a hand keyword newer than the layout, which does nothing on the
/// field where the counts are taken (R1838000's Tradable), and Stinkyscape's Starship Piece and
/// Starship (R1852000, R1852003), which came after the layout was fitted and wait for its next refit,
/// as does Can't Defend (R2657000). Left out, so the layout stays the one `FEATURES_VERSION` names and
/// the weights fitted on it still read it (R1010000); a Unit carrying one counts its other keywords as
/// before.
const UNCOUNTED_KEYWORDS: &[KeywordKind] = &[
    KeywordKind::Tradable,
    KeywordKind::StarshipPiece,
    KeywordKind::Starship,
    KeywordKind::CantDefend,
];

/// The keyword kinds the layout counts, one dense feature a side each, in `KeywordKind::ALL`'s order.
fn counted_keywords() -> impl Iterator<Item = KeywordKind> {
    KeywordKind::ALL
        .iter()
        .copied()
        .filter(|kind| !UNCOUNTED_KEYWORDS.contains(kind))
}

/// A position as the learned evaluation reads it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct Features {
    /// One integer per name of `dense_names`, in its order.
    pub dense: Vec<i32>,
    /// `(key, count)`, sorted by key, each key once.
    pub sparse: Vec<(String, i32)>,
}

/// The name of each dense feature, in the order `features` writes them.
pub fn dense_names() -> Vec<String> {
    let mut names: Vec<String> = GLOBAL_FIELDS.iter().map(|name| (*name).to_string()).collect();
    for side in SIDES {
        for field in SIDE_FIELDS {
            names.push(format!("{side}.{field}"));
        }
        for kind in counted_keywords() {
            names.push(format!("{side}.kw.{}", kind.as_str()));
        }
    }
    for cost in 0..HAND_COST_BINS {
        let plus = if cost == HAND_COST_BINS - 1 { "+" } else { "" };
        names.push(format!("own.hand_cost.{cost}{plus}"));
    }
    names.push("own.hand_radiant".to_string());
    names
}

/// How many dense features there are (`dense_names().len()`), without building the names.
pub fn dense_len() -> usize {
    GLOBAL_FIELDS.len()
        + SIDES.len() * (SIDE_FIELDS.len() + counted_keywords().count())
        + HAND_COST_BINS as usize
        + 1
}

/// R185, R1006000: `state`'s features for `seat`, read from `redact(state, seat)` alone.
pub fn features_of(state: &GameState, seat: PlayerId) -> Features {
    features(&redact(state, seat), seat)
}

/// `view`'s features for `seat`. `view` must already be what `seat` may know (`redact`); the search
/// hands it a redacted copy of the world it scores, and `features_of` redacts for anyone else.
pub fn features(view: &GameState, seat: PlayerId) -> Features {
    let public = public_world(view, seat);
    let mut dense: Vec<i32> = Vec::with_capacity(dense_len());
    dense.push(view.turn);
    dense.push(i32::from(view.active == seat));
    dense.push(i32::from(seat == PlayerId::P1));
    for player in [seat, seat.opponent()] {
        side_fields(view, &public, player, seat, &mut dense);
    }
    hand_costs(view, seat, &mut dense);
    dense.push(len_i32(radiant_in_hand(view, seat).count()));

    let mut sparse: Vec<(String, i32)> = Vec::new();
    sparse_cards(view, &public, seat, &mut sparse);
    Features {
        dense,
        sparse: tally(sparse),
    }
}

/// R1006000: `view` without the backrow cards `seat` cannot read. The engine's projections
/// (`unit_view`'s auras, `face_threat`'s hero guards, `hero_armor_of`) read every backrow card's
/// text, and a redacted view's hidden ones are placeholders with no text: what the seat cannot read
/// is left out of them rather than guessed, so a projection reads only what the seat can see. The
/// counts of hidden cards come from `view` itself.
fn public_world(view: &GameState, seat: PlayerId) -> GameState {
    let mut public = view.clone();
    for player in [seat, seat.opponent()] {
        let hidden: Vec<usize> = view.players[player]
            .backrow
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.as_ref().is_some_and(|card| !readable_by(view, card, seat)))
            .map(|(at, _)| at)
            .collect();
        for at in hidden {
            public.players[player].backrow[at] = None;
        }
    }
    public
}

/// `player`'s side, as `seat` reads it: `SIDE_FIELDS` in order, then one count per keyword kind.
/// Counts come from `view`; what reads card text (units through the layers, armor, the threat) from
/// `public` (`public_world`).
fn side_fields(view: &GameState, public: &GameState, player: PlayerId, seat: PlayerId, dense: &mut Vec<i32>) {
    let side = &view.players[player];
    let mut backrow = 0;
    let mut readable = 0;
    for card in side.backrow.iter().flatten() {
        backrow += 1;
        if readable_by(view, card, seat) {
            readable += 1;
        }
    }

    let units = active_units_of(public, player);
    let mut atk = 0;
    let mut def = 0;
    let mut can_swing = 0;
    let mut attack = 0;
    let mut attack_ready = 0;
    let mut health = 0;
    let mut max_health = 0;
    let mut unit_armor = 0;
    let mut damaged = 0;
    let mut radiant = 0;
    let mut keywords = vec![0; counted_keywords().count()];
    for unit in &units {
        let shown = unit_view(public, unit);
        let swings = !has_keyword(&shown.keywords, KeywordKind::CantAttack) && !cannot_attack(public, unit);
        match shown.position {
            Position::Atk => atk += 1,
            Position::Def => def += 1,
        }
        can_swing += i32::from(swings);
        attack += shown.attack;
        if swings && shown.position == Position::Atk {
            attack_ready += shown.attack;
        }
        health += shown.health;
        max_health += shown.max_health;
        unit_armor += shown.armor;
        damaged += i32::from(shown.health < shown.max_health);
        radiant += i32::from(unit.radiant);
        for keyword in &shown.keywords {
            if let Some(at) = counted_keywords().position(|kind| kind == keyword.kind()) {
                keywords[at] += keyword_weight(keyword);
            }
        }
    }

    // In `SIDE_FIELDS`' order.
    dense.push(effective_life(public, player));
    dense.push(hero_armor_of(public, player));
    dense.push(side.mana.current);
    dense.push(side.mana.max);
    dense.push(owed_mana_of(view, player));
    dense.push(len_i32(side.hand.len()));
    dense.push(len_i32(side.library.len()));
    dense.push(len_i32(side.graveyard.len()));
    dense.push(backrow);
    dense.push(readable);
    dense.push(backrow - readable);
    dense.push(len_i32(units.len()));
    dense.push(atk);
    dense.push(def);
    dense.push(can_swing);
    dense.push(attack);
    dense.push(attack_ready);
    dense.push(health);
    dense.push(max_health);
    dense.push(unit_armor);
    dense.push(damaged);
    dense.push(radiant);
    dense.push(face_threat(public, player));
    dense.push(side.jade.unwrap_or(0));
    dense.push(side.extra_turns.unwrap_or(0));
    dense.extend(keywords);
}

/// A keyword's count: 1 for a flag, its N for Brittle N and Spell Damage +N.
fn keyword_weight(keyword: &Keyword) -> i32 {
    match keyword {
        Keyword::SpellDamage { n } | Keyword::Brittle { n } => *n,
        _ => 1,
    }
}

/// The seat's own hand by printed cost, `HAND_COST_BINS` bins, the last one open-ended. A card with
/// no definition (none in the seat's own hand after `redact`) counts in none.
fn hand_costs(view: &GameState, seat: PlayerId, dense: &mut Vec<i32>) {
    let mut bins = vec![0; HAND_COST_BINS as usize];
    for card in &view.players[seat].hand {
        if let Some(def) = find_def(Some(view), &card.def_id) {
            let at = query_cost(def).clamp(0, HAND_COST_BINS - 1);
            bins[at as usize] += 1;
        }
    }
    dense.extend(bins);
}

/// §10.8, R33, R686: whether `seat` may read this backrow card, as `evaluate` asks it: a placeholder
/// never, then `backrow_is_public`.
fn readable_by(view: &GameState, card: &jackioh_engine::CardInstance, seat: PlayerId) -> bool {
    find_def(Some(view), &card.def_id).is_some() && backrow_is_public(view, card, seat)
}

/// Every card the seat can name on the board, in its own hand and readable in a backrow, one key per
/// copy (`tally` adds them up).
fn sparse_cards(view: &GameState, public: &GameState, seat: PlayerId, out: &mut Vec<(String, i32)>) {
    for (player, side) in [(seat, SIDES[0]), (seat.opponent(), SIDES[1])] {
        for unit in active_units_of(public, player) {
            if find_def(Some(view), &unit.def_id).is_some() {
                out.push((format!("board:{side}:{}", unit.def_id), 1));
            }
        }
        for card in view.players[player].backrow.iter().flatten() {
            if readable_by(view, card, seat) {
                out.push((format!("backrow:{side}:{}", card.def_id), 1));
            }
        }
    }
    for card in &view.players[seat].hand {
        if find_def(Some(view), &card.def_id).is_some() {
            out.push((format!("hand:{}:{}", SIDES[0], card.def_id), 1));
        }
    }
    for card in radiant_in_hand(view, seat) {
        out.push((format!("hand_radiant:{}:{}", SIDES[0], card.def_id), 1));
    }
}

/// R1012004: the Radiant cards in the seat's own hand that it can name (each with a definition).
fn radiant_in_hand(view: &GameState, seat: PlayerId) -> impl Iterator<Item = &jackioh_engine::CardInstance> {
    view.players[seat]
        .hand
        .iter()
        .filter(move |card| card.radiant && find_def(Some(view), &card.def_id).is_some())
}

/// `pairs` sorted by key with each key's counts added: one entry per key.
fn tally(mut pairs: Vec<(String, i32)>) -> Vec<(String, i32)> {
    pairs.sort();
    let mut out: Vec<(String, i32)> = Vec::with_capacity(pairs.len());
    for (key, count) in pairs {
        match out.last_mut() {
            Some((last, total)) if *last == key => *total += count,
            _ => out.push((key, count)),
        }
    }
    out
}

fn len_i32(len: usize) -> i32 {
    i32::try_from(len).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_and_the_length_agree() {
        assert_eq!(dense_names().len(), dense_len());
        let names = dense_names();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "every dense name is unique");
    }

    #[test]
    fn tally_adds_repeats_and_sorts() {
        let pairs = vec![("b".to_string(), 1), ("a".to_string(), 1), ("b".to_string(), 1)];
        assert_eq!(tally(pairs), vec![("a".to_string(), 1), ("b".to_string(), 2)]);
    }
}
