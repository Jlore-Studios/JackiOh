//! What a card view carries of the card's instance data (docs/classic-sets.md B2.7, B3.3, B3.4, B5 E38,
//! E39; R243's "the view carries what a card is made of"): its type where a face gives it one of its
//! own, its Brittle count, its declared numbers as they stand, what Degrade, Upgrade and KY's Constant
//! changed, its enchantments, and — on a hand card — the keywords it has gained there.
//!
//! `view_for` builds every card view it shows through `card_view`, and only for a card the viewer may
//! read there (the viewer's own hand, a unit, a face-up backrow card or the controller's face-down
//! one, a graveyard, an exile pile, the resolving zone), so none of this reaches a player who may not
//! read the card: the opponent's hand and every library travel as counts, a face-down trap as its
//! zone and cost (R351), and the owner's library list reads what the owner was shown going in, never
//! the card (R311), so a change made inside a deck shows once the card leaves it. Every key is absent
//! when it has nothing to say, so a card with no instance data views exactly as before patch v0.2.0.
//!
//! Port of `packages/engine/src/instanceView.ts`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::state::{CardInstance, GameState};
use crate::wire::{CardType, CardView, Enchantment, Keyword, Tuning, keyword_key};

/// TS `Pick<CardView, "type" | "brittle" | "params" | "tuning" | "enchantments">`: the instance-data
/// keys of a card view, each `None` when it has nothing to say. Serialises as those keys of
/// `CardView` do, so it can be merged into one key for key (`apply_to`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstanceData {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<CardType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brittle: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<IndexMap<String, i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tuning: Option<Tuning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enchantments: Option<Vec<Enchantment>>,
}

impl InstanceData {
    /// TS `{ ...view, ...instanceDataView(state, card) }`: every key this holds overwrites the view's,
    /// and a key it does not hold leaves the view's as it was (a spread copies only present keys).
    pub fn apply_to(self, view: &mut CardView) {
        if let Some(type_) = self.type_ {
            view.type_ = Some(type_);
        }
        if let Some(brittle) = self.brittle {
            view.brittle = Some(brittle);
        }
        if let Some(params) = self.params {
            view.params = Some(params);
        }
        if let Some(tuning) = self.tuning {
            view.tuning = Some(tuning);
        }
        if let Some(enchantments) = self.enchantments {
            view.enchantments = Some(enchantments);
        }
    }
}

/// The instance-data keys of a card view (this file's header), each only when it has something to say.
pub fn instance_data_view(state: &GameState, card: &CardInstance) -> InstanceData {
    let mut out = InstanceData::default();
    // B2.7: the type now, where the running face's differs from the definition's (Blood Moon's Radiant).
    let type_ = crate::faces::card_type_of(state, card);
    if type_ != crate::catalog::def_of(Some(state), &card.def_id).type_ {
        out.type_ = Some(type_);
    }
    // B3.3 rule 6: public on the field, the owner's in a hand — both are where this view is built.
    let brittle = crate::brittle_count::active_brittle_count(card);
    if let Some(brittle) = brittle {
        out.brittle = Some(brittle);
    }
    // B3.4 rule 5, R386: the declared numbers the client fills the face's `{key}`s with.
    let params = crate::params::params_view(state, card);
    if let Some(params) = params {
        out.params = Some(params);
    }
    // B3.4 rule 7: what changed, for the client's Degrade and Upgrade marks. (TS `copyTuning`, a JSON
    // deep copy, is a clone here.)
    let tuning = if crate::tuning::is_tuned(card) {
        card.tuning.clone()
    } else {
        None
    };
    if let Some(tuning) = tuning {
        out.tuning = Some(tuning);
    }
    // E39. (TS `enchantmentsOf(card)` is `card.enchantments ?? []`.)
    let enchantments: &[Enchantment] = card.enchantments.as_deref().unwrap_or(&[]);
    if !enchantments.is_empty() {
        out.enchantments = Some(enchantments.to_vec());
    }
    out
}

fn same_keywords(a: &[Keyword], b: &[Keyword]) -> bool {
    // A comparator-less `.sort()` on strings (SURFACE §4.4.1): a plain sort of the keys.
    let key = |list: &[Keyword]| -> String {
        let mut keys: Vec<String> = list.iter().map(keyword_key).collect();
        keys.sort();
        keys.join("|")
    };
    key(a) == key(b)
}

/// B5 E38, R243: a hand card's keywords as it will carry them onto the field — its printed ones as
/// tuning leaves them and the ones it was granted in the hand or the deck (`layers::card_keywords`) —
/// set only where they differ from its running face's printed keywords, which the client reads off
/// the definition. `None` otherwise.
pub fn hand_keywords_view(state: &GameState, card: &CardInstance) -> Option<Vec<Keyword>> {
    let now: Vec<Keyword> = crate::layers::card_keywords(state, card);
    let face = crate::faces::running_face(state, card);
    if same_keywords(&now, &face.keywords) {
        None
    } else {
        Some(now)
    }
}
