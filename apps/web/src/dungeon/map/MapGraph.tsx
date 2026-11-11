// The dungeon run's climbing-path map (#3587): one tall graph read bottom to top. Round
// kind-colored badges sit at their grid columns with a deterministic jitter, one SVG line joins
// each node to each of the next row's it leads to, the walked path draws bold, the current node's
// outgoing edges highlight, and only the nodes the one stood on leads to are live buttons.
// Presentation only (CLAUDE.md rule 7): the run answers every step, the screen only reads its
// `next` edges, and every word goes through `t()` at render, never at module scope.

import { useEffect, useRef, useState, type ReactElement } from "react";

import type { RunContent, RunNodeKind, RunState } from "@jackioh/run";

import { t } from "../../i18n/i18n.ts";
import { NodeIcon } from "./NodeIcon.tsx";
import { dungeonMapTestid } from "./testids.ts";
import { NODE_LABELS, nextCols } from "./MapScreen.tsx";
import { passiveName, type SendRunAction } from "./RunDisplay.tsx";

/** One grid column's width, so a 7-wide map spans 322 px and fits a 360 px phone. */
export const MAP_COL_W = 46;
/** One map row's height: a badge with its captions stacked under it. */
export const MAP_ROW_H = 140;
/** A badge's diameter; the button around it is never smaller, so every tap target clears 44 px. */
export const MAP_BADGE = 44;
/** Bosses' badges draw larger, for the emblem at the top and the two mini-bosses. */
export const MAP_BADGE_BOSS = 64;
/** The jitter's reach each way: the map reads as drawn paths, never a spreadsheet. */
export const MAP_JITTER = 5;
/** The walked path's line width, bold against the thin lines around it. */
export const MAP_WALKED_W = 3;

/**
 * A node's jitter from its row and column: an integer hash inside ±`MAP_JITTER`, never
 * `Math.random`, so the map holds still across renders.
 */
export function mapJitter(row: number, column: number): { x: number; y: number } {
  const span = MAP_JITTER * 2 + 1;
  const x = (((row * 7 + column * 13 + 3) % span) + span) % span - MAP_JITTER;
  const y = (((row * 11 + column * 5 + 7) % span) + span) % span - MAP_JITTER;
  return { x, y };
}

/** A badge's diameter: bosses draw larger than every other kind. */
export function mapBadge(kind: RunNodeKind): number {
  return kind === "boss" || kind === "miniBoss" ? MAP_BADGE_BOSS : MAP_BADGE;
}

/**
 * A node's badge center in the graph: its grid column across, row 0 at the bottom. The badge's
 * top sits at `y - badge / 2`; the captions stack under it inside the same button.
 */
export function mapCenter(
  rowCount: number,
  row: number,
  column: number,
  badge: number,
): { x: number; y: number; top: number } {
  const jitter = mapJitter(row, column);
  const x = column * MAP_COL_W + MAP_COL_W / 2 + jitter.x;
  const y = (rowCount - 1 - row) * MAP_ROW_H + 52 + jitter.y;
  return { x, y, top: y - badge / 2 };
}

/** One edge of the run's map: the node at (`fromRow`, `fromCol`) leads to `to`, an index of the next row. */
export type MapEdge = { fromRow: number; fromCol: number; to: number };

/** Every edge of the run's map, as each node's own `next` names it (R3583000). */
export function mapEdges(run: RunState): MapEdge[] {
  return run.map.flatMap((nodes, fromRow) =>
    nodes.flatMap((node, fromCol) => node.next.map((to) => ({ fromRow, fromCol, to }))),
  );
}

type MapGraphProps = {
  run: RunState;
  content: RunContent;
  /** The columns stepped onto, one per `go`, in order; a short trail simply marks what lines up. */
  trail: readonly number[];
  send: SendRunAction;
};

/**
 * The whole map as one graph: the lines under the badges, every node focusable in climbing
 * order (row 0 first), the current node scrolled into view. Only the nodes the one stood on
 * leads to are live; the rest keep `aria-disabled` and tap open their details instead of
 * sending anything.
 */
export function MapGraph({ run, content, trail, send }: MapGraphProps): ReactElement {
  const next = (run.row ?? -1) + 1;
  const open = nextCols(run);
  const rowCount = run.map.length;
  const cols = Math.max(1, ...run.map.flatMap((nodes) => nodes.map((node) => node.column + 1)));
  const width = cols * MAP_COL_W;
  const height = rowCount * MAP_ROW_H;
  const edges = mapEdges(run);
  const centerOf = (row: number, col: number): { x: number; y: number } => {
    const node = run.map[row]?.[col];
    const column = node?.column ?? col;
    return mapCenter(rowCount, row, column, MAP_BADGE);
  };
  // The tapped node's details, held open until another node is tapped (`hover` and focus show
  // theirs through CSS alone).
  const [openKey, setOpenKey] = useState<string | null>(null);
  const currentRef = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    // jsdom (the unit tests) draws no layout, so the scroll call stays optional.
    currentRef.current?.scrollIntoView?.({ block: "nearest" });
  }, [run.row, run.col]);
  return (
    <div
      className="dungeon-run-map__graph"
      data-testid={dungeonMapTestid.mapGraph}
      style={{ width, height }}
    >
      <svg
        className="dungeon-run-map__lines"
        width={width}
        height={height}
        viewBox={`0 0 ${String(width)} ${String(height)}`}
        aria-hidden="true"
      >
        {edges.map(({ fromRow, fromCol, to }) => {
          const from = centerOf(fromRow, fromCol);
          const into = centerOf(fromRow + 1, to);
          const walked = trail[fromRow] === fromCol && trail[fromRow + 1] === to;
          const live =
            run.row !== null && fromRow === run.row && fromCol === run.col && open.includes(to);
          const dim = run.row !== null && fromRow < run.row && !walked;
          return (
            <line
              key={`${String(fromRow)}-${String(fromCol)}-${String(to)}`}
              data-testid={dungeonMapTestid.mapEdge(fromRow, fromCol, to)}
              data-walked={walked ? "true" : undefined}
              data-live={live ? "true" : undefined}
              data-dim={dim ? "true" : undefined}
              x1={from.x}
              y1={from.y}
              x2={into.x}
              y2={into.y}
            />
          );
        })}
      </svg>
      {run.map.map((nodes, row) =>
        nodes.map((node, col) => {
          const name = t(NODE_LABELS[node.kind]);
          const live = row === next && open.includes(col);
          const current = run.row === row && run.col === col;
          const walked = trail[row] === col;
          const past = run.row !== null && row < run.row;
          const badge = mapBadge(node.kind);
          const { x, top } = mapCenter(rowCount, row, node.column, badge);
          const key = `${String(row)}-${String(col)}`;
          const label =
            node.enemy === null
              ? t("{node} {col}", { node: name, col: String(col + 1) })
              : t("{node} {col}: {enemy}", {
                  node: name,
                  col: String(col + 1),
                  enemy: t(node.enemy.name),
                });
          const relic = node.artifact === null ? null : passiveName(content, node.artifact);
          return (
            <button
              type="button"
              className="dungeon-run-map__node"
              data-testid={dungeonMapTestid.mapNode(row, col)}
              data-kind={node.kind}
              data-live={live ? "true" : undefined}
              data-current={current ? "true" : undefined}
              data-walked={walked ? "true" : undefined}
              data-dim={past && !walked ? "true" : undefined}
              data-open={openKey === key ? "true" : undefined}
              key={key}
              ref={current ? currentRef : undefined}
              style={{ left: x, top }}
              aria-label={label}
              aria-disabled={live ? undefined : "true"}
              title={label}
              onClick={() => {
                if (live) {
                  send({ type: "go", col });
                } else {
                  setOpenKey(openKey === key ? null : key);
                }
              }}
            >
              <span
                className="dungeon-run-map__badge"
                aria-hidden="true"
                style={{ width: badge, height: badge }}
              >
                <NodeIcon kind={node.kind} />
              </span>
              <span className="dungeon-run-map__label">{name}</span>
              {node.enemy === null ? null : (
                <span className="dungeon-run-map__enemy">{t(node.enemy.name)}</span>
              )}
              {relic === null ? null : (
                <span
                  className="dungeon-run-map__relic"
                  data-testid={dungeonMapTestid.mapArtifact(row, col)}
                >
                  {t("Relic: {name}", { name: relic })}
                </span>
              )}
              {walked ? <span className="dungeon-run-map__visited">{t("Visited")}</span> : null}
              {current ? (
                <span className="dungeon-run-map__current" data-testid={dungeonMapTestid.mapCurrent}>
                  {t("You are here")}
                </span>
              ) : null}
              <span
                className="dungeon-run-map__tip"
                data-testid={dungeonMapTestid.mapTip(row, col)}
                aria-hidden="true"
              >
                {name}
                {node.enemy === null ? null : ` · ${t(node.enemy.name)}`}
                {relic === null ? null : ` · ${t("Relic: {name}", { name: relic })}`}
              </span>
            </button>
          );
        }),
      )}
    </div>
  );
}
