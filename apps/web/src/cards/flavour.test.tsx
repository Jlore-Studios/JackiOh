// R658: a card's flavour line and artist credit on the client. The sidecar's shape and keys are
// packages/cards/test/flavour.test.ts's; here its words are held to the voice lines' rule (issue
// #115: flavour never restates rules words, and none of these lines may), and the inspect views are
// held to where the words show: the hover preview's column, the touch sheet and the detail view, and
// never the face itself, a card with no catalog def, or a card no entry names.

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { BANNED_RULES_WORDS } from "../audio/constants.ts";
import { CardFace } from "./CardFace.tsx";
import { CARD_FLAVOUR, flavourFor } from "./flavour.ts";
import { Flavour } from "./inspect/Flavour.tsx";
import { HoverPreview } from "./inspect/HoverPreview.tsx";
import { InspectSheet } from "./inspect/InspectSheet.tsx";
import { CardDetail, closeInspect } from "./inspect/index.ts";
import { INSPECT_ARTIST, INSPECT_DETAIL, INSPECT_FLAVOUR, INSPECT_HOVER, INSPECT_SHEET } from "./inspect/testids.ts";
import { faceModel } from "./model.ts";

const ANCHOR = { left: 100, top: 100, right: 160, bottom: 190, width: 60, height: 90 };

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`expected ${id} in the catalog`);
  return def;
}

function lineOf(id: string): string {
  const line = CARD_FLAVOUR[id]?.flavour;
  if (line === undefined) throw new Error(`expected a flavour line for ${id}`);
  return line;
}

/** Issue #115's matcher, as voice-lines.test.ts writes it: whole word, any case, across whitespace. */
function bannedMatcher(word: string): RegExp {
  const body = word
    .trim()
    .split(/\s+/)
    .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
    .join("\\s+");
  return new RegExp(`(?<![A-Za-z])${body}(?![A-Za-z])`, "i");
}

afterEach(() => {
  closeInspect();
  cleanup();
});

describe("R658 the flavour lines' words", () => {
  it("R658 no flavour line speaks a rules word (the voice lines' list, issue #115)", () => {
    const offending: string[] = [];
    for (const [id, entry] of Object.entries(CARD_FLAVOUR)) {
      const line = entry.flavour ?? "";
      for (const word of BANNED_RULES_WORDS) {
        if (bannedMatcher(word).test(line)) offending.push(`${id} says "${word}": ${line}`);
      }
    }
    expect(offending).toEqual([]);
  });

  it("R658 the matcher catches a rules word in any case and leaves a longer word alone", () => {
    expect(bannedMatcher("damage").test("All the DAMAGE.")).toBe(true);
    expect(bannedMatcher("Divine Shield").test("a divine  shield")).toBe(true);
    expect(bannedMatcher("Counter").test("Countered again")).toBe(false);
  });
});

describe("R658 flavourFor", () => {
  it("R658 is a card's entry, and null for an id with none", () => {
    expect(flavourFor("core-001")?.flavour).toBe(lineOf("core-001"));
    expect(flavourFor("not-a-card")).toBeNull();
    expect(flavourFor("toString")).toBeNull();
    expect(flavourFor("core-001", { "core-001": {} })).toBeNull();
    expect(flavourFor("core-001", { "core-001": { artist: "A. Painter" } })).toEqual({ artist: "A. Painter" });
  });
});

describe("R658 where the flavour shows", () => {
  it("R658 the detail view shows the card's flavour line, and no artist line while the sidecar names none", () => {
    render(<CardDetail def={defOf("core-008")} onClose={() => {}} />);
    const flavour = within(screen.getByTestId(INSPECT_DETAIL)).getByTestId(INSPECT_FLAVOUR);
    expect(flavour).toHaveTextContent(lineOf("core-008"));
    expect(within(flavour).queryByTestId(INSPECT_ARTIST)).toBeNull();
  });

  it("R658 the touch sheet shows it beside the face", () => {
    render(<InspectSheet face={faceModel({ defId: "core-002", def: defOf("core-002"), radiant: true })} onClose={() => {}} />);
    expect(within(screen.getByTestId(INSPECT_SHEET)).getByTestId(INSPECT_FLAVOUR)).toHaveTextContent(lineOf("core-002"));
  });

  it("R658 the hover preview draws its column for the flavour line alone, as for a card with no text", () => {
    render(<HoverPreview face={faceModel({ defId: "core-008", def: defOf("core-008"), radiant: false })} anchor={ANCHOR} />);
    const hover = screen.getByTestId(INSPECT_HOVER);
    expect(hover.querySelector(".inspect-side")).not.toBeNull();
    expect(within(hover).getByTestId(INSPECT_FLAVOUR)).toHaveTextContent(lineOf("core-008"));
  });

  it("R658 a face with no catalog def has no flavour line, and its preview stays alone", () => {
    render(<HoverPreview face={faceModel({ defId: "core-008", radiant: false })} anchor={ANCHOR} />);
    const hover = screen.getByTestId(INSPECT_HOVER);
    expect(within(hover).queryByTestId(INSPECT_FLAVOUR)).toBeNull();
  });

  it("R658 an artist the sidecar names is credited under the line, and an entry with neither draws nothing", () => {
    const sidecar = { "core-001": { flavour: "Stands still.", artist: "A. Painter" }, "core-002": {} };
    render(<Flavour defId="core-001" sidecar={sidecar} />);
    expect(screen.getByTestId(INSPECT_ARTIST)).toHaveTextContent("Art by A. Painter");
    expect(screen.getByTestId(INSPECT_FLAVOUR)).toHaveTextContent("Stands still.");
    cleanup();
    const { container } = render(<Flavour defId="core-002" sidecar={sidecar} />);
    expect(container.firstChild).toBeNull();
  });

  it("R658 the face itself never prints it", () => {
    const { container } = render(<CardFace face={faceModel({ defId: "core-001", def: defOf("core-001"), radiant: false })} layout="full" />);
    expect(container.textContent).not.toContain(lineOf("core-001"));
    expect(container.querySelector(`[data-testid="${INSPECT_FLAVOUR}"]`)).toBeNull();
  });
});
