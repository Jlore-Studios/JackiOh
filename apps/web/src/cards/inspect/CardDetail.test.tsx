// Polish 6, slice C: the detail view (docs/polish/6-cards.md, B29).
//
// `<CardDetail def onClose actions? meta?>` is a centred dialog: its faces one at a time (issue #37), a meta
// line `#<index> · <set> · <rarity> · <type>` plus ` · <tags>` and ` · N lines of code` (E36), the
// glossary of both faces, then the caller's meta and actions, then inspect-close. Its close paths and
// focus return are B25, in inspect.test.tsx. Real catalog throughout.

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi, type Mock } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { INSPECT_VOICE_DELAY_MS, VOICE_PRIORITY } from "../../audio/constants.ts";
import { getAudioEngine, setAudioEngineForTests } from "../../audio/engine.ts";
import type { AudioEngine } from "../../audio/types.ts";
import { CardDefsProvider } from "../refContext.tsx";
import { locWords } from "../model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../settings.ts";
import { detailMetaLine } from "./CardDetail.tsx";
import { CAROUSEL_SWIPE_PX } from "./constants.ts";
import { CardDetail, closeInspect } from "./index.ts";
import {
  INSPECT_CLOSE,
  INSPECT_CAROUSEL,
  INSPECT_CAROUSEL_NEXT,
  INSPECT_CAROUSEL_POSITION,
  INSPECT_CAROUSEL_PREVIOUS,
  INSPECT_DETAIL,
  INSPECT_FACE_BASE,
  INSPECT_FACE_RADIANT,
  INSPECT_GLOSSARY,
} from "./testids.ts";

/** The meta line's separator, as the Surface spells it. */
const SEP = " · ";
/**
 * #98's pages: its base and Radiant faces, then what each face names on the face it names it — the
 * base text's Rush Token and Felinor Token, the Radiant text's Radiant Rush Token and Ghoul Token.
 */
const HEROIC_POWER_PAGES = ["Base", "Radiant", "Rush Token", "Felinor Token", "Radiant Rush Token", "Ghoul Token"];

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`expected ${id} in the catalog`);
  return def;
}

function metaOf(def: CardDef): string {
  return `#${def.index}${SEP}${def.set}${SEP}${def.rarity}${SEP}${def.type}`;
}

function faceRoot(testId: string): HTMLElement {
  const holder = screen.getByTestId(testId);
  const face = holder.querySelector<HTMLElement>(".cf");
  if (face === null) throw new Error(`expected a CardFace inside ${testId}`);
  return face;
}

function termsInDetail(): (string | null)[] {
  const detail = screen.getByTestId(INSPECT_DETAIL);
  return Array.from(detail.querySelectorAll(`[data-testid="${INSPECT_GLOSSARY}"] li`)).map((li) =>
    li.getAttribute("data-glossary-term"),
  );
}

function precedes(a: Node, b: Node): boolean {
  return (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
}

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
  writeCardSettings(CARD_SETTINGS_DEFAULTS);
  setAudioEngineForTests(null);
  vi.restoreAllMocks();
});

describe("CardDetail (B29)", () => {
  it("B29 renders inspect-detail as a dialog holding the base face, then the radiant face", () => {
    const def = defOf("core-043");
    render(<CardDetail def={def} onClose={() => undefined} />);

    const detail = screen.getByTestId(INSPECT_DETAIL);
    expect(detail).toHaveAttribute("role", "dialog");
    const base = within(detail).getByTestId(INSPECT_FACE_BASE);
    const radiant = within(detail).getByTestId(INSPECT_FACE_RADIANT);
    expect(precedes(base, radiant), "base face first").toBe(true);

    expect(faceRoot(INSPECT_FACE_BASE).querySelector(".card-name")).toHaveTextContent(def.name);
    expect(faceRoot(INSPECT_FACE_RADIANT).querySelector(".card-name")).toHaveTextContent(def.name);
    expect(faceRoot(INSPECT_FACE_BASE)).not.toHaveAttribute("data-radiant-face");
    expect(faceRoot(INSPECT_FACE_RADIANT)).toHaveAttribute("data-radiant-face", "true");
  });

  it("B29 is a portal into document.body, outside the caller's tree", () => {
    const { container } = render(<CardDetail def={defOf("core-043")} onClose={() => undefined} />);
    const detail = screen.getByTestId(INSPECT_DETAIL);
    expect(document.body.contains(detail)).toBe(true);
    expect(container.contains(detail)).toBe(false);
  });

  it("B29 the meta line reads #<index> · <set> · <rarity> · <type> · <tag>", () => {
    const def = defOf("core-043");
    expect(def.tags).toEqual(["Felinor"]);
    render(<CardDetail def={def} onClose={() => undefined} />);
    expect(screen.getByTestId(INSPECT_DETAIL)).toHaveTextContent(`${metaOf(def)}${SEP}Felinor`);
  });

  it("B29 E36 a card with no tags goes from its type straight to its lines of code, which end the line", () => {
    const def = defOf("core-019");
    expect(def.tags, "the fixture card is untagged").toEqual([]);
    expect(def.loc, "the fixture card's script is counted").toBeGreaterThan(0);
    render(<CardDetail def={def} onClose={() => undefined} />);
    const text = screen.getByTestId(INSPECT_DETAIL).textContent ?? "";
    const line = `${metaOf(def)}${SEP}${locWords(def.loc ?? 0)}`;
    expect(text).toContain(line);
    expect(text).not.toContain(`${line}${SEP}`);
  });

  it("B29 a card whose script nobody counted ends its meta line at the type", () => {
    const { loc: _loc, ...uncounted } = defOf("core-019");
    render(<CardDetail def={uncounted} onClose={() => undefined} />);
    const text = screen.getByTestId(INSPECT_DETAIL).textContent ?? "";
    expect(text).toContain(metaOf(uncounted));
    expect(text).not.toContain(`${metaOf(uncounted)}${SEP}`);
    expect(text).not.toContain("lines of code");
  });

  it("E36 the meta line ends with the card's lines of code, every catalog card that has a count", () => {
    for (const def of Object.values(CATALOG)) {
      if (def.loc === undefined) continue;
      expect(detailMetaLine(def), def.id).toMatch(new RegExp(`${SEP}${String(def.loc)} lines? of code$`));
    }
    expect(locWords(1)).toBe("1 line of code");
    expect(locWords(27)).toBe("27 lines of code");
  });

  it("B2.7 a card whose Radiant face has a type of its own says so after the type (Classic+ #22 Blood Moon)", () => {
    const def = defOf("classicplus-022");
    expect(def.type).toBe("Trap");
    expect(def.radiant.type).toBe("Field Trap");
    render(<CardDetail def={def} onClose={() => undefined} />);
    expect(screen.getByTestId(INSPECT_DETAIL).textContent).toContain(`${SEP}Trap (Radiant: Field Trap)`);
    const radiantFace = faceRoot(INSPECT_FACE_RADIANT);
    expect(radiantFace.getAttribute("data-card-type")).toBe("Field Trap");
    expect(radiantFace.querySelector(".card-type")?.textContent).toBe("Field Trap");
    expect(faceRoot(INSPECT_FACE_BASE).getAttribute("data-card-type")).toBe("Trap");
  });

  it("B29 a card with two tags names both after the type", () => {
    // No deckable Core card carries two tags; a token does, and the detail takes any def.
    const def = defOf("core-t-felinor");
    expect(def.tags).toEqual(["Felinor", "Token"]);
    render(<CardDetail def={def} onClose={() => undefined} />);
    const text = screen.getByTestId(INSPECT_DETAIL).textContent ?? "";
    const start = text.indexOf(`${metaOf(def)}${SEP}`);
    expect(start, "the meta line, with its tag separator").toBeGreaterThanOrEqual(0);
    const tail = text.slice(start + metaOf(def).length);
    expect(tail).toContain("Felinor");
    expect(tail).toContain("Token");
  });

  it("B29 the meta line of a Mythic names its rarity", () => {
    const def = defOf("core-100");
    expect(def.rarity).toBe("Mythic");
    render(<CardDetail def={def} onClose={() => undefined} />);
    expect(screen.getByTestId(INSPECT_DETAIL)).toHaveTextContent(metaOf(def));
  });

  it("B29 the glossary covers both faces: a keyword only the radiant face has is listed", () => {
    const def = defOf("core-011");
    expect(def.base.keywords.map((k) => k.kind)).not.toContain("Charge");
    expect(def.radiant.keywords.map((k) => k.kind)).toContain("Charge");
    render(<CardDetail def={def} onClose={() => undefined} />);
    const terms = termsInDetail();
    expect(terms).toContain("Rush");
    expect(terms).toContain("First Strike");
    expect(terms).toContain("Charge");
  });

  it("B29 the glossary covers both faces: a trigger only the radiant text names is listed", () => {
    const def = defOf("core-003");
    expect(def.base.text).not.toContain("Death:");
    expect(def.radiant.text).toContain("Death:");
    render(<CardDetail def={def} onClose={() => undefined} />);
    const terms = termsInDetail();
    for (const term of ["Taunt", "Divine Shield", "Reborn", "Death"]) expect(terms).toContain(term);
  });

  it("B29 a card whose faces name no term and carry no keyword renders no glossary", () => {
    // Every Core card now names a term or carries a keyword on one face (the Felinor Token's radiant
    // face has Rush, R276), so the case is a made-up def with nothing on either face.
    const def: CardDef = {
      ...defOf("core-t-felinor"),
      id: "x-blank",
      base: { attack: 1, health: 1, keywords: [], text: "" },
      radiant: { attack: 2, health: 2, keywords: [], text: "" },
    };
    render(<CardDetail def={def} onClose={() => undefined} />);
    expect(within(screen.getByTestId(INSPECT_DETAIL)).queryByTestId(INSPECT_GLOSSARY)).toBeNull();
  });

  it("B29 renders the caller's meta and actions inside the dialog, before inspect-close", () => {
    render(
      <CardDetail
        def={defOf("core-043")}
        onClose={() => undefined}
        meta={<span data-testid="caller-meta">In Deck 2</span>}
        actions={
          <button type="button" data-testid="caller-action">
            Add
          </button>
        }
      />,
    );
    const detail = screen.getByTestId(INSPECT_DETAIL);
    const meta = within(detail).getByTestId("caller-meta");
    const action = within(detail).getByTestId("caller-action");
    const close = within(detail).getByTestId(INSPECT_CLOSE);
    expect(meta).toHaveTextContent("In Deck 2");
    expect(precedes(meta, close), "meta before inspect-close").toBe(true);
    expect(precedes(action, close), "actions before inspect-close").toBe(true);
  });

  it("B29 without actions or meta it still holds both faces and inspect-close", () => {
    render(<CardDetail def={defOf("core-084")} onClose={() => undefined} />);
    const detail = screen.getByTestId(INSPECT_DETAIL);
    expect(within(detail).getByTestId(INSPECT_FACE_BASE)).toBeInTheDocument();
    expect(within(detail).getByTestId(INSPECT_FACE_RADIANT)).toBeInTheDocument();
    expect(within(detail).getByTestId(INSPECT_CLOSE)).toBeInTheDocument();
  });

  it("B29 a radiant face whose text is the base text is still the radiant face, and marks only its stats", () => {
    const def = defOf("core-012");
    expect(def.radiant.text).toBe(def.base.text);
    render(<CardDetail def={def} onClose={() => undefined} />);
    expect(faceRoot(INSPECT_FACE_RADIANT)).toHaveAttribute("data-radiant-face", "true");
    expect(faceRoot(INSPECT_FACE_RADIANT).querySelector(".cf-mark")).toBeNull();
    expect(faceRoot(INSPECT_FACE_RADIANT).querySelector('.cf-atk[data-grew="true"]')).not.toBeNull();
  });

  it("R277 the base face prints its text; the radiant face prints its own whole text with the new words marked", () => {
    const def = defOf("core-043");
    expect(def.radiant.text).not.toBe(def.base.text);
    render(<CardDetail def={def} onClose={() => undefined} />);
    expect(faceRoot(INSPECT_FACE_BASE).querySelector(".cf-text-base")).toHaveTextContent(def.base.text);
    expect(faceRoot(INSPECT_FACE_BASE).querySelector(".cf-mark")).toBeNull();
    expect(faceRoot(INSPECT_FACE_RADIANT).querySelector(".cf-text-base")).toHaveTextContent(def.radiant.text);
    expect([...faceRoot(INSPECT_FACE_RADIANT).querySelectorAll(".cf-mark")].map((mark) => mark.textContent)).toEqual([
      "enemy",
    ]);
    // The reading-size Radiant line marks the same words.
    const line = screen.getByTestId(INSPECT_DETAIL).querySelector(".inspect-rules-line--radiant");
    expect([...(line?.querySelectorAll(".cf-mark") ?? [])].map((mark) => mark.textContent)).toEqual(["enemy"]);
  });

  it("B29 every catalog card opens a detail with its name on both faces and its #index in the meta", () => {
    for (const def of Object.values(CATALOG)) {
      const { unmount } = render(<CardDetail def={def} onClose={() => undefined} />);
      const detail = screen.getByTestId(INSPECT_DETAIL);
      expect(faceRoot(INSPECT_FACE_BASE).querySelector(".card-name"), def.id).toHaveTextContent(def.name);
      expect(faceRoot(INSPECT_FACE_RADIANT).querySelector(".card-name"), def.id).toHaveTextContent(def.name);
      expect(detail.textContent ?? "", def.id).toContain(metaOf(def));
      unmount();
    }
    // 317 catalog entries since v0.2.0 (the default 5 s was sized for Core's 111).
  }, 30_000);

  it("pages a card's base, Radiant, then each named card on the face its text names, one at a time", () => {
    const def = defOf("core-098");
    render(
      <CardDefsProvider defs={CATALOG}>
        <CardDetail def={def} onClose={() => undefined} />
      </CardDefsProvider>,
    );
    const shown = (): HTMLElement[] =>
      [...screen.getByTestId(INSPECT_CAROUSEL).querySelectorAll<HTMLElement>("figure")].filter((figure) => !figure.hidden);
    const caption = (): string => shown()[0]?.querySelector("figcaption")?.textContent ?? "";
    const count = HEROIC_POWER_PAGES.length;

    expect(screen.getByTestId(INSPECT_CAROUSEL).querySelectorAll("figure")).toHaveLength(count);
    expect(shown()).toHaveLength(1);
    expect(screen.getByTestId(INSPECT_CAROUSEL_POSITION)).toHaveTextContent(`1 of ${String(count)}`);
    expect(screen.getByTestId(INSPECT_FACE_BASE).closest("figure")).not.toHaveAttribute("hidden");
    expect(screen.getByTestId(INSPECT_FACE_RADIANT).closest("figure")).toHaveAttribute("hidden");

    const seen: string[] = [];
    for (let n = 0; n < count; n += 1) {
      seen.push(caption());
      fireEvent.click(screen.getByTestId(INSPECT_CAROUSEL_NEXT));
    }
    expect(seen).toEqual(HEROIC_POWER_PAGES);
    // It wraps: past the last page is the first again.
    expect(caption()).toBe("Base");
    fireEvent.click(screen.getByTestId(INSPECT_CAROUSEL_PREVIOUS));
    expect(caption()).toBe("Ghoul Token");
    // The Radiant Rush Token page is the token's Radiant face.
    fireEvent.click(screen.getByTestId(INSPECT_CAROUSEL_PREVIOUS));
    expect(shown()[0]).toHaveAttribute("data-related-id", "core-t-rush");
    expect(shown()[0]?.querySelector(".cf")).toHaveAttribute("data-radiant-face", "true");
  });

  it("the arrow keys and a horizontal swipe page it too; a mouse drag does not", () => {
    render(
      <CardDefsProvider defs={CATALOG}>
        <CardDetail def={defOf("core-098")} onClose={() => undefined} />
      </CardDefsProvider>,
    );
    const carousel = screen.getByTestId(INSPECT_CAROUSEL);
    const position = (): string => screen.getByTestId(INSPECT_CAROUSEL_POSITION).textContent ?? "";
    const faces = carousel.querySelector<HTMLElement>(".inspect-detail-faces");
    if (faces === null) throw new Error("no faces");
    const swipe = (pointerType: string, from: number, to: number): void => {
      fireEvent.pointerDown(faces, { pointerType, clientX: from, pointerId: 3 });
      fireEvent.pointerUp(faces, { pointerType, clientX: to, pointerId: 3 });
    };

    fireEvent.keyDown(carousel, { key: "ArrowRight" });
    expect(position()).toMatch(/^2 of /);
    fireEvent.keyDown(carousel, { key: "ArrowLeft" });
    expect(position()).toMatch(/^1 of /);

    swipe("touch", 300, 300 - CAROUSEL_SWIPE_PX);
    expect(position()).toMatch(/^2 of /);
    swipe("touch", 100, 100 + CAROUSEL_SWIPE_PX);
    expect(position()).toMatch(/^1 of /);
    // Shorter than a swipe, or a mouse, pages nothing.
    swipe("touch", 300, 300 - CAROUSEL_SWIPE_PX + 1);
    swipe("mouse", 300, 100);
    expect(position()).toMatch(/^1 of /);
  });

  it("offers every voice-line kind that the inspected card has and plays the selected line", () => {
    const playVoice = vi.spyOn(getAudioEngine(), "playVoice");
    const unlock = vi.spyOn(getAudioEngine(), "unlock");
    const unit = defOf("core-004");
    const { rerender } = render(<CardDetail def={unit} onClose={() => undefined} />);

    expect(screen.getByRole("button", { name: "Play voice line" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Death voice line" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Cast voice line" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Death voice line" }));
    expect(unlock).toHaveBeenCalledOnce();
    expect(playVoice).toHaveBeenCalledWith(unit.id, "death", INSPECT_VOICE_DELAY_MS, VOICE_PRIORITY.summon);

    rerender(<CardDetail def={defOf("core-005")} onClose={() => undefined} />);
    expect(screen.queryByRole("button", { name: "Play voice line" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Death voice line" })).toBeNull();
    expect(screen.getByRole("button", { name: "Cast voice line" })).toBeInTheDocument();
  });
});

describe("VoicePreview (R630)", () => {
  /** A stand-in for the audio singleton: records every call, plays nothing. */
  function fakeEngine(playVoice: Mock<AudioEngine["playVoice"]>): AudioEngine {
    return {
      state: vi.fn<AudioEngine["state"]>(() => "running"),
      unlock: vi.fn<AudioEngine["unlock"]>(),
      preloadVoices: vi.fn<AudioEngine["preloadVoices"]>(),
      setBusy: vi.fn<AudioEngine["setBusy"]>(),
      log: vi.fn<AudioEngine["log"]>(() => []),
      clearLog: vi.fn<AudioEngine["clearLog"]>(),
      contextsCreated: vi.fn<AudioEngine["contextsCreated"]>(() => 0),
      speaking: vi.fn<AudioEngine["speaking"]>(() => false),
      subscribeSpeaking: vi.fn<AudioEngine["subscribeSpeaking"]>(() => () => undefined),
      musicOutput: vi.fn<AudioEngine["musicOutput"]>(() => null),
      subscribeState: vi.fn<AudioEngine["subscribeState"]>(() => () => undefined),
      dispose: vi.fn<AudioEngine["dispose"]>(),
      playSfx: vi.fn<AudioEngine["playSfx"]>(() => true),
      playVoice,
    };
  }

  function optionsOf(select: HTMLElement): string[] {
    return within(select as HTMLSelectElement)
      .getAllByRole("option")
      .map((option) => (option as HTMLOptionElement).value);
  }

  it("R630 a unit offers its play and death lines, and choosing one plays it at the summon priority", () => {
    const playVoice = vi.fn<AudioEngine["playVoice"]>(() => true);
    setAudioEngineForTests(fakeEngine(playVoice));
    render(<CardDetail def={defOf("core-008")} onClose={() => undefined} />);

    const select = screen.getByTestId("voice-preview");
    expect(select.tagName).toBe("SELECT");
    expect(optionsOf(select)).toEqual(["", "play", "death"]);

    fireEvent.change(select, { target: { value: "death" } });
    expect(playVoice).toHaveBeenCalledWith("core-008", "death", 0, VOICE_PRIORITY.summon);
    expect((select as HTMLSelectElement).value, "the dropdown resets to its placeholder").toBe("");
  });

  it("R630 a spell offers only its cast line", () => {
    const playVoice = vi.fn<AudioEngine["playVoice"]>(() => true);
    setAudioEngineForTests(fakeEngine(playVoice));
    render(<CardDetail def={defOf("core-023")} onClose={() => undefined} />);

    const select = screen.getByTestId("voice-preview");
    expect(optionsOf(select)).toEqual(["", "cast"]);

    fireEvent.change(select, { target: { value: "cast" } });
    expect(playVoice).toHaveBeenCalledWith("core-023", "cast", 0, VOICE_PRIORITY.summon);
  });

  it("R630 the dropdown sits in the pinned actions row before Close, and a card with no lines has none", () => {
    const playVoice = vi.fn<AudioEngine["playVoice"]>(() => true);
    setAudioEngineForTests(fakeEngine(playVoice));
    render(
      <CardDetail
        def={defOf("core-008")}
        onClose={() => undefined}
        actions={
          <button type="button" data-testid="caller-action">
            Add
          </button>
        }
      />,
    );
    const detail = screen.getByTestId(INSPECT_DETAIL);
    const voice = within(detail).getByTestId("voice-preview");
    const action = within(detail).getByTestId("caller-action");
    const close = within(detail).getByTestId(INSPECT_CLOSE);
    expect(precedes(action, voice), "the caller's actions come first").toBe(true);
    expect(precedes(voice, close), "the dropdown comes before inspect-close").toBe(true);
    expect(playVoice).not.toHaveBeenCalled();

    // A card the voice table never heard of renders no dropdown at all.
    cleanup();
    const lineless: CardDef = { ...defOf("core-008"), id: "x-no-voice" };
    render(<CardDetail def={lineless} onClose={() => undefined} />);
    expect(within(screen.getByTestId(INSPECT_DETAIL)).queryByTestId("voice-preview")).toBeNull();
  });
});
