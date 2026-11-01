// #2646: a short choice's options are choice cards, and a choice card's text is its option's detail
// (modeText.ts). A card's Choose one of DISCOVER_OPTION_LIMIT options or fewer whose option has no
// words in MODE_TEXT would draw a card that says nothing, so these hold the map to every such option:
// the ones the card scripts declare, which crates/tools/tests/export_card_modes.rs writes into
// cardModes.json, and the ones an engine prompt asks as a card resolves, built at run time and so
// listed here by hand, each from its script.

import { SHIPPED_SETS } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import CARD_MODES from "./cardModes.json";
import { DISCOVER_OPTION_LIMIT } from "./contract.ts";
import { modeText } from "./modeText.ts";

type Face = "base" | "radiant";

/** The card scripts' declared Choose ones, by card and face; `"//"` says where the file comes from. */
const DECLARED = CARD_MODES as unknown as Readonly<Record<string, string | Partial<Record<Face, readonly (readonly string[])[]>>>>;

/** A Choose one the picker draws as another picker: a number pad (R247) or the direction arrows (R14). */
function drawnOtherwise(options: readonly string[]): boolean {
  return options.every((option) => /^\d+$/.test(option)) || options.every((option) => option === "left" || option === "right");
}

/** Every option of `lists` that a choice card would draw with no words: `"<id> <face> <option>"`. */
function wordless(id: string, face: Face, lists: readonly (readonly string[])[]): string[] {
  return lists
    .filter((options) => options.length <= DISCOVER_OPTION_LIMIT && !drawnOtherwise(options))
    .flatMap((options) => options.filter((option) => modeText(id, option, face === "radiant").detail === undefined).map((option) => `${id} ${face} ${option}`));
}

/**
 * The options an engine Choose one offers (`choose_mode`), by the card asking and its faces. Each is
 * the script's or the subsystem's own list, the whole of what it may offer.
 */
const ENGINE_MODES: readonly { id: string; faces: readonly Face[]; options: readonly string[] }[] = [
  // crates/cards/src/scripts/classic/c008_pickle.rs: DISCARD, EXILE, DRAW.
  { id: "classic-008", faces: ["base", "radiant"], options: ["discard", "exile", "draw"] },
  // crates/cards/src/scripts/core/c051_kys_private_tutor.rs: TypeOption, then BracketOption.
  { id: "core-051", faces: ["base", "radiant"], options: ["Spell", "Unit", "Field Spell", "Trap"] },
  { id: "core-051", faces: ["base", "radiant"], options: ["0-1", "2", "3", "4+"] },
  // crates/cards/src/scripts/core/c065_masochism_mask.rs: base_options, radiant_options ("lose" and its `loss`).
  { id: "core-065", faces: ["base"], options: ["exile bottom", "lose 3", "summon Spikey Pillow"] },
  { id: "core-065", faces: ["radiant"], options: ["nothing", "exile bottom", "lose 3", "summon Spikey Pillow"] },
  // crates/cards/src/scripts/core/c082_kys_trial.rs: three of SHIPPED_SETS.
  { id: "core-082", faces: ["base", "radiant"], options: SHIPPED_SETS },
  // crates/cards/src/scripts/meditative/c085_playtester.rs: v0.5.2's BUFF, NERF, kept for its replays.
  { id: "meditative-085", faces: ["base", "radiant"], options: ["Book of Buff", "Book of Nerf"] },
  // crates/cards/src/scripts/stinkyscape/c148_1_edge_of_the_universe.rs: DRAW, LOSS.
  { id: "stinkyscape-148-1", faces: ["radiant"], options: ["Draw", "Loss"] },
  // crates/engine/src/subsystems/ky_test.rs: KY_TEST_DIFFICULTIES.
  { id: "classicplus-042", faces: ["base", "radiant"], options: ["Easy", "Medium", "Hard"] },
  // crates/cards/src/scripts/stinkyscape/c018_1_melchior.rs: three of RANDOM_KEYWORD_POOL (config.rs).
  {
    id: "stinkyscape-018-1",
    faces: ["base"],
    options: ["Taunt", "Rush", "First Strike", "Flying", "Ranged", "Poisonous", "Reborn", "Divine Shield", "Windfury", "Cleave", "Deft", "Lifesteal", "Pierce", "Trample"],
  },
  // crates/engine/src/subsystems/papaya.rs: cells `x,y` (`cell_option`), corners and middle, and PAPAYA_DONE.
  { id: "classicplus-062", faces: ["base", "radiant"], options: ["0,0", "2,1", "4,3", "done"] },
];

describe("#2646 every short choice card has its words", () => {
  it("#2646 every declared Choose one of DISCOVER_OPTION_LIMIT options or fewer has MODE_TEXT's words", () => {
    const missing = Object.entries(DECLARED).flatMap(([id, faces]) =>
      typeof faces === "string" ? [] : (["base", "radiant"] as const).flatMap((face) => wordless(id, face, faces[face] ?? [])),
    );
    expect(missing).toEqual([]);
    // The export is read, and holds the cards it should: Pocket Chaos's three on both faces.
    expect(DECLARED["core-087"]).toMatchObject({ base: [["health", "board", "library"]] });
  });

  it("#2646 every option an engine Choose one offers has MODE_TEXT's words", () => {
    const missing = ENGINE_MODES.flatMap(({ id, faces, options }) => faces.flatMap((face) => options.flatMap((option) => (modeText(id, option, face === "radiant").detail === undefined ? [`${id} ${face} ${option}`] : []))));
    expect(missing).toEqual([]);
  });

  it("#2646 an option nobody wrote words for still has none, so the tests above can fail", () => {
    expect(modeText("core-087", "no-such-option").detail).toBeUndefined();
    expect(modeText(undefined, "health").detail).toBeUndefined();
  });
});
