//! SPEC §8.10 Stinkyscape 169 Tiny Asteroid, a card of the Stinkyscape set.
//!   Base:    "Can't attack\nCan't be in Defense Position.\nDeath: Gain {resources|random Stinkyscape
//!            Resource|random Stinkyscape Resources}.\nActivate: Take {selfDamage} damage. Tribute
//!            this." (0/4; 1, 2)
//!   Radiant: "… Death: Gain {resources|random Radiant Stinkyscape Resource|…}.\nActivate: Tribute
//!            this." (0/8; 1)
//!
//! A mining node (R1840006): Can't attack is printed, and Can't Defend (R2657000) keeps it out of
//! Defense Position (§4.1). Activate (§6.2, R384), once a turn: its controller's hero takes `selfDamage`
//! (§4.4), then a Sacrifice of it (§6.3), a death, so its Death runs: `gain_resources` of `resources`
//! random Resources into its controller's hand (R1840000, R1840002). Any other death pays too; an exile
//! or a Bounce nothing. Radiant: the Activate is the Sacrifice alone, and the Resources are Radiant.
//! The numbers are declared (R386), `selfDamage` on the base face only (R1431).

use jackioh_engine::effects::{damage, gain_resources, sacrifice};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-169";

/// The Activate's id.
const MINE: &str = "mine";

/// "Activate: Take {selfDamage} damage. Tribute this.", the damage dropped on the Radiant face.
fn mine(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: MINE.to_string(),
        label: if radiant {
            "Tribute this"
        } else {
            "Take damage, then Tribute this"
        }
        .to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            let tribute = sacrifice(json_as(json!({ "target": { "of": "self" } })));
            if radiant {
                vec![tribute]
            } else {
                vec![
                    damage(json_as(
                        json!({ "to": { "of": "selfHero" }, "amount": param(&*ctx, "selfDamage") }),
                    )),
                    tribute,
                ]
            }
        }),
    }
}

/// R1855002, R2476000: on the field, to both seats, the pool an Anomaly board wrote on it (its rider,
/// `RESOURCE_POOL_KEY`), which its Death's Resources come from in place of every Stinkyscape one: by
/// name after "random Stinkyscape Resource" (Radiant: "random Radiant Stinkyscape Resource"), its value
/// the pool's size. Nothing for an Asteroid with no rider, which gains what its text says. The board
/// that wrote it laid its cards in public. S #170, #171 and #172 show the same.
pub fn pool_preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Field {
            return vec![];
        }
        let Some(pool) = ctx
            .self_
            .memory
            .get(RESOURCE_POOL_KEY)
            .and_then(|value| serde_json::from_value::<Vec<String>>(value.clone()).ok())
        else {
            return vec![];
        };
        let names: Vec<String> = pool
            .iter()
            .map(|def_id| def_of(Some(ctx.state), def_id).name.clone())
            .collect();
        let label = if ctx.radiant {
            "random Radiant Stinkyscape Resource"
        } else {
            "random Stinkyscape Resource"
        };
        vec![PreviewValue {
            label: label.to_string(),
            value: names.len() as i32,
            display: Some(names.join(" or ")),
            ids: None,
        }]
    })
}

fn face(radiant: bool) -> Script {
    Script {
        activations: vec![mine(radiant)],
        death: Some(hook(move |ctx| {
            vec![gain_resources(json_as(
                json!({ "count": param(&*ctx, "resources"), "radiant": radiant }),
            ))]
        })),
        preview: Some(pool_preview()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

/// R767000: through v0.5.9f "Can't be in Defense Position" was the static flag `never_defense` on both
/// faces, before R2657000 made it the keyword Can't Defend; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_9f() -> CardScripts {
    crate::never_defense_kept(script(), true, true)
}

// S 169 Tiny Asteroid — SPEC §8.10 row 169, BUILD M12 row S 169.
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::stinkyscape_family as fam;

    const SELF_DAMAGE: i32 = 2;
    const RESOURCES: usize = 1;

    #[test]
    fn r1840006_activate_damages_then_tributes_and_pays() {
        fam::asteroid_activate_pays(ID, SELF_DAMAGE, RESOURCES);
    }

    #[test]
    fn destroyed_in_combat_or_by_twisting_nether_it_pays() {
        fam::asteroid_death_pays(ID, RESOURCES);
    }

    #[test]
    fn never_attacks_or_switches() {
        fam::asteroid_never_attacks(ID);
    }

    #[test]
    fn resources_and_self_damage_read_through_param() {
        fam::asteroid_params(ID);
    }

    #[test]
    fn radiant_no_damage_radiant_resources() {
        fam::asteroid_radiant(ID, RESOURCES);
    }

    /// R2476000, R1855002: with no rider it previews nothing; with an Anomaly board's rider it names the
    /// pool after "random Stinkyscape Resource" (Radiant: "random Radiant Stinkyscape Resource"), to
    /// both seats, and its Death's Resources come from that pool alone.
    #[test]
    fn r2476000_it_names_the_resource_pool_its_rider_holds() {
        fam::asteroid_rider_pool(ID);
    }
}
