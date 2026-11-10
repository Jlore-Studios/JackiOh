// The dungeon run's map (DR4, DR14 part F, #2707): the whole map, each node's kind with a drawn
// icon and a label, each boss's preseeded relic, and where the player stands. Only the nodes the one
// stood on leads to can be chosen (`go`, R3583000); every other node stays visible but disabled.
// Presentation only (CLAUDE.md rule 7): the run answers every rule, and every word goes through
// `t()` at render, never at module scope.

import type { ReactElement } from "react";

import type { RunContent, RunNodeKind, RunState } from "@jackioh/run";

import { t } from "../../i18n/i18n.ts";
import { NodeIcon } from "./NodeIcon.tsx";
import { dungeonMapTestid } from "./testids.ts";
import { passiveName, type SendRunAction } from "./RunDisplay.tsx";

/** Each node kind's untranslated label; `t()` at render, never at module scope. */
const NODE_LABELS: Readonly<Record<RunNodeKind, string>> = {
  fight: "Fight",
  elite: "Elite",
  ghost: "Ghost",
  miniBoss: "Mini-boss",
  boss: "Final boss",
  shop: "Shop",
  campfire: "Campfire",
  event: "Event",
  chest: "Chest",
  shrine: "Shrine",
};

/**
 * R3583000: the next row's nodes the run lets `go` step onto, by index, as the run's own `next` says:
 * every node of the first row before the first step, then those the node stood on leads to. The run
 * still answers every step (CLAUDE.md rule 7); this only says which buttons are live.
 */
function nextCols(run: RunState): readonly number[] {
  if (run.row === null) return (run.map[0] ?? []).map((_node, col) => col);
  return run.map[run.row]?.[run.col]?.next ?? [];
}

type MapScreenProps = {
  run: RunState;
  content: RunContent;
  send: SendRunAction;
};

/** The whole map: every node visible, only the nodes the one stood on leads to live. */
export function MapScreen({ run, content, send }: MapScreenProps): ReactElement {
  const next = (run.row ?? -1) + 1;
  const open = nextCols(run);
  return (
    <section className="dungeon-run-map" data-testid={dungeonMapTestid.map} aria-label={t("Map")}>
      {run.map.map((nodes, row) => (
        <div className="dungeon-run-map__row" data-testid={dungeonMapTestid.mapRow(row)} key={row}>
          {nodes.map((node, col) => {
            const name = t(NODE_LABELS[node.kind]);
            const live = row === next && open.includes(col);
            const current = run.row === row && run.col === col;
            const label = t("{node} {col}", { node: name, col: String(col + 1) });
            return (
              <button
                type="button"
                className="dungeon-run-map__node"
                data-testid={dungeonMapTestid.mapNode(row, col)}
                data-kind={node.kind}
                data-live={live ? "true" : undefined}
                data-current={current ? "true" : undefined}
                key={col}
                disabled={!live}
                aria-label={label}
                title={label}
                onClick={() => {
                  send({ type: "go", col });
                }}
              >
                <span className="dungeon-run-map__icon" aria-hidden="true">
                  <NodeIcon kind={node.kind} />
                </span>
                <span className="dungeon-run-map__label">{name}</span>
                {node.enemy === null ? null : (
                  <span className="dungeon-run-map__enemy">{t(node.enemy.name)}</span>
                )}
                {node.artifact === null ? null : (
                  <span
                    className="dungeon-run-map__relic"
                    data-testid={dungeonMapTestid.mapArtifact(row, col)}
                  >
                    {t("Relic: {name}", { name: passiveName(content, node.artifact) })}
                  </span>
                )}
                {current ? (
                  <span className="dungeon-run-map__current" data-testid={dungeonMapTestid.mapCurrent}>
                    {t("You are here")}
                  </span>
                ) : null}
              </button>
            );
          })}
        </div>
      ))}
    </section>
  );
}
