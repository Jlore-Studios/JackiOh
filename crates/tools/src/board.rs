//! The position printers `trace` and `think` share: a card's name, one seat's side of the board on one
//! line, a seat's hand and an action in words. One module, so that the two commands print a board the
//! same way.

use jackioh_ai::HIDDEN_DEF_ID;
use jackioh_engine::layers::unit_view;
use jackioh_engine::state::find_instance;
use jackioh_engine::zones::active_units_of;
use jackioh_engine::{ActionBody, GameState, PlayerId, Position};

/// The evaluation prints with this many decimals.
pub(crate) const EVAL_DECIMALS: usize = 1;

/// What a card a seat cannot see (`redact`'s placeholder, R185) is called.
pub(crate) const HIDDEN_LABEL: &str = "hidden";

/// A card's name, by its def (a transient one first, as `findDef` reads it), else its id; a redacted
/// card's placeholder is `HIDDEN_LABEL`.
pub(crate) fn name_of(state: &GameState, def_id: &str) -> String {
    if def_id == HIDDEN_DEF_ID {
        return HIDDEN_LABEL.to_string();
    }
    state
        .transient_defs
        .get(def_id)
        .or_else(|| jackioh_cards::CATALOG.get(def_id))
        .map_or_else(|| def_id.to_string(), |def| def.name.clone())
}

/// One seat's side of the board on one line: hero, mana, hand and library sizes, its units with
/// their stats and keyword initials, and its backrow.
pub(crate) fn board(state: &GameState, p: PlayerId) -> String {
    let units: Vec<String> = active_units_of(state, p)
        .iter()
        .map(|u| {
            let v = unit_view(state, u);
            let kw: String = v
                .keywords
                .iter()
                .filter_map(|keyword| keyword.kind().as_str().chars().next())
                .collect();
            format!(
                "{}{} {}/{}{}{}",
                name_of(state, &u.def_id),
                if u.radiant { "*" } else { "" },
                v.attack,
                v.health,
                if v.position == Position::Def { " DEF" } else { "" },
                if kw.is_empty() {
                    String::new()
                } else {
                    format!(" [{kw}]")
                }
            )
        })
        .collect();
    let side = &state.players[p];
    let back: Vec<String> = side
        .backrow
        .iter()
        .flatten()
        .map(|card| name_of(state, &card.def_id))
        .collect();
    format!(
        "{p} hp {} mana {}/{} hand {} lib {} | {}{}",
        side.hero.health,
        side.mana.current,
        side.mana.max,
        side.hand.len(),
        side.library.len(),
        units.join(", "),
        if back.is_empty() {
            String::new()
        } else {
            format!(" || {}", back.join(", "))
        }
    )
}

/// The names of the cards in one seat's hand, in hand order.
pub(crate) fn hand_names(state: &GameState, p: PlayerId) -> Vec<String> {
    state.players[p]
        .hand
        .iter()
        .map(|card| name_of(state, &card.def_id))
        .collect()
}

/// An action in words: what is played, what attacks what, what switches; anything else as its JSON.
pub(crate) fn describe(state: &GameState, _seat: PlayerId, action: &ActionBody) -> String {
    match action {
        ActionBody::Play {
            instance_id, targets, ..
        } => {
            let card = find_instance(state, instance_id);
            let targets = match targets {
                Some(targets) => serde_json::to_string(targets).unwrap_or_default(),
                None => String::new(),
            };
            let name = card.map_or_else(|| instance_id.clone(), |card| name_of(state, &card.def_id));
            format!("play {name} {targets}")
        }
        ActionBody::Attack {
            attacker_id,
            target_id,
        } => {
            let attacker = find_instance(state, attacker_id)
                .map_or_else(|| attacker_id.clone(), |card| name_of(state, &card.def_id));
            let target = if target_id.starts_with("hero") {
                target_id.clone()
            } else {
                find_instance(state, target_id)
                    .map_or_else(|| target_id.clone(), |card| name_of(state, &card.def_id))
            };
            format!("attack {attacker} -> {target}")
        }
        ActionBody::SwitchPosition { instance_id } => {
            let name = find_instance(state, instance_id)
                .map_or_else(String::new, |card| name_of(state, &card.def_id));
            format!("switch {name}")
        }
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_ai::redact;
    use jackioh_engine::replay::{ReplayInput, fold};

    #[test]
    fn a_card_the_seat_cannot_see_is_named_hidden_and_its_own_are_named() {
        jackioh_cards::register_all();
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../engine/tests/golden/01-hotseat-full-game.json"
        ))
        .unwrap();
        let mut input: ReplayInput = serde_json::from_str(&text).unwrap();
        input.log.truncate(13);
        let state = fold(&input).state;
        let view = redact(&state, PlayerId::P1);

        let theirs = hand_names(&view, PlayerId::P2);
        assert_eq!(theirs.len(), state.players[PlayerId::P2].hand.len());
        assert!(theirs.iter().all(|name| name == HIDDEN_LABEL), "{theirs:?}");
        let mine = hand_names(&view, PlayerId::P1);
        assert!(!mine.is_empty());
        assert!(mine.iter().all(|name| name != HIDDEN_LABEL), "{mine:?}");
        assert_eq!(mine, hand_names(&state, PlayerId::P1));
        // The true hand is named; the redacted one never is.
        assert!(
            hand_names(&state, PlayerId::P2)
                .iter()
                .all(|name| name != HIDDEN_LABEL)
        );
    }
}
