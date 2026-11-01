// A match socket's state in a player's words: the match bar's and the Duos screen's, whose DuosConnection names the same five states (#2644).

import { t } from "../i18n/i18n.ts";
import type { ConnectionState } from "./net.ts";

/** A connection state in a player's words, for the match bar and the wait before the board. */
export function connectionWords(state: ConnectionState): string {
  switch (state) {
    case "open":
      return t("Connected");
    case "connecting":
      return t("Connecting…");
    case "reconnecting":
      return t("Reconnecting…");
    case "refused":
      return t("This match isn't available");
    case "closed":
      return t("Disconnected");
  }
}
