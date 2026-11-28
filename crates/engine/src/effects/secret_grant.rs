//! A keyword granted in secret (R5238001, FvH #46): the target takes the keyword for real, while the
//! other seat is shown it on the target and on every decoy alike, so it cannot tell which one holds it.
//!
//! The truth is the target's `granted_keywords`, read by every rule and by the controller's view. The
//! disguise is `CardInstance.shown_to_opponent`, on the target and each decoy: the keywords the seat that
//! does not control the card is shown besides its own (`keywords_shown_to`, read by `view_for`'s unit view),
//! and believed by that seat's copies of the state (`believe_secret_grants`: the AI's `redact`, the dry
//! run's `concealed_copy` and the combat forecast). R78's reset takes the disguise off a card leaving the
//! field (`zones::reset_instance`). The grant reports nothing, as a hidden Nerf does (R1845000): a
//! `keywordGranted` naming the target would tell the other seat which card is real. The pick of the target
//! is the card's own choice, made before this verb runs, so the verb opens no prompt.

use serde::{Deserialize, Serialize};

use super::targets::{TargetSpec, resolve_target};
use crate::damage::DamageTarget;
use crate::script::Effect;
use crate::state::{CardInstance, GameState, every_instance, find_instance_mut};
use crate::wire::{Keyword, PlayerId};

/// `grantKeywordSecretly`'s argument: the card that takes the keyword, the cards that only seem to, and
/// the keyword.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SecretGrantArgs {
    pub target: TargetSpec,
    /// The instance ids the other seat is shown the keyword on as well (the target among them or not).
    pub decoys: Vec<String>,
    pub keyword: Keyword,
}

/// R5238001: `keyword` is granted to the card `target` names, as `grant_keyword` grants it (§10.4: a
/// set, Armor and Lucky summing; another card's grant passes an Immutable card by, R5113000), with no
/// event; then the target and every decoy found are marked with it in `shown_to_opponent`, so the seat
/// that does not control each one sees the keyword on all of them, and its controller sees the truth. A
/// target that names no card fizzles the whole verb: nothing is granted and nothing is marked.
pub fn grant_keyword_secretly(args: SecretGrantArgs) -> Effect {
    Effect::new("grantKeywordSecretly", move |ctx| {
        let Some(DamageTarget::Unit { instance }) = resolve_target(ctx, &args.target) else {
            return;
        };
        if !crate::restrictions::immutable_keeps_off(ctx, &instance)
            && let Some(live) = find_instance_mut(ctx.state, &instance.id)
        {
            crate::effects::buff::grant_onto(live, &args.keyword);
        }
        let mut marked: Vec<String> = vec![instance.id.clone()];
        for decoy in &args.decoys {
            if !marked.contains(decoy) {
                marked.push(decoy.clone());
            }
        }
        for id in marked {
            if let Some(card) = find_instance_mut(ctx.state, &id) {
                show_to_opponent(card, &args.keyword);
            }
        }
    })
}

/// R5238001: the disguise on one card: `keyword` joins what the other seat is shown, once per kind.
fn show_to_opponent(card: &mut CardInstance, keyword: &Keyword) {
    let shown = card.shown_to_opponent.get_or_insert_with(Vec::new);
    if !shown.iter().any(|held| held.kind() == keyword.kind()) {
        shown.push(keyword.clone());
    }
}

/// R5238001: the keywords `viewer` is shown on `card`, given those it has (`keywords`): the same to its
/// controller; to the other seat, those with each keyword of its disguise it lacks added after them.
pub fn keywords_shown_to(card: &CardInstance, viewer: PlayerId, keywords: Vec<Keyword>) -> Vec<Keyword> {
    let Some(shown) = card.shown_to_opponent.as_ref() else {
        return keywords;
    };
    if viewer == card.controller {
        return keywords;
    }
    let mut out = keywords;
    for keyword in shown {
        if !out.iter().any(|held| held.kind() == keyword.kind()) {
            out.push(keyword.clone());
        }
    }
    out
}

/// R5238001: whether any card holds a secret grant's disguise, so a copy of the state for one seat must
/// be made to believe it (`believe_secret_grants`).
pub fn any_secret_grant(state: &GameState) -> bool {
    every_instance(state).any(|card| card.shown_to_opponent.is_some())
}

/// R5238001: a copy of the state made for `viewer` believes what `viewer` is shown: each card `viewer`
/// does not control that wears a disguise is given its keywords (granted, once per kind), and every
/// disguise is then dropped, so the copy holds no mark of which card is real. The AI's `redact`, the dry
/// run's `concealed_copy` and `combat_forecast` call it, after `forget_hidden_nerfs`.
pub fn believe_secret_grants(state: &mut GameState, viewer: PlayerId) {
    let marked: Vec<String> = every_instance(state)
        .filter(|card| card.shown_to_opponent.is_some())
        .map(|card| card.id.clone())
        .collect();
    for id in marked {
        let Some(card) = find_instance_mut(state, &id) else {
            continue;
        };
        let shown = card.shown_to_opponent.take().unwrap_or_default();
        if card.controller == viewer {
            continue;
        }
        for keyword in shown {
            if !card
                .granted_keywords
                .iter()
                .any(|held| held.kind() == keyword.kind())
            {
                card.granted_keywords.push(keyword);
            }
        }
    }
}
