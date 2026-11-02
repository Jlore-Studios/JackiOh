// R2789000: the shared callout (docs/polish/9-card-animations.md, "Keyword beats"): a keyword's icon
// (`KEYWORD_CALLOUT_ICONS`) pops over a unit for FX_CALLOUT_MS. Every keyword part of #2782 builds its
// callout here, so the combat keywords' (beats.ts) and the later parts' look and last alike. It names a
// keyword and an anchor, never a card (R202); the DOM draws it (dom.ts, fx.css). `still` is a callout
// under reduced motion: the icon held static, its motion dropped.

import type { KeywordKind } from "@jackioh/shared";

import { FX_CALLOUT_MS } from "../constants.ts";
import type { FxAnchor, FxCalloutCue } from "../types.ts";

/** `keyword`'s icon over `at` from `delayMs`, lasting FX_CALLOUT_MS (its tail is inside FX_MAX_TAIL_MS, R200). */
export function calloutCue(keyword: KeywordKind, at: FxAnchor, delayMs: number, still: boolean): FxCalloutCue {
  return { kind: "callout", keyword, at, delayMs, durationMs: FX_CALLOUT_MS, ...(still ? { still: true as const } : {}) };
}
