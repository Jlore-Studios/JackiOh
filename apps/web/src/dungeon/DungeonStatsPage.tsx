// `/dungeon/stats` (DR14 part D, #2705): the Sandbox's stats and leaderboards, which part G draws
// (`stats/DungeonStats.tsx`); this is the route chunk's default export, so the page loads as the others do.

import type { ReactElement } from "react";

import { DungeonStats } from "./stats/DungeonStats.tsx";

export default function DungeonStatsPage(): ReactElement {
  return <DungeonStats />;
}
