//! "If you have a Dormant Starship, launch it, then Discover a War Doctrine and cast it": the text S #137
//! Starfleet Commander's Cry and S #176 Preemptive Strike share (SPEC §8.10 rows 137 and 176; R70, R113,
//! R1839002, R1852003, R1852006, R1857002, R1859006, R3869001, R5106000). Each card's script builds its
//! faces from these, its Discover's prompt naming the card.
//!
//! R5106000: the Dormant Starship is read once, as the text begins (`for_each_card`'s memo, R113), and
//! the launch and the Discover are one part rebuilt from that read, so a fused Cry that asks during the
//! launch (S #40 Poison Capsule's placement) is answered and the Discover follows. Before issue #5106
//! (`Read::EachBuild`, each card's `script_before_launch_read_once`, R767000) the read was made again each
//! time the list was built, and the rebuild after that answer found the Starship launched, so the
//! Discover was lost. The text lives here, outside both card files, so neither card's script crosses the
//! complicated line count (R1846000, R2173001).

use jackioh_engine::prelude::*;

/// The resume step the Discover's answer comes back to (§10.6).
pub(crate) const PICKED: &str = "picked";

/// When the text reads its Dormant Starship (R5106000).
#[derive(Clone, Copy)]
pub(crate) enum Read {
    /// Once, as the text begins: the launch and the Discover rebuilt from that read.
    Once,
    /// Before issue #5106: again each time the list was built (R767000).
    EachBuild,
}

/// The face's words for the pick, "a War Doctrine" or "a Radiant War Doctrine".
pub(crate) fn doctrine_text(radiant: bool) -> &'static str {
    if radiant {
        "a Radiant War Doctrine"
    } else {
        "a War Doctrine"
    }
}

/// The text, the Discover asking `prompt` and answering at `PICKED` (R1852006, R1859006). With no
/// Dormant Starship, nothing: no launch and no Discover.
pub(crate) fn launch_then_discover(ctx: &EffectContext<'_>, prompt: String, read: Read) -> Vec<Effect> {
    match read {
        Read::Once => vec![for_each_card(ForEachCardArgs {
            cards: Arc::new(|at: &mut EffectContext<'_>| -> Vec<String> {
                dormant_starship_of(&*at.state, at.controller)
                    .map(|ship| ship.id.clone())
                    .into_iter()
                    .collect()
            }),
            each: Arc::new(move |ship: &str| {
                let ship = ship.to_string();
                let prompt = prompt.clone();
                jackioh_engine::lazy_part("launchThenDiscover", move |_ctx, _memo| EffectPart {
                    effects: launch_and_discover(&ship, &prompt),
                    memo: None,
                })
            }),
        })],
        Read::EachBuild => match dormant_starship_of(&*ctx.state, ctx.controller) {
            Some(ship) => launch_and_discover(&ship.id, &prompt),
            None => vec![],
        },
    }
}

/// `awaken` the Starship `ship`, which launches it for nothing, spending no launch price (R1839002,
/// R1852003, R1852006), then the Discover of three of the War Doctrine pool (R1857002, R3869001).
fn launch_and_discover(ship: &str, prompt: &str) -> Vec<Effect> {
    vec![
        awaken(json_as(
            json!({ "target": { "of": "instance", "instanceId": ship } }),
        )),
        discover_from_catalog(json_as(json!({
            "step": PICKED,
            "query": crate::query::war_doctrines(),
            "prompt": prompt,
        }))),
    ]
}

/// The resume step at `PICKED`: the pick cast free and counted as played (R70), Radiant on the Radiant
/// face.
pub(crate) fn cast_the_pick(radiant: bool) -> Hook {
    hook(move |ctx| {
        let Some(picked) = chosen_options(ctx).into_iter().next() else {
            return vec![];
        };
        vec![cast_new(json_as(json!({ "def": picked, "radiant": radiant })))]
    })
}
