//! ME-GRANT (docs/meditative-set.md M5, MD-D13): granted Death abilities, held as plain data.
//!
//! Meditative #58 and #59 grant the Unit they summon a Death ability — #58's "Death: Add {books}
//! random Book cards to your hand", #59's "Death: Fuse a random Book and AI card". The grant is a
//! hook the granting card registers beside its faces (`Script.grants`, keyed `<key>`); the
//! instance carries only the name, the face it was granted on and the numbers it was granted with
//! (`CardInstance.grants`). A closure never enters state, so state stays JSON and replays exactly,
//! and a resume rebuilds the same Death list from the snapshot alone. A grant behaves like a
//! granted keyword (§10.4): it shows on the Unit (`grant_texts`, read by `UnitView.grants`), survives
//! a copy (R57) and a Vanilla, and is lost when the card leaves the field (R78).

use indexmap::IndexMap;
use serde_json::{Value, json};

use crate::script::{EffectContext, Hook, hook};
use crate::state::{CardInstance, GameState};

/// The `data` key a granted Death runs under: the grant's JSON, so `grant_param` reads its numbers.
pub const GRANT_DATA_KEY: &str = "__grant";

/// The hook the grant `<defId>#<key>` names: the granting card's face `radiant` registers under
/// `key`. `None` when the name is malformed or the face registers nothing under it (a catalog the
/// grant outlived): the grant then does nothing, rather than a missing hook failing the game.
pub fn grant_hook(state: &GameState, def_id: &str, radiant: bool, key: &str) -> Option<Hook> {
    crate::scripts::face_ref(state, def_id, radiant)
        .grants
        .get(key)
        .cloned()
}

/// §4.5 step 3 (MD-D13): the Death hook the dying card runs — its own Death effects, then each
/// granted Death in the order granted. Built from the snapshot alone (R78's last-known state), so
/// a resume rebuilds the same list. Each grant's list runs with the grant's JSON inserted under
/// `GRANT_DATA_KEY`, which `grant_param` reads; the caller's data is restored afterwards, so an own
/// Death beside grants reads its data as before.
pub fn death_hook_of(state: &GameState, snapshot: &CardInstance) -> Option<Hook> {
    let own = crate::scripts::script_of(state, snapshot).death.clone();
    let grants: Vec<crate::state::Grant> = snapshot.grants.clone().unwrap_or_default();
    if grants.is_empty() {
        return own;
    }
    Some(hook(move |ctx| {
        let mut out = Vec::new();
        if let Some(own) = own.clone() {
            out.extend(own(ctx));
        }
        for grant in &grants {
            let Some((def_id, key)) = grant.grant.split_once('#') else {
                continue;
            };
            let Some(hook) = grant_hook(ctx.state, def_id, grant.radiant, key) else {
                continue;
            };
            let staged = json!({
                "grant": grant.grant,
                "radiant": grant.radiant,
                "params": grant.params,
            });
            let saved = ctx.data.insert(GRANT_DATA_KEY.to_string(), staged);
            out.extend(hook(ctx));
            match saved {
                Some(value) => {
                    ctx.data.insert(GRANT_DATA_KEY.to_string(), value);
                }
                None => {
                    ctx.data.shift_remove(GRANT_DATA_KEY);
                }
            }
        }
        out
    }))
}

/// ME-GRANT: the number the running grant was granted with — `ctx.data[__grant].params[key]`, the
/// face's printed value when the grant carries none. A grant written without its numbers still reads
/// its face's number, never 0 by accident.
pub fn grant_param(ctx: &EffectContext<'_>, key: &str) -> i32 {
    let grant = ctx.data.get(GRANT_DATA_KEY);
    if let Some(hit) = grant
        .and_then(|grant| grant.get("params"))
        .and_then(|params| params.get(key))
        .and_then(Value::as_i64)
    {
        return hit as i32;
    }
    grant
        .and_then(|grant| grant.get("grant"))
        .and_then(Value::as_str)
        .and_then(|name| name.split_once('#'))
        .and_then(|(def_id, _)| {
            let radiant = grant
                .and_then(|grant| grant.get("radiant"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            crate::catalog::find_def(Some(ctx.state), def_id).and_then(|def| {
                def.params.as_ref().and_then(|params| {
                    params
                        .iter()
                        .find(|param| param.key == key)
                        .map(|param| if radiant { param.radiant } else { param.base })
                })
            })
        })
        .unwrap_or(0)
}

/// ME-GRANT: the granted abilities as the Unit's lines read — for each grant, the granting face's
/// `grants[key]` text with the grant's numbers filled in. `None` when the card holds no grants.
/// A grant whose face is gone reads its name, never failing the view.
pub fn grant_texts(state: &GameState, card: &CardInstance) -> Option<Vec<String>> {
    let grants = card.grants.as_ref().filter(|grants| !grants.is_empty())?;
    let mut out = Vec::new();
    for grant in grants {
        let Some((def_id, key)) = grant.grant.split_once('#') else {
            continue;
        };
        let Some(def) = crate::catalog::find_def(Some(state), def_id) else {
            continue;
        };
        let face = if grant.radiant {
            crate::wire::FaceKind::Radiant
        } else {
            crate::wire::FaceKind::Base
        };
        let Some(text) = def.face(face).grants.as_ref().and_then(|grants| grants.get(key)) else {
            continue;
        };
        let values: IndexMap<String, i32> = grant.params.clone();
        out.push(crate::wire::fill_text(text, def, face, Some(&values)));
    }
    if out.is_empty() { None } else { Some(out) }
}
