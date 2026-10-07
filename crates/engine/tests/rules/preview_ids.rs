//! R280's preview may count a set of cards (`PreviewValue.ids`): Classic+ #44 Simplicity Audit's and
//! #45 Complexity Audit's Radiant "highlight targets" are the permanents the card would exile now
//! (SPEC §8.7 rows 44 and 45). `previewOf` copies the ids beside the label and the value, as it copies
//! `display` (R372), so the view holds its own array and never a reference into the hook's answer.
//!
//! The definitions are test-only, on top of the fixture catalog, put back in `afterAll` as
//! preview.test.ts does.
//!
//! Port of `packages/engine/test/preview-ids.test.ts`. The registries are the testkit's per-thread
//! override (SURFACE §8) and each test runs on its own thread, so TS's `beforeAll`/`afterAll` save and
//! restore have nothing left to do. TS's module `let answered` is a thread-local the hook reads.

use std::cell::Cell;

use jackioh_engine::testkit::PlayerId::P1;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{in_hand, new_game};

fn marker() -> CardDef {
    json_as(json!({
        "id": "pid-marker",
        "index": "2901",
        "name": "marker",
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": "marker" },
        "radiant": { "keywords": [], "text": "marker" },
    }))
}

thread_local! {
    /// The ids the hook answers with, held so a test can mutate them after the view is built.
    static ANSWERED: Cell<Vec<String>> = const { Cell::new(Vec::new()) };
}

fn answered() -> Vec<String> {
    ANSWERED.with(|cell| {
        let ids = cell.take();
        cell.set(ids.clone());
        ids
    })
}

fn set_answered(ids: &[&str]) {
    ANSWERED.with(|cell| cell.set(ids.iter().map(|id| id.to_string()).collect()));
}

fn hook() -> PreviewHook {
    condition_hook(|_ctx: ConditionContext<'_>| {
        let ids = answered();
        vec![
            PreviewValue {
                label: "marker".to_string(),
                value: ids.len() as i32,
                display: None,
                ids: Some(ids),
            },
            PreviewValue {
                label: "plain".to_string(),
                value: 1,
                display: None,
                ids: None,
            },
        ]
    })
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        marker().id,
        CardScripts {
            base: Script {
                preview: Some(hook()),
                ..Script::default()
            },
            radiant: Script {
                preview: Some(hook()),
                ..Script::default()
            },
        },
    );
    scripts
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    catalog.insert(marker().id, marker());
    register_catalog(catalog);
    let mut all = registered_scripts().clone();
    all.extend(scripts());
    register_scripts(all);
    state.turn = 3;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

fn hand_card(view: &PlayerView, instance_id: &str) -> CardView {
    let HandView::Cards(hand) = &view.you.hand else {
        panic!("the viewer's own hand travels in full");
    };
    hand.iter()
        .find(|card| card.instance_id == instance_id)
        .cloned()
        .unwrap_or_else(|| panic!("{instance_id} is not in the hand"))
}

/// R280 a preview value may carry the set of cards it counts
mod r280_a_preview_value_may_carry_the_set_of_cards_it_counts {
    use super::*;

    #[test]
    fn r280_the_view_carries_the_ids_beside_the_label_and_the_value_and_a_value_without_ids_carries_none() {
        let mut state = game("preview-ids");
        let card = in_hand(&mut state, &marker().id, P1, 1)
            .into_iter()
            .next()
            .expect("the marker in hand");
        set_answered(&["c1", "c2"]);

        let preview = hand_card(&view_for(&state, P1), &card.id).preview;

        assert_eq!(
            preview,
            Some(vec![
                PreviewValue {
                    label: "marker".to_string(),
                    value: 2,
                    display: None,
                    ids: Some(vec!["c1".to_string(), "c2".to_string()]),
                },
                PreviewValue {
                    label: "plain".to_string(),
                    value: 1,
                    display: None,
                    ids: None,
                },
            ])
        );
    }

    #[test]
    fn r280_the_view_holds_a_copy_of_the_ids_never_the_hook_s_own_array() {
        let mut state = game("preview-ids-copy");
        let card = in_hand(&mut state, &marker().id, P1, 1)
            .into_iter()
            .next()
            .expect("the marker in hand");
        set_answered(&["c7"]);

        let preview = hand_card(&view_for(&state, P1), &card.id).preview;
        set_answered(&["c7", "c8"]);

        assert_eq!(
            preview
                .as_ref()
                .and_then(|values| values.first())
                .and_then(|value| value.ids.clone()),
            Some(vec!["c7".to_string()])
        );
    }
}
