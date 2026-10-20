//! The cosmetics an account owns (SPEC §14.2, SPEC §11 R1313000): read-only, and only the caller's
//! own, keyed on the verified token's profile and never on a field of the request. The client
//! reads it for the full-art faces its own copies print once the economy is live; an opponent's
//! ownership never travels here (CLAUDE.md rule 7).

use std::sync::Arc;

use crate::api::collection::caller_profile;
use crate::api::http::{ApiResult, Req, ok_of};
use crate::app::App;

/// `GET /api/cosmetics` (`AuthLevel::Active`): `{ cards: [{ cardId, cosmeticId }], account: [id] }`,
/// the per-card cosmetics by card then cosmetic and the account cosmetics by id. The starter
/// cosmetics, owned by every account, have no row and are not listed.
pub async fn get_owned_cosmetics(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let owned = tx.economy_cosmetics_get(&profile.id).await?;
    tx.commit().await?;
    ok_of(&owned)
}
