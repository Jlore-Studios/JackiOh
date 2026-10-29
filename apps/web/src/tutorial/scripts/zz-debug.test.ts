import { it } from "vitest";
import { appendFileSync } from "node:fs";
import { findInstance } from "@jackioh/engine";
import { playLesson } from "../harness.ts";
import { script } from "./spells.ts";
import { stepsOf } from "./testkit.ts";

const OUT = "/tmp/claude-0/-home-user-JackiOh/10b2088a-cedc-5d64-ab37-fec0e3592b0d/scratchpad/spells-scan.txt";
const FROM = Number(process.env.SCAN_FROM ?? "1");
const TO = Number(process.env.SCAN_TO ?? "40");

it("scan", () => {
  for (let n = FROM; n <= TO; n += 1) {
    const seed = `tutorial-spells-${String(n)}`;
    try {
      const run = playLesson("spells", { policy: "coach", seed });
      const shown = run.shown.map((s) => s.id);
      const all = script.steps.map((s) => s.id);
      const inOrder = JSON.stringify(shown) === JSON.stringify(all);
      const offCoach = run.humanActions.filter((e) => !e.byCoach || e.refused !== null).length;
      const steps = stepsOf(run);
      const aimed = ["core-035", "core-044"].map((defId) => {
        const cast = steps.find((st) => st.action.playerId === run.humanSeat && st.action.type === "play" && findInstance(st.before, st.action.instanceId)?.defId === defId);
        const t = cast?.action.type === "play" ? cast.action.targets?.[0] : undefined;
        return t?.pick === "instance";
      });
      const ok = run.winner === run.humanSeat && inOrder && offCoach === 0 && aimed.every(Boolean) && run.humanTurns <= 9;
      appendFileSync(OUT, `${seed} ${ok ? "OK" : "no"} win=${String(run.winner)} turns=${run.humanTurns} inOrder=${inOrder} off=${offCoach} aimed=${aimed.join(",")} missing=${all.filter((id) => !shown.includes(id)).join(",")}\n`);
    } catch (error) {
      appendFileSync(OUT, `${seed} error ${String(error)}\n`);
    }
  }
}, 3_600_000);
