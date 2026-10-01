// C+ #76 Brother Lar (SPEC §8.7 row 76). (1) Unit, CN, Human, Rare, 1/1 → 2/2.
//   Base:    "Death: Summon a Brother Ping."
//   Radiant: "Death: Summon a Radiant Brother Ping."
//   Engine:  "Brother Ping (C+ #76.1) goes into your leftmost open unit zone (R64): Brother Lar's own zone
//            is free again unless a Reborn reserves it, and a full row summons nothing. Summoned, so no
//            Cry. Tunes: none."
//
// Death fires when Brother Lar goes from the field to a graveyard (§6.2): destroyed or tributed — not
// when it is exiled or bounced. By then its zone is empty, so R64's leftmost open zone may be its own,
// unless a granted Reborn has reserved that zone for Lar's return (R64), when Ping goes elsewhere.

import type { Script } from "@jackioh/engine";
import { summon } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-076");

/** §7, §8.7: the token Brother Lar leaves behind. */
const PING = cardDef("classicplus-076-1").id;

export const base: Script = { death: () => [summon({ defId: PING })] };

export const radiant: Script = { death: () => [summon({ defId: PING, radiant: true })] };
