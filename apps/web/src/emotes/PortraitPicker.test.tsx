// The deck builder's portrait picker (issue §8, R641, R644): the deck's current portrait as the
// collapsed control, a grid of the whole six-portrait roster on open, and each tile's Preview
// panel whose pool of emotes plays exactly as they would in a match — voice line on the voice channel
// plus its speech bubble, emoji synth on the effects channel plus the sticker.
//
// The engine seam is the module singleton: `setAudioEngineForTests` installs the recording fake
// the audio suite's own wiring test uses, so "played on the voice channel" is a `playVoice` call
// and "played on the effects channel" a `playSfx` one — no real AudioContext ever runs.

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { PortraitId } from "@jackioh/shared";
import { EMOJI_EMOTE_IDS, EMOTE_IDS, PORTRAIT_IDS, VOICE_EMOTE_IDS } from "@jackioh/shared";

import { VOICE_PRIORITY } from "../audio/constants.ts";
import { setAudioEngineForTests } from "../audio/engine.ts";
import { fakeAudioEngine, type FakeAudioEngine } from "../audio/test/fakeAudio.ts";
import type { SavedDeck } from "../net/api.ts";
import DeckWorkshop from "../game/deckbuilder/DeckWorkshop.tsx";
import { fixtureCatalog, fixtureCollection, legalDecks } from "../game/deckbuilder/fixtures.ts";
import {
  TEST_PROFILE,
  decksResponse,
  fakeDeckServer,
  manualClock,
  memoryStorage,
  savedDeck,
} from "../game/deckbuilder/testkit.ts";
import { pressBack } from "../test/history.ts";
import { at } from "../test/stubServer.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { EMOJI_LABEL } from "./EmojiArt.tsx";
import { ownedPortraits, portraitCosmeticId, portraitOfCosmetic, PortraitsOwned } from "./ownedPortraits.ts";
import { PORTRAIT_DEFS } from "./portraits.ts";
import { PortraitPicker } from "./PortraitPicker.tsx";

let engine: FakeAudioEngine;

beforeEach(() => {
  engine = fakeAudioEngine();
  setAudioEngineForTests(engine);
});

afterEach(() => {
  setAudioEngineForTests(null);
  cleanup();
});

function picker(portrait: PortraitId = "gary", onPick = vi.fn()): ReactElement {
  return <PortraitPicker portrait={portrait} onPick={onPick} />;
}

/** The collapsed control, opened. */
function openPicker(): HTMLElement {
  fireEvent.click(screen.getByTestId("portrait-current"));
  return screen.getByRole("dialog", { name: "Choose a hero portrait" });
}

describe("the deck's portrait button (R641)", () => {
  it("R641 the collapsed control wears the deck's portrait art as the hero does — its oval", () => {
    render(picker("dfender"));
    const current = screen.getByTestId("portrait-current");
    expect(current).toHaveAttribute("title", `Hero portrait: ${PORTRAIT_DEFS.dfender.def.name}`);
    // CardArt's oval, the same element the hero's portrait uses.
    expect(current.querySelector(".cf-art--oval")).not.toBeNull();
    // Root + button is all the deck editor's row has to fit.
    const root = screen.getByTestId("portrait-picker");
    expect(root).toHaveClass("portrait-picker");
    expect(root.children).toHaveLength(1);
  });

  it("R641 the open menu is the whole six-portrait roster, each tile its card's art and name", () => {
    render(picker("gary"));
    const menu = openPicker();
    const grid = within(menu).getByRole("listbox", { name: "Portraits" });
    const tiles = grid.querySelectorAll<HTMLElement>(".portrait-tile");
    expect(tiles).toHaveLength(PORTRAIT_IDS.length);
    for (const [at, portrait] of PORTRAIT_IDS.entries()) {
      const tile = tiles[at] as HTMLElement;
      expect(tile).toHaveAttribute("data-portrait", portrait);
      const pick = within(tile).getByTestId(`portrait-pick-${portrait}`);
      expect(pick.querySelector(".cf-art--oval")).not.toBeNull();
      // The tile names the card it draws — the roster is looked up by name, never by number.
      const def = PORTRAIT_DEFS[portrait].def;
      expect(pick.title.startsWith(`${def.name} — `)).toBe(true);
      // And its Preview toggle is the other control on the tile.
      expect(within(tile).getByTestId(`portrait-preview-${portrait}`).textContent).toBe("Preview");
    }
  });

  it("R641 the deck's own tile shows selected, and picking another reports its id and closes", () => {
    const onPick = vi.fn();
    render(picker("timmy", onPick));
    openPicker();
    const chosen = screen.getByTestId("portrait-pick-timmy");
    expect(chosen).toHaveAttribute("aria-pressed", "true");
    expect(chosen.closest(".portrait-tile")).toHaveAttribute("data-selected", "true");
    const other = screen.getByTestId("portrait-pick-shredder");
    expect(other).toHaveAttribute("aria-pressed", "false");
    expect(other.closest(".portrait-tile")).not.toHaveAttribute("data-selected");

    fireEvent.click(screen.getByTestId("portrait-pick-felinors"));
    expect(onPick).toHaveBeenCalledWith("felinors");
    expect(onPick).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("the menu closes on Escape and on a press outside the picker", () => {
    render(picker("gary"));
    openPicker();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();

    openPicker();
    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});

describe("the portraits an account owns (R1827000)", () => {
  /** The open menu's tiles, by portrait, in the order they show. */
  function tiles(): (string | null)[] {
    const grid = within(openPicker()).getByRole("listbox", { name: "Portraits" });
    return [...grid.querySelectorAll<HTMLElement>(".portrait-tile")].map((tile) => tile.getAttribute("data-portrait"));
  }

  it("R1827000 the menu offers only the owned portraits, in roster order, and the deck's own", () => {
    render(<PortraitsOwned owned={["vanilla", "gary"]}>{picker("timmy")}</PortraitsOwned>);
    expect(tiles()).toEqual(["vanilla", "gary", "timmy"]);
    // The deck's own pick, owned or not, is the one shown selected.
    expect(screen.getByTestId("portrait-pick-timmy")).toHaveAttribute("aria-pressed", "true");
    expect(screen.queryByTestId("portrait-pick-shredder")).toBeNull();
    cleanup();

    // An owned pick shows once; under "every", as with no provider above it, every portrait is offered.
    render(<PortraitsOwned owned={["vanilla", "gary"]}>{picker("gary")}</PortraitsOwned>);
    expect(tiles()).toEqual(["vanilla", "gary"]);
    cleanup();
    render(<PortraitsOwned owned="every">{picker("gary")}</PortraitsOwned>);
    expect(tiles()).toEqual([...PORTRAIT_IDS]);
  });

  it("R1827000 ownedPortraits reads the starters and the account's rows, and portraitOfCosmetic maps ids", () => {
    // `vanilla` is the registry's starter: every account owns it with no row.
    expect(ownedPortraits([])).toEqual(["vanilla"]);
    expect(ownedPortraits(["portrait-shredder", "table-wood", "portrait-gary", "portrait-ulfric", "gary"])).toEqual([
      "vanilla",
      "gary",
      "shredder",
    ]);
    expect(ownedPortraits(PORTRAIT_IDS.map(portraitCosmeticId))).toEqual([...PORTRAIT_IDS]);

    expect(portraitCosmeticId("timmy")).toBe("portrait-timmy");
    expect(portraitOfCosmetic("portrait-gary")).toBe("gary");
    expect(portraitOfCosmetic("portrait-ulfric")).toBeNull();
    expect(portraitOfCosmetic("table-wood")).toBeNull();
    expect(portraitOfCosmetic("gary")).toBeNull();
    // The registry holds one portrait cosmetic per roster id, and only `vanilla`'s is a starter.
    const registry = COSMETICS.filter((cosmetic) => cosmetic.kind === "portrait");
    expect(registry.map((cosmetic) => portraitOfCosmetic(cosmetic.id))).toEqual([...PORTRAIT_IDS]);
    expect(registry.filter((cosmetic) => cosmetic.starter).map((cosmetic) => cosmetic.id)).toEqual(["portrait-vanilla"]);
  });
});

describe("the picker gives the focus back (#1569)", () => {
  it("Escape from inside the menu closes it and gives the focus back to the portrait button, which names the menu only while it is open", () => {
    render(picker("gary"));
    const current = screen.getByTestId("portrait-current");
    expect(current).not.toHaveAttribute("aria-controls");
    const menu = openPicker();
    expect(menu.id).not.toBe("");
    expect(current).toHaveAttribute("aria-controls", menu.id);

    const tile = screen.getByTestId("portrait-pick-timmy");
    tile.focus();
    fireEvent.keyDown(tile, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(current).toHaveFocus();
    expect(current).not.toHaveAttribute("aria-controls");
  });

  it("a pick closes the menu and gives the focus back to the portrait button", () => {
    const onPick = vi.fn();
    render(picker("gary", onPick));
    openPicker();
    const tile = screen.getByTestId("portrait-pick-felinors");
    tile.focus();
    fireEvent.click(tile);
    expect(onPick).toHaveBeenCalledWith("felinors");
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByTestId("portrait-current")).toHaveFocus();
  });

  it("a press outside, or Escape once the focus is elsewhere on the page, leaves the focus where the player put it", () => {
    render(
      <>
        <button type="button" data-testid="elsewhere">
          elsewhere
        </button>
        {picker("gary")}
      </>,
    );
    const elsewhere = screen.getByTestId("elsewhere");

    openPicker();
    elsewhere.focus();
    fireEvent.pointerDown(elsewhere);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(elsewhere).toHaveFocus();

    openPicker();
    elsewhere.focus();
    fireEvent.keyDown(elsewhere, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(elsewhere).toHaveFocus();
  });

  it("each portrait's Preview names the preview panel while it shows that portrait's pool", () => {
    render(picker("gary"));
    openPicker();
    const garys = screen.getByTestId("portrait-preview-gary");
    expect(garys).not.toHaveAttribute("aria-controls");
    fireEvent.click(garys);
    expect(garys).toHaveAttribute("aria-expanded", "true");
    const panel = screen.getByTestId("portrait-preview-panel");
    expect(panel.id).not.toBe("");
    expect(garys).toHaveAttribute("aria-controls", panel.id);
    expect(screen.getByTestId("portrait-preview-timmy")).not.toHaveAttribute("aria-controls");
  });
});

describe("the emote previews (R644)", () => {
  it("R644 R1343 a tile's Preview opens the whole pool — five voice lines and every emoji a game may deal", () => {
    render(picker("gary"));
    openPicker();
    expect(screen.queryByTestId("portrait-preview-panel")).toBeNull();

    fireEvent.click(screen.getByTestId("portrait-preview-gary"));

    const panel = screen.getByTestId("portrait-preview-panel");
    for (const emote of VOICE_EMOTE_IDS) {
      expect(within(panel).getByTestId(`emote-preview-${emote}`)).toBeInTheDocument();
    }
    for (const emote of EMOJI_EMOTE_IDS) {
      const item = within(panel).getByTestId(`emote-preview-${emote}`);
      expect(item).toHaveAttribute("aria-label", EMOJI_LABEL[emote]);
    }
    // One control for each emote of the pool, whichever eight a game deals (R1340).
    expect(within(panel).getAllByRole("button")).toHaveLength(EMOTE_IDS.length);
  });

  it("R644 a voice preview speaks the previewed portrait's own line and shows its bubble", () => {
    render(picker("gary"));
    openPicker();
    // Preview a portrait that is not the deck's: the previewed one owns the line (R644).
    fireEvent.click(screen.getByTestId("portrait-preview-shredder"));
    fireEvent.click(screen.getByTestId("emote-preview-greetings"));

    // The voice channel, at react priority — the same call a match's emote makes.
    expect(engine.playVoice).toHaveBeenCalledWith(
      "emote-shredder",
      "greetings",
      0,
      VOICE_PRIORITY.react,
    );
    expect(engine.playSfx).not.toHaveBeenCalled();
    // …captioned by the speech bubble even with the sound off (issue §3).
    const bubble = screen.getByTestId("emote-bubble");
    expect(bubble).toHaveClass("emote-bubble");
    expect(bubble.textContent).not.toBe("");
  });

  it("R644 an emoji preview plays its synth on the effects channel and pops the sticker", () => {
    render(picker("gary"));
    openPicker();
    fireEvent.click(screen.getByTestId("portrait-preview-timmy"));
    fireEvent.click(screen.getByTestId("emote-preview-laugh"));

    expect(engine.playSfx).toHaveBeenCalledWith("emoteLaugh");
    expect(engine.playVoice).not.toHaveBeenCalled();
    const sticker = screen.getByTestId("emote-bubble");
    expect(sticker).toHaveClass("emote-sticker");
    expect(sticker).toHaveAttribute("data-emoji", "laugh");
  });

  it("R644 the same preview is live per emote — each press plays again as the newest show", () => {
    render(picker("gary"));
    openPicker();
    fireEvent.click(screen.getByTestId("portrait-preview-gary"));
    fireEvent.click(screen.getByTestId("emote-preview-sob"));
    fireEvent.click(screen.getByTestId("emote-preview-wahWah"));

    expect(engine.playSfx).toHaveBeenNthCalledWith(1, "emoteSob");
    expect(engine.playSfx).toHaveBeenNthCalledWith(2, "emoteWahWah");
    // The newest emote's sticker replaced the first — one show at a time, as in a match.
    expect(screen.getByTestId("emote-bubble")).toHaveAttribute("data-emoji", "wahWah");
  });
});

describe("the picker's place in the deck editor (R641)", () => {
  it("R641 the menu opens inside the picker root and the root is the inline-block it anchors to", () => {
    render(picker("gary"));
    const menu = openPicker();
    const root = screen.getByTestId("portrait-picker");
    // Structural containment: the menu is the root's own child, so it fits wherever the
    // deck editor drops the control, and the CSS pins the bounds it must keep.
    expect(root.contains(menu)).toBe(true);
    expect(menu).toHaveClass("portrait-menu");
    expect(menu.querySelector(".portrait-grid")).not.toBeNull();

    const css = readFileSync(
      join(dirname(fileURLToPath(import.meta.url)), "emotes.css"),
      "utf8",
    );
    expect(css).toMatch(/\.portrait-picker\s*\{[^}]*position:\s*relative/);
    expect(css).toMatch(/\.portrait-menu\s*\{[^}]*position:\s*absolute/);
    expect(css).toMatch(/\.portrait-menu\s*\{[^}]*max-width:/);
  });
});

// ---------------------------------------------------------------------------------------------
// The picker where the player meets it: in the open deck's editor (issue §8, R641, D5)
// ---------------------------------------------------------------------------------------------

const catalog = fixtureCatalog();
const collection = fixtureCollection();
const [ONE = []] = legalDecks();

/** The workshop with `decks` already the server's, opened straight into deck `id`'s editor. */
function mountWorkshop(decks: readonly SavedDeck[], id: string) {
  const server = fakeDeckServer();
  const data = decksResponse([...decks], [], catalog.version);
  server.seed(data);
  const storage = memoryStorage();
  render(
    <DeckWorkshop
      catalog={catalog}
      collection={collection}
      data={data}
      profileId={TEST_PROFILE}
      api={server.api}
      storage={storage}
      clock={manualClock()}
      initialOpen={{ kind: "deck", id }}
    />,
  );
  return { server, storage };
}

describe("the picker in the deck editor (R641)", () => {
  it("R641 the editor's picker wears the deck's saved portrait and a pick saves the new one", () => {
    const { storage } = mountWorkshop(
      [{ ...savedDeck("d1", "Aggro", ONE, 1), portrait: "gary" }],
      "d1",
    );

    // The collapsed control is the deck's portrait: its card's name, its oval art.
    const current = screen.getByTestId("portrait-current");
    expect(current).toHaveAttribute("title", `Hero portrait: ${PORTRAIT_DEFS.gary.def.name}`);
    expect(current.querySelector(".cf-art--oval")).not.toBeNull();

    // A pick writes through the deck store like any edit — `updateDeck(id, { portrait })` —
    // which the local mirror shows at once and the debounced PUT then carries (sync.test.ts).
    fireEvent.click(current);
    fireEvent.click(screen.getByTestId("portrait-pick-shredder"));
    expect(screen.getByTestId("portrait-current")).toHaveAttribute(
      "title",
      `Hero portrait: ${PORTRAIT_DEFS.shredder.def.name}`,
    );

    const mirror = JSON.parse(
      [...storage.data.entries()].find(([key]) => key.startsWith("jackioh.decks.v1."))?.[1] ?? "{}",
    ) as { decks?: { item: { id: string; portrait: string | null }; dirty: boolean }[] };
    const entry = mirror.decks?.find(({ item }) => item.id === "d1");
    expect(entry?.item.portrait).toBe("shredder");
    expect(entry?.dirty).toBe(true);
  });
});

describe("R2127000 Back closes the portrait picker", () => {
  afterEach(() => {
    at("/");
  });

  it("R2127000 one Back closes the open picker, gives the focus back to the portrait button and the page stays on /decks; after Escape, one Back leaves", async () => {
    at("/play");
    window.history.pushState(null, "", "/decks");
    render(picker("gary"));
    openPicker();
    screen.getByTestId("portrait-pick-timmy").focus();

    await pressBack();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByTestId("portrait-current")).toHaveFocus();
    expect(window.location.pathname).toBe("/decks");

    // Escape closes it once and leaves no layer: the next Back is the page's.
    openPicker();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog")).toBeNull();
    await pressBack();
    expect(window.location.pathname).toBe("/play");
  });
});
