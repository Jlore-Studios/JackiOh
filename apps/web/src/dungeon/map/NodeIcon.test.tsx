// #3229: the dungeon map draws each node kind as an SVG in `currentColor`, not as a system emoji,
// and each node's button keeps the accessible name that says its kind and its testid.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { RunNode, RunNodeKind } from "@jackioh/run";

import { CONTENT, runState } from "../test/fixtures.ts";
import { MapScreen } from "./MapScreen.tsx";
import { NodeIcon } from "./NodeIcon.tsx";
import { dungeonMapTestid } from "./testids.ts";

afterEach(cleanup);

/** Every kind, held as a record's keys so a new kind in the run's enum breaks the compile here. */
const KIND_SET: Record<RunNodeKind, true> = {
  fight: true,
  elite: true,
  ghost: true,
  miniBoss: true,
  boss: true,
  shop: true,
  campfire: true,
  event: true,
  chest: true,
  shrine: true,
};
const KINDS = Object.keys(KIND_SET) as RunNodeKind[];

function node(kind: RunNodeKind): RunNode {
  return { kind, enemy: null, event: null, artifact: null, column: 0, next: [] };
}

describe("#3229 the node icons are drawn", () => {
  it("#3229 every node kind draws its own SVG in currentColor on the 24 grid", () => {
    const drawn = new Set<string>();
    for (const kind of KINDS) {
      const { container } = render(<NodeIcon kind={kind} />);
      const svg = container.querySelector("svg");
      expect(svg, kind).not.toBeNull();
      if (svg === null) continue;
      expect(svg.getAttribute("data-icon"), kind).toBe(kind);
      expect(svg.getAttribute("viewBox"), kind).toBe("0 0 24 24");
      expect(svg.getAttribute("aria-hidden"), kind).toBe("true");
      expect(svg.getAttribute("stroke"), kind).toBe("currentColor");
      expect(svg.querySelector("path, circle"), kind).not.toBeNull();
      expect(svg.querySelector("text, title"), kind).toBeNull();
      expect(svg.outerHTML, kind).not.toMatch(/#[0-9a-f]{3,8}\b|rgb|hsl/i);
      drawn.add(svg.innerHTML);
      cleanup();
    }
    expect(drawn.size).toBe(KINDS.length);
  });

  it("#3229 the map draws each kind's icon in a button named for the kind and keeps its testid", () => {
    const run = runState({
      map: [
        [node("fight"), node("elite"), node("ghost")],
        [node("shop"), node("campfire"), node("event"), node("chest"), node("shrine")],
        [node("miniBoss"), node("boss")],
      ],
      row: null,
    });
    render(<MapScreen run={run} content={CONTENT} send={vi.fn()} />);

    const nodes: [number, number, RunNodeKind, string][] = [
      [0, 0, "fight", "Fight"],
      [0, 1, "elite", "Elite"],
      [0, 2, "ghost", "Ghost"],
      [1, 0, "shop", "Shop"],
      [1, 1, "campfire", "Campfire"],
      [1, 2, "event", "Event"],
      [1, 3, "chest", "Chest"],
      [1, 4, "shrine", "Shrine"],
      [2, 0, "miniBoss", "Mini-boss"],
      [2, 1, "boss", "Final boss"],
    ];
    expect(new Set(nodes.map(([, , kind]) => kind))).toEqual(new Set(KINDS));
    for (const [row, col, kind, name] of nodes) {
      const button = screen.getByTestId(dungeonMapTestid.mapNode(row, col));
      expect(button).toHaveAccessibleName(`${name} ${String(col + 1)}`);
      expect(button.getAttribute("data-kind")).toBe(kind);
      const icons = button.querySelectorAll(`.dungeon-run-map__icon[aria-hidden="true"] svg[data-icon="${kind}"]`);
      expect(icons, kind).toHaveLength(1);
      expect(button.querySelectorAll("svg"), kind).toHaveLength(1);
      // The old emoji were text: the label, relic and enemy lines are plain ASCII.
      expect(button.textContent, kind).toMatch(/^[\x20-\x7E]*$/);
    }
  });
});
