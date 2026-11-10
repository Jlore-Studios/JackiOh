// Spec 50 — the busy-board frame probe (issue #3605; SPEC §2.4's lane, backrow and hand caps,
// §3–§4's combat, §6's keywords; BUILD M5-T3's `window.__jackioh`, M5-T4's assertion-only waits).
// The worst case both sides share (`support/busyBoard.ts`, quoted by #3604's later parts): five
// looping minions and two set traps a side, Mythic and Legendary animated faces (#3386), a full
// hand of playable cards. Idle samples the untouched board with the pointer over a minion;
// combat samples a scripted turn of two attacks and a spell through the effects layer (FxLayer).
// The third `it` proves the gate bites: an injected busy loop must fail the p95 budget.
//
// Headless approximations, not pointer-exact costs: Chromium has no pointer, so the idle hover is
// dispatched mouse events (`hoverBoardMinion`), and `document.getAnimations()` counts compositor
// and CSS animations alike, so it is reported, never judged. Judged are only each phase's p95 and
// long-task count, with 5× headroom for CI's shared runners; this spec stays out of the PR smoke
// list (daily super run only).

import {
  FRAME_P95_BUDGET_IDLE_MS,
  FRAME_PROBE_COMBAT_SECONDS,
  FRAME_PROBE_IDLE_SECONDS,
  FRAME_PROBE_LOOP_SECONDS,
} from "../../../apps/web/src/perf/frameBudgets.ts";
import type { PhaseReport, ProbeReport } from "../../../apps/web/src/perf/frameStats.ts";
import {
  BUSY_BOARD_COMBAT_SPELL_DEF_IDS,
  BUSY_BOARD_SEED,
  BUSY_BOARD_TARGETED_SPELLS,
  busyBoardNameOf,
  setupBusyBoard,
} from "../../support/busyBoard.ts";
import {
  COMBAT_BUDGET,
  IDLE_BUDGET,
  formatProbeReport,
  hoverBoardMinion,
  reportPhase,
  startProbe,
  type FrameSample,
} from "../../support/frameProbe.ts";
import { PROMPT, cardId, ts } from "../../support/testids.ts";

const PROBE_JSON = ["1", "true"].includes(String(Cypress.expose("probeJson") ?? ""));

/** The sampling promises outlast `defaultCommandTimeout` (idle 5 s, combat 8 s). */
const PROBE_TIMEOUT_MS = 120_000;

function logReport(report: ProbeReport): void {
  cy.task("log", PROBE_JSON ? JSON.stringify(report) : formatProbeReport(report));
}

function expectPass(report: PhaseReport): void {
  expect(report.verdict.ok, report.verdict.failedBy ?? `${report.name} over budget`).to.eq(true);
}

describe("Spec 50 — busy-board frame probe (issue #3605)", () => {
  it("idle: the untouched worst-case board holds its p95 and long-task budgets", () => {
    setupBusyBoard();
    cy.unitIds("p1").then((units) => {
      const first = units[0];
      if (first === undefined) throw new Error("idle: p1 holds no board minion to hover");
      cy.window({ log: false }).then((win) => {
        hoverBoardMinion(win, ts(cardId(first)));
      });
    });
    cy.window({ log: false })
      .then({ timeout: PROBE_TIMEOUT_MS }, (win) =>
        startProbe(win, FRAME_PROBE_IDLE_SECONDS).done(),
      )
      .then((sample: FrameSample) => {
        const report = reportPhase("idle", sample, IDLE_BUDGET);
        logReport({ seed: BUSY_BOARD_SEED, phases: [report], idleAnimations: sample.runningAnimations });
        expectPass(report);
      });
  });

  it("combat: two attacks and a spell through the effects layer hold their budgets", () => {
    setupBusyBoard();
    let collect: (() => Promise<FrameSample>) | null = null;
    cy.window({ log: false }).then((win) => {
      collect = startProbe(win, FRAME_PROBE_COMBAT_SECONDS).done;
    });
    // Paragon Mk2 (lane 3) and the lane-4 looper hit the lane-1 Megabase: 0 attack back, 20
    // health, so both attackers live and the board stays the worst case throughout the window.
    cy.unitIds("p1").then((attackers) => {
      const charger = attackers[2];
      const looper = attackers[3];
      if (charger === undefined || looper === undefined) throw new Error("combat: p1 lacks attackers");
      cy.unitIds("p2").then((defenders) => {
        const wall = defenders[0];
        if (wall === undefined) throw new Error("combat: p2 holds no target");
        cy.attack(charger, { card: wall });
        cy.unitIds("p2").then((rest) => {
          const still = rest[0];
          if (still === undefined) throw new Error("combat: p2 holds no second target");
          cy.attack(looper, { card: still });
        });
      });
    });
    // The first preference spell in hand, through the play picker as any player casts it;
    // a targeted one declares the lane-1 wall, which outlives the window.
    cy.unitIds("p2").then((defenders) => {
      const target = defenders[0];
      if (target === undefined) throw new Error("combat: p2 holds no target");
      cy.gameState().then((state) => {
        const hand = (state.players.p1 as { hand?: { id: string; defId: string }[] }).hand ?? [];
        const pick = BUSY_BOARD_COMBAT_SPELL_DEF_IDS.find((defId) =>
          hand.some((card) => card.defId === defId),
        );
        if (pick === undefined) throw new Error("combat: p1 holds no combat spell");
        return pick;
      }).then((defId: string) => {
        cy.handCardByName(busyBoardNameOf(defId)).then((instanceId) => {
          if (BUSY_BOARD_TARGETED_SPELLS.includes(defId)) {
            cy.playCard(instanceId, { answers: [{ kind: "target", cards: [target] }] });
          } else {
            cy.playCard(instanceId);
          }
        });
        cy.get(PROMPT).should("not.exist");
      });
    });
    cy.then({ timeout: PROBE_TIMEOUT_MS }, () => {
      if (collect === null) throw new Error("combat: the probe never started");
      return collect();
    }).then((sample: FrameSample) => {
      const report = reportPhase("combat", sample, COMBAT_BUDGET);
      logReport({ seed: BUSY_BOARD_SEED, phases: [report], idleAnimations: null });
      expectPass(report);
    });
  });

  it("expensive loop: a deliberately busy board fails the p95 budget, naming it", () => {
    setupBusyBoard();
    cy.window({ log: false })
      .then({ timeout: PROBE_TIMEOUT_MS }, (win) => {
        // Blocks longer than the idle p95 budget every tick: the p95 must land over budget and
        // say which constant it crossed. Long tasks trip too, but the p95 names the failure.
        const blockMs = FRAME_P95_BUDGET_IDLE_MS + 60;
        const interval = win.setInterval(() => {
          const start = win.performance.now();
          while (win.performance.now() - start < blockMs) {
            // Spin: the injected load is the busy-wait itself.
          }
        }, 120);
        return startProbe(win, FRAME_PROBE_LOOP_SECONDS)
          .done()
          .finally(() => {
            win.clearInterval(interval);
          });
      })
      .then((sample: FrameSample) => {
        const report = reportPhase("idle+loop", sample, IDLE_BUDGET);
        expect(report.verdict.ok, "the injected loop trips the p95 budget").to.eq(false);
        expect(report.verdict.failedBy ?? "").to.contain("FRAME_P95_BUDGET_IDLE_MS");
      });
  });
});
