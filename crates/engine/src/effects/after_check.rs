//! "…, then …" after the deaths a list's destroys caused (SPEC §4.5, R59, R113, R174): Classic #43
//! Plague Nuke's "Destroy all Units. Gain 1 mana for each Plague Counter that was on them. Then summon …".
//!
//! A destroy only marks, and §4.5's check follows the whole effect (R59), so nothing later in the same
//! list sees them dead. `after_state_check` runs the check mid-list (one check, §4.5 steps 1–5), then
//! builds the rest of the text and applies it on a fresh stay mark (R174): the run's own `exits_from`
//! calls a card the check moved gone wherever it is, and a Reborn body the check put back is a new
//! arrival (R83).
//!
//! A Death hook in the check may ask something (R113): the check effect is the part's first entry and
//! the rest its second, so a resume re-enters at the rest (`prompts::run_resumable_list`), built after
//! the answer and run once; the mark is kept as the rest's memo.

use std::sync::Arc;

use serde_json::Value;

use crate::resolve::lazy_part;
use crate::script::{Effect, EffectContext, EffectExpand, EffectPart, Memo};
use crate::state_check::state_check;
use crate::stays::exit_mark;

/// `after_state_check`'s `build`: the rest of the text, built once the check is over.
pub type AfterCheckBuild = Arc<dyn Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync>;

/// Run `f` on the context with its `exits_from` set to `exits_from`, then put the context's own mark
/// back.
fn with_exits_from<R>(
    ctx: &mut EffectContext<'_>,
    exits_from: u32,
    f: impl FnOnce(&mut EffectContext<'_>) -> R,
) -> R {
    let before = ctx.exits_from.replace(exits_from);
    let out = f(ctx);
    ctx.exits_from = before;
    out
}

/// The same effect, applied — and, for a part, built and its entries applied — with the fresh mark.
fn at_mark(effect: Effect, exits_from: u32) -> Effect {
    let apply = effect.apply.clone();
    let expand: Option<EffectExpand> = effect.expand.clone().map(|expand| -> EffectExpand {
        Arc::new(move |ctx: &mut EffectContext<'_>, memo: &Memo| {
            let built = with_exits_from(ctx, exits_from, |ctx| expand(ctx, memo));
            EffectPart {
                effects: built
                    .effects
                    .into_iter()
                    .map(|inner| at_mark(inner, exits_from))
                    .collect(),
                memo: built.memo,
            }
        })
    });
    Effect {
        kind: effect.kind,
        apply: Arc::new(move |ctx: &mut EffectContext<'_>| {
            with_exits_from(ctx, exits_from, |ctx| apply(ctx));
        }),
        expand,
    }
}

/// The outer part's memo: its shape is fixed, so it only needs to be JSON.
const PART_MEMO: &str = "afterStateCheck";

/// §4.5 at this point of the list: collect and resolve everything marked so far.
fn check_now() -> Effect {
    Effect::new("stateCheckNow", |ctx| {
        if ctx.state.result.is_some() {
            return;
        }
        state_check(ctx);
    })
}

/// §4.5, R59: run the state check here, then build the rest of the text with `build` and apply it,
/// on a stay that begins after the check (R174). `build` is called once the check is over — after the
/// answer, when a Death hook in it asked (R113) — with a context whose `exits_from` is that stay's.
pub fn after_state_check(
    build: impl Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync + 'static,
) -> Effect {
    let build: AfterCheckBuild = Arc::new(build);
    lazy_part(
        "afterStateCheck",
        move |_ctx: &mut EffectContext<'_>, _memo: &Memo| {
            let build = build.clone();
            EffectPart {
                effects: vec![
                    check_now(),
                    lazy_part(
                        "afterStateCheck:rest",
                        move |ctx: &mut EffectContext<'_>, memo: &Memo| {
                            let exits_from = match memo {
                                Some(value @ Value::Number(_)) => value.as_f64().map(|mark| mark as u32),
                                _ => None,
                            }
                            .unwrap_or_else(|| exit_mark(ctx.state));
                            let effects = with_exits_from(ctx, exits_from, |ctx| build(ctx))
                                .into_iter()
                                .map(|effect| at_mark(effect, exits_from))
                                .collect();
                            EffectPart {
                                effects,
                                memo: Some(Value::from(exits_from)),
                            }
                        },
                    ),
                ],
                // A memo that survives JSON as it is (a pause records one per part it stands in, and a bare
                // `undefined` there would come back from a round trip as `null`); the part's shape needs none.
                memo: Some(Value::String(PART_MEMO.to_string())),
            }
        },
    )
}
