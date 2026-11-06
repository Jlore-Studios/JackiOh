// The two-letter marks of a unit's keyword chips (MinionFace.tsx). Display only: the words a keyword is called
// in, and its rule, are the face model's (jackioh-shared).

import type { KeywordKind } from "@jackioh/shared";

/** Two-letter marks rather than glyphs, so a snapshot is stable and a screen reader gets the name. */
export const KEYWORD_MARK: Readonly<Record<KeywordKind, string>> = {
  Taunt: "TA",
  Rush: "RU",
  Charge: "CH",
  "First Strike": "FS",
  Poisonous: "PO",
  Lifesteal: "LS",
  Reborn: "RB",
  "Divine Shield": "DS",
  Trample: "TR",
  Cleave: "CL",
  Pierce: "PI",
  Indestructible: "ND",
  Immutable: "IM",
  Stack: "ST",
  "Can't attack": "NA",
  Armor: "AR",
  Lucky: "LK",
  Animated: "AN",
  "Animated on your turn": "AT",
  Brittle: "BR",
  "Spell Damage": "SD",
  "Immune to Spells": "IS",
  Windfury: "WF",
  Temporary: "TE",
  Deft: "DE",
  Untributable: "UT",
  Equip: "EQ",
  Ranged: "RG",
  Flying: "FL",
  Cloaked: "CK",
  Crater: "CR",
  Flashback: "FB",
  Tradable: "TD",
  "Starship Piece": "SP",
  Starship: "SS",
  "Can't Defend": "CD",
};
