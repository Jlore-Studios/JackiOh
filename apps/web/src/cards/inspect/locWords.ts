// E36: a card's lines of code, in the inspect overlays' meta and state notes ("27 lines of code").

import { t } from "../../i18n/i18n.ts";

export function locWords(loc: number): string {
  return loc === 1 ? t("{n} line of code", { n: loc }) : t("{n} lines of code", { n: loc });
}
