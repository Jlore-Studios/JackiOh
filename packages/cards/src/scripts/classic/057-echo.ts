// C #57 Echo (SPEC §8.6 row 57, BUILD M9 Classic row C 57). (1) Spell, Epic.
//   Base:    "This has the text of the last Spell either player played."
//   Radiant: "Echo 1 / This has the text of the last Spell either player played."
//   Engine:  "The per-game record of the last Spell played by anyone (§10.1), `lastSpell = { defId,
//            radiant }`, overwritten by every Spell play and cast (R70) and never cleared; a countered
//            Spell was never played and is not recorded. In hand, Echo's view carries that Spell's text
//            on the face it was played on, under Echo's name (as R243 carries a fused card's); played,
//            Echo declares and resolves that Spell's choices and script. Echo keeps its own name, type
//            and (1) Cost. A played Echo records the Spell it copied, never Echo, so it can't copy itself
//            into a loop; with no Spell played yet it has no text and does nothing (R399). … Radiant:
//            plus Echo 1 (§6.2), which repeats the copied text. Tunes: Radiant Echo 1 ↑, a numbered
//            keyword that Degrade and Upgrade move as an X (R386), not a `params` entry."
//
// The card is a flag and a record; the text it has is the engine's B5 E14 subsystem
// (`engine/src/subsystems/copiedText.ts`), with its own engine tests through fixture cards:
//   - `copiesLastSpell` makes the running text the last Spell's face (R399): its declared targets,
//     modes and X (R545: chosen up to the mana left once Echo's own (1) is paid), its resolution with
//     Echo as "this", its prompt continuations and its declared numbers (`param`), its Echo X, its
//     Cast on draw, its `preview` and `conditionMet` (R546, R547), and the owner's hand view
//     (`CardView.copies`).
//   - The copy is fixed as the play begins and kept through the play (R546); in hand and in a deck it
//     follows the record live.
//   - `recordsPlayAs` (B5 E4) records the Spell Echo copied, on its face, and nothing when it copied
//     nothing, so two Echoes never loop (R399).
//   - The Radiant face's "Echo 1" is §6.2's Echo X (`staticFlags.echo`), a numbered keyword Degrade and
//     Upgrade move (R386), added to any Echo the copied face prints (R546). Its number is a keyword's,
//     not a declared `params` entry, so nothing here reads `param`.
// "Echo" is also a rules word: the "Echo 1" printed on other cards is never a reference to this card
// (R381, `test/references.test.ts`).

import type { Script } from "@jackioh/engine";
import { subsystems } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-057");

/** B5 E4, R399: the Spell Echo copied, on its face, or nothing when it copied nothing. */
const recordsPlayAs: Script["recordsPlayAs"] = ({ state, self }) => subsystems.copiedTextOf(state, self);

export const base: Script = {
  staticFlags: { copiesLastSpell: true },
  recordsPlayAs,
};

/** §6.2: Echo 1 — the copied text resolves once more, with fresh prompts. */
export const radiant: Script = {
  staticFlags: { copiesLastSpell: true, echo: 1 },
  recordsPlayAs,
};
