// The Dungeon run (DR14 part D, #2705; B13): the one page at `/dungeon` that switches between the run's
// screens by what the run waits on. The start screen shows while no run stands, and for a run kept on
// the device until the player picks it up (B19): the screen offers Continue and Abandon rather than
// dropping them into a fight. A fight is the practice board (part E), every other phase, the map and its
// tiles and the run-over screen (part F). The store holds the run (`store.ts`); the screens show it.

import { useEffect, useRef, useState, type ReactElement } from "react";

import { FightScreen } from "./fight/FightScreen.tsx";
import { RunScreen } from "./map/RunScreen.tsx";
import { StartScreen } from "./start/StartScreen.tsx";
import { runFinished, useDungeon } from "./store.ts";
import { dungeonTestid } from "./testids.ts";

export default function DungeonPage(): ReactElement {
  const dungeon = useDungeon();
  const { run } = dungeon;
  // A run that is finished is on its run-over screen already; one folded from the device waits for Continue.
  const [entered, setEntered] = useState(() => run !== null && runFinished(run));
  // A run given up leaves nothing to enter: the next one is entered when it starts.
  if (run === null && entered) setEntered(false);
  const kind = run === null || !entered ? "start" : run.phase.phase === "fight" ? "fight" : "run";
  // Each screen opens at its top, as a new address does (#1756): the start screen is long.
  const shown = useRef(kind);
  useEffect(() => {
    if (shown.current === kind) return;
    shown.current = kind;
    window.scrollTo(0, 0);
  }, [kind]);

  let screen: ReactElement;
  if (kind === "start") {
    screen = (
      <StartScreen
        dungeon={dungeon}
        onEnter={() => {
          setEntered(true);
        }}
      />
    );
  } else if (kind === "fight") {
    screen = <FightScreen dungeon={dungeon} />;
  } else {
    screen = <RunScreen dungeon={dungeon} />;
  }
  return (
    <div className="dungeon" data-testid={dungeonTestid.screen}>
      {screen}
    </div>
  );
}
