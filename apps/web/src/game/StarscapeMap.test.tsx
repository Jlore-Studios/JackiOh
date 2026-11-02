// R1847000: each seat's starscape map (Stinkyscape #148 Alteran Reach) on the board, drawn from the
// view alone (CLAUDE.md rule 7): nothing before its player first travels, then every node, the one
// they stand on lit and the edge last.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { StarscapeNodeView, StarscapeView } from "@jackioh/shared";

import Board from "./Board.tsx";
import { fullBoardView } from "../test/fixtures.ts";

afterEach(cleanup);

function map(at: number): StarscapeView {
  const heal: StarscapeNodeView = { reward: "heal", amount: 3 };
  const nodes: StarscapeNodeView[] = [
    { reward: "draw", amount: 1 },
    { reward: "mana", amount: 1 },
    { reward: "damage", amount: 2 },
    ...Array.from({ length: 16 }, () => heal),
    { reward: "edge" },
  ];
  return { nodes, at };
}

describe("StarscapeMap", () => {
  it("R1847000 each seat's starscape map renders from the view alone, nothing before it is charted", () => {
    const plain = fullBoardView();
    render(<Board view={plain} />);
    expect(document.querySelectorAll(".starscape")).toHaveLength(0);
    cleanup();

    render(
      <Board
        view={{
          ...plain,
          you: { ...plain.you, starscape: map(3) },
          opponent: { ...plain.opponent, starscape: map(0) },
        }}
      />,
    );
    const own = screen.getByTestId("starscape-you");
    const nodes = [...own.querySelectorAll(".starscape-node")];
    expect(nodes).toHaveLength(20);
    expect(own.getAttribute("data-at")).toBe("3");
    expect(own.getAttribute("title")).toBe("Starscape map: node 3 of 20");
    expect(nodes[2]?.getAttribute("data-here")).toBe("true");
    expect(nodes.filter((node) => node.getAttribute("data-here") === "true")).toHaveLength(1);
    expect(nodes.filter((node) => node.getAttribute("data-visited") === "true")).toHaveLength(2);
    expect(nodes.slice(0, 4).map((node) => node.getAttribute("title"))).toEqual([
      "Draw 1",
      "Gain 1 mana",
      "Deal 2 damage to the enemy hero",
      "Heal your hero by 3",
    ]);
    expect(nodes[19]?.getAttribute("data-reward")).toBe("edge");
    expect(nodes[19]?.getAttribute("title")).toBe("The edge of the universe");

    const theirs = screen.getByTestId("starscape-opponent");
    expect(theirs.querySelectorAll(".starscape-node")).toHaveLength(20);
    expect(theirs.querySelector("[data-here]")).toBeNull();
    expect(theirs.querySelector("[data-visited]")).toBeNull();
  });
});
