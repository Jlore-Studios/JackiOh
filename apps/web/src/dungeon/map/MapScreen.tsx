// The dungeon run's map (DR14 part F, #2707): the whole map as one climbing path read bottom
// to top (#3587), each node's kind with a drawn icon and a label, each boss's preseeded relic,
// and where the player stands. Only the nodes the one stood on leads to can be chosen (`go`,
// R3583000); every other node stays visible but unfocusable as a choice, still focusable for its
// details. Presentation only (CLAUDE.md rule 7): the run answers every rule, and every word goes
// through `t()` at render, never at module scope.

import { useEffect, useRef, type ReactElement } from "react";

import type { RunContent, RunNodeKind, RunState } from "@jackioh/run";

import { t } from "../../i18n/i18n.ts";
import { MapGraph } from "./MapGraph.tsx";
import { NodeIcon } from "./NodeIcon.tsx";
import { dungeonMapTestid } from "./testids.ts";
import type { SendRunAction } from "./RunDisplay.tsx";

/** Each node kind's untranslated label; `t()` at render, never at module scope. */
export const NODE_LABELS: Readonly<Record<RunNodeKind, string>> = {
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
export function nextCols(run: RunState): readonly number[] {
  if (run.row === null) return (run.map[0] ?? []).map((_node, col) => col);
  return run.map[run.row]?.[run.col]?.next ?? [];
}

type MapScreenProps = {
  run: RunState;
  content: RunContent;
  /** The columns stepped onto, one per `go`, in order; the graph marks the path walked. */
  trail: readonly number[];
  send: SendRunAction;
};

/** The whole map: one climbing path, then the legend naming the node kinds. */
export function MapScreen({ run, content, trail, send }: MapScreenProps): ReactElement {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    // Before the first step there is no current node to scroll to; the live first row sits at
    // the bottom, so the map opens there instead of at the final boss.
    if (run.row === null && scrollRef.current !== null) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [run.row]);
  return (
    <section className="dungeon-run-map" data-testid={dungeonMapTestid.map} aria-label={t("Map")}>
      <div className="dungeon-run-map__scroll" ref={scrollRef}>
        <MapGraph run={run} content={content} trail={trail} send={send} />
      </div>
      <div
        className="dungeon-run-map__legend"
        data-testid={dungeonMapTestid.mapLegend}
        role="list"
        aria-label={t("Legend")}
      >
        {(Object.keys(NODE_LABELS) as RunNodeKind[]).map((kind) => (
          <span
            className="dungeon-run-map__legend-item"
            data-testid={dungeonMapTestid.mapLegendItem(kind)}
            role="listitem"
            key={kind}
          >
            <NodeIcon kind={kind} />
            {t(NODE_LABELS[kind])}
          </span>
        ))}
      </div>
    </section>
  );
}
