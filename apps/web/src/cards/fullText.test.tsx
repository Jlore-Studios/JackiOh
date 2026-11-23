// R3838001: below an `AlwaysFullText` (the Almanac) the hover preview carries every card's whole text, not only a face
// that prints less. R3838002: a reference's tooltip carries its card's whole text beside a face that prints less (a
// full-art card), or beside any face below an `AlwaysFullText`, and is sized for it (`refTooltipSize`).

import { act, cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLITCH_DEF_ID } from "@jackioh/engine/config";
import type { CardDef } from "@jackioh/shared";

import { REF_TOOLTIP_TESTID } from "./CardRef.tsx";
import { faceText, printsText } from "./CardFace.tsx";
import { FACE_ASPECT, REF_TOOLTIP_HEIGHT_PX } from "./constants.ts";
import { isFullArtCard, wearsFullArt } from "./fullArt.ts";
import { CardDetail } from "./inspect/CardDetail.tsx";
import {
  PREVIEW_EXTENDED_STACKED_MAX_PX,
  PREVIEW_EXTENDED_WIDTH_PX,
  PREVIEW_GLOSSARY_GAP_PX,
  PREVIEW_MARGIN_PX,
  PREVIEW_PHONE_WIDTH_PX,
} from "./inspect/constants.ts";
import { AlwaysFullText, showsFullText } from "./inspect/Extended.tsx";
import { HoverPreview } from "./inspect/HoverPreview.tsx";
import { refTooltipSize } from "./inspect/placement.ts";
import { closeInspect } from "./inspect/store.ts";
import { INSPECT_DETAIL, INSPECT_EXTENDED } from "./inspect/testids.ts";
import { faceModel, type FaceModel } from "./model.ts";
import { CardDefsProvider } from "./refContext.tsx";

const ANCHOR = { left: 600, top: 200, width: 100, height: 140 };

/** Every card but Glitch, whose words are corrupted wherever a reference reads them. */
const DEFS: readonly CardDef[] = Object.values(CATALOG).filter((def) => def.id !== GLITCH_DEF_ID);

function base(def: CardDef): FaceModel {
  return faceModel({ defId: def.id, def, radiant: false });
}

/** A card with no full art whose face prints its whole text: no preview carried its text before R3838001. */
const PLAIN = DEFS.find((def) => !def.token && !wearsFullArt(def.id) && printsText(base(def)) && !faceText(base(def), undefined, false).less);

/** The first card whose text names another, `named` choosing which: the detail view's related cards (R279). */
function naming(named: (id: string) => boolean): { from: CardDef; to: string } | undefined {
  for (const def of DEFS) {
    const to = base(def).mentions.find((mention) => !mention.radiant && mention.defId !== def.id && named(mention.defId));
    if (to !== undefined) return { from: def, to: to.defId };
  }
  return undefined;
}

/** A reference's tooltip in `from`'s detail view, opened from its rules text at reading size. */
function openRef(from: CardDef, to: string, always: boolean): HTMLElement {
  const detail = <CardDetail def={from} onClose={() => undefined} />;
  render(<CardDefsProvider defs={CATALOG}>{always ? <AlwaysFullText>{detail}</AlwaysFullText> : detail}</CardDefsProvider>);
  const ref = screen.getByTestId(INSPECT_DETAIL).querySelector<HTMLElement>(`.inspect-rules .cf-ref[data-ref="${to}"]`);
  if (ref === null) throw new Error(`${from.id}'s rules name no ${to}`);
  act(() => ref.focus());
  return screen.getByTestId(REF_TOOLTIP_TESTID);
}

afterEach(() => {
  act(() => closeInspect());
  cleanup();
});

describe("R3838001 every card's whole text below an AlwaysFullText", () => {
  it("R3838001 the hover preview carries the Full text of a face that prints it whole only below an AlwaysFullText", () => {
    expect(PLAIN).toBeDefined();
    const face = base(PLAIN!);
    expect(showsFullText(face, undefined, false, false)).toBe(false);
    expect(showsFullText(face, undefined, false, true)).toBe(true);
    render(<HoverPreview face={face} anchor={ANCHOR} />);
    expect(screen.queryByTestId(INSPECT_EXTENDED)).toBeNull();
    cleanup();
    render(
      <AlwaysFullText>
        <HoverPreview face={face} anchor={ANCHOR} />
      </AlwaysFullText>,
    );
    expect(within(screen.getByTestId(INSPECT_EXTENDED)).getByText("Full text")).toBeInTheDocument();
  });

  it("R3838001 a face with no words has no Full text, even below an AlwaysFullText", () => {
    const vanilla = DEFS.find((def) => !printsText(base(def)));
    expect(vanilla).toBeDefined();
    expect(showsFullText(base(vanilla!), undefined, false, true)).toBe(false);
  });
});

describe("R3838002 a reference's tooltip carries its card's whole text", () => {
  it("R3838002 a full-art card's tooltip has its whole text beside its face, with no AlwaysFullText", () => {
    const pair = naming((id) => isFullArtCard(id) && printsText(base(CATALOG[id]!)));
    expect(pair).toBeDefined();
    const tooltip = openRef(pair!.from, pair!.to, false);
    expect(tooltip).toHaveAttribute("data-ref", pair!.to);
    expect(tooltip.querySelector('.cf[data-full-art="true"]')).not.toBeNull();
    expect(within(tooltip).getByTestId(INSPECT_EXTENDED)).toBeInTheDocument();
  });

  it("R3838002 a face that prints its whole text has the panel only below an AlwaysFullText", () => {
    const pair = naming((id) => !wearsFullArt(id) && printsText(base(CATALOG[id]!)) && !faceText(base(CATALOG[id]!), undefined, false).less);
    expect(pair).toBeDefined();
    expect(within(openRef(pair!.from, pair!.to, false)).queryByTestId(INSPECT_EXTENDED)).toBeNull();
    act(() => closeInspect());
    cleanup();
    expect(within(openRef(pair!.from, pair!.to, true)).getByTestId(INSPECT_EXTENDED)).toBeInTheDocument();
  });

  it("R3838002 the tooltip stands the text on the face's left, and on a phone stacks it above a face that shrinks to fit", () => {
    const desk = { width: 1280, height: 800 };
    expect(refTooltipSize(desk, false)).toEqual({
      width: REF_TOOLTIP_HEIGHT_PX * FACE_ASPECT,
      height: REF_TOOLTIP_HEIGHT_PX,
      faceHeight: REF_TOOLTIP_HEIGHT_PX,
      stacked: false,
    });
    expect(refTooltipSize(desk, true)).toEqual({
      width: REF_TOOLTIP_HEIGHT_PX * FACE_ASPECT + PREVIEW_GLOSSARY_GAP_PX + PREVIEW_EXTENDED_WIDTH_PX,
      height: REF_TOOLTIP_HEIGHT_PX,
      faceHeight: REF_TOOLTIP_HEIGHT_PX,
      stacked: false,
    });
    const phone = { width: PREVIEW_PHONE_WIDTH_PX - 210, height: 480 };
    const stacked = refTooltipSize(phone, true);
    const face = phone.height - 2 * PREVIEW_MARGIN_PX - PREVIEW_GLOSSARY_GAP_PX - PREVIEW_EXTENDED_STACKED_MAX_PX;
    expect(face).toBeLessThan(REF_TOOLTIP_HEIGHT_PX);
    expect(stacked).toEqual({
      width: face * FACE_ASPECT,
      height: face + PREVIEW_GLOSSARY_GAP_PX + PREVIEW_EXTENDED_STACKED_MAX_PX,
      faceHeight: face,
      stacked: true,
    });
    expect(stacked.height).toBeLessThanOrEqual(phone.height - 2 * PREVIEW_MARGIN_PX);
    // Without the text a phone's tooltip is the face alone, as before.
    expect(refTooltipSize(phone, false).stacked).toBe(false);
  });
});
