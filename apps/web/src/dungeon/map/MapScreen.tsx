// The dungeon run's map (DR4, DR14 part F, #2707): the whole current act ahead, each node's kind
// with an icon and a label, each boss's preseeded relic, and where the player stands. Only the
// next row's nodes can be chosen (`go`); every other node stays visible but disabled.
// Presentation only (CLAUDE.md rule 7): the run answers every rule, and every word goes through
// `t()` at render, never at module scope.

import type { ReactElement } from "react";

import type { RunContent, RunNodeKind, RunState } from "@jackioh/run";

import { t } from "../../i18n/i18n.ts";
import { dungeonMapTestid } from "./testids.ts";
import { passiveName, type SendRunAction } from "./RunDisplay.tsx";

/** Each node kind's icon and untranslated label; `t()` at render, never at module scope. */
const NODE_META: Readonly<Record<RunNodeKind, { icon: string; label: string }>> = {
  fight: { icon: "⚔️", label: "Fight" },
  elite: { icon: "🛡️", label: "Elite" },
  ghost: { icon: "👻", label: "Ghost" },
  miniBoss: { icon: "🐲", label: "Mini-boss" },
  boss: { icon: "👑", label: "Final boss" },
  shop: { icon: "🛒", label: "Shop" },
  campfire: { icon: "🔥", label: "Campfire" },
  event: { icon: "❓", label: "Event" },
  chest: { icon: "🎁", label: "Chest" },
  shrine: { icon: "⛩️", label: "Shrine" },
};

/** Whether the row holds an act's end: a mini-boss's or the final boss's row. */
function isBossRow(run: RunState, row: number): boolean {
  return (run.map[row] ?? []).some((node) => node.kind === "miniBoss" || node.kind === "boss");
}

/**
 * The current act's rows with their absolute numbers: from row 0, or just past the last boss row
 * before the next row to visit, through the next row holding a mini-boss or boss, inclusive. Just
 * past a beaten boss the completed act stays behind, so the only selectable row is always shown.
 */
function currentActRows(run: RunState): { row: number }[] {
  const next = (run.row ?? -1) + 1;
  let start = 0;
  for (let row = 0; row < next && row < run.map.length; row += 1) {
    if (isBossRow(run, row)) start = row + 1;
  }
  let end = run.map.length - 1;
  for (let row = next; row < run.map.length; row += 1) {
    if (isBossRow(run, row)) {
      end = row;
      break;
    }
  }
  const rows: { row: number }[] = [];
  for (let row = start; row <= end; row += 1) rows.push({ row });
  return rows;
}

type MapScreenProps = {
  run: RunState;
  content: RunContent;
  send: SendRunAction;
};

/** The current act: every node visible, only the next row's nodes live. */
export function MapScreen({ run, content, send }: MapScreenProps): ReactElement {
  const next = (run.row ?? -1) + 1;
  return (
    <section className="dungeon-run-map" data-testid={dungeonMapTestid.map} aria-label={t("Map")}>
      {currentActRows(run).map(({ row }) => (
        <div className="dungeon-run-map__row" data-testid={dungeonMapTestid.mapRow(row)} key={row}>
          {(run.map[row] ?? []).map((node, col) => {
            const meta = NODE_META[node.kind];
            const live = row === next;
            const current = run.row === row && run.col === col;
            const label = t("{node} {col}", { node: t(meta.label), col: String(col + 1) });
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
                  {meta.icon}
                </span>
                <span className="dungeon-run-map__label">{t(meta.label)}</span>
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
