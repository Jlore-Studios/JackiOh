// A seat's starscape map (Stinkyscape #148 Alteran Reach, R1847000). It renders the seat's view alone, which
// carries the map for both seats once its player has first traveled (CLAUDE.md rule 7); the travel is the card's
// Activate ability, offered and refused by the engine.

import type { ReactElement } from "react";

import type { StarscapeNodeView, StarscapeView } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import type { Side } from "./contract.ts";
import "./starscape.css";

function rewardWords(node: StarscapeNodeView): string {
  const amount = node.amount ?? 0;
  switch (node.reward) {
    case "heal":
      return t("Heal your hero by {amount}", { amount });
    case "draw":
      return t("Draw {amount}", { amount });
    case "mana":
      return t("Gain {amount} mana", { amount });
    case "damage":
      return t("Deal {amount} damage to the enemy hero", { amount });
    case "edge":
      return t("The edge of the universe");
  }
}

export default function StarscapeMap({ side, starscape }: { side: Side; starscape: StarscapeView }): ReactElement {
  const { nodes, at } = starscape;
  const label = t("Starscape map: node {at} of {total}", { at, total: nodes.length });
  return (
    <span className="starscape" data-testid={`starscape-${side}`} data-at={at} aria-label={label} title={label}>
      {nodes.map((node, index) => (
        <span
          key={index}
          className="starscape-node"
          data-reward={node.reward}
          data-here={index + 1 === at ? "true" : undefined}
          data-visited={index + 1 < at ? "true" : undefined}
          title={rewardWords(node)}
        />
      ))}
    </span>
  );
}
