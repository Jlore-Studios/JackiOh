// The only things the dashboard takes from apps/web, both from its wire layer and both data, no code
// that runs: the server's own numbers (serverConfig.ts, generated from crates/server/src/config.rs, so
// the dashboard restates none, CLAUDE.md rule 9) and the sets that ship (catalog.ts's SHIPPED_SETS, the
// list the web's Dev tools offer a set track from). Neither module imports anything.

export {
  API_REQUEST_TIMEOUT_SECONDS,
  AUTH_PROVIDER_TIMEOUT_SECONDS,
  AUTH_SESSION_REFRESH_MARGIN_SECONDS,
  AUTH_SESSION_RENEWAL_FLOOR_SECONDS,
} from "../../web/src/wire/serverConfig.ts";
export { SHIPPED_SETS, type SetName } from "../../web/src/wire/catalog.ts";

/** Unit conversion, not configuration. */
export const MS_PER_SECOND = 1000;
