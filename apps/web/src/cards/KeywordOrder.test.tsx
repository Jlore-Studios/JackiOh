// R4747000 (§6.1): every card's keyword line prints in one fixed order, and the face model sorts
// live keywords into it. These tests hold the web to the face's order: `KeywordIcons` and
// `FieldBadges` render a sorted face's words even when the view's `unit.keywords` arrive shuffled,
// with each icon's accessible name still its own keyword's words.

import type { Keyword } from "@jackioh/shared";
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { card, unit } from "../test/fixtures.ts";
import FieldBadges from "../game/FieldBadges.tsx";
import { KeywordIcons } from "./MinionFace.tsx";

/** The face's sorted order for a unit that arrived as Ranged, Flying, Can't Defend. */
const faceKeywords: Keyword[] = [{ kind: "Can't Defend" }, { kind: "Flying" }, { kind: "Ranged" }];
const faceWords = ["Can't Defend", "Flying", "Ranged"];

/** The same keywords as the view lists them: shuffled arrival order. */
const unitKeywords: Keyword[] = [{ kind: "Ranged" }, { kind: "Flying" }, { kind: "Can't Defend" }];

const kindsOf = (root: ParentNode): string[] =>
  [...root.querySelectorAll(".keyword-icon")].map((mark) => mark.getAttribute("data-keyword") ?? "");
const namesOf = (root: ParentNode): (string | null)[] =>
  [...root.querySelectorAll(".keyword-icon")].map((mark) => mark.getAttribute("aria-label"));

afterEach(() => {
  cleanup();
});

describe("R4747000 the keyword marks follow the face's fixed order", () => {
  it("R4747000 KeywordIcons renders the face's order with each icon named by its own words", () => {
    render(<KeywordIcons keywords={faceKeywords} words={faceWords} join=", " armor={0} />);
    expect(kindsOf(document)).toEqual(["Can't Defend", "Flying", "Ranged"]);
    expect(namesOf(document)).toEqual(["Can't Defend", "Flying", "Ranged"]);
  });

  it("R4747000 FieldBadges renders the face's order over shuffled unit.keywords, still paired", () => {
    render(
      <FieldBadges
        card={card()}
        unit={unit("p1", { keywords: unitKeywords })}
        keywords={faceKeywords}
        words={faceWords}
        join=", "
      />,
    );
    expect(kindsOf(document)).toEqual(["Can't Defend", "Flying", "Ranged"]);
    expect(namesOf(document)).toEqual(["Can't Defend", "Flying", "Ranged"]);
  });
});
