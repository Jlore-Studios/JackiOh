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
        // The verdict throw runs as its own command so the numbers above print first:
        // a throw inside this callback would skip the queued `log` task.
        cy.then(() => expectPass(report));
      });
  });

  it("combat: two attacks and a spell through the effects layer hold their budgets", () => {
    setupBusyBoard();
    let collect: (() => Promise<FrameSample>) | null = null;
    cy.window({ log: false }).then((win) => {
      collect = startProbe(win, FRAME_PROBE_COMBAT_SECONDS).done;
    });
    // The only legal attack target is the lane-5 Taunt wall (Right-house defender):
    // the lane-1 Megabase cannot be attacked (Can't Defend) and cannot be spell-targeted
    // (Immune to Spells). Paragon Mk2 (lane 3) pops the wall's Divine Shield and Tylannt
    // (lane 2, 4 HP) survives the strike back; the wall then dies and Reborns while Pocket
    // Peptides answers on its own, so no prompt ever opens mid-window. Neither side set
    // Traitorous Blood, My Pawn wants a lethal hero attack and Explosive Runes a hero
    // attack, so the attacks trigger nothing else.
    cy.unitIds("p1").then((attackers) => {
      const charger = attackers[2];
      const second = attackers[1];
      if (charger === undefined || second === undefined) throw new Error("combat: p1 lacks attackers");
      cy.unitIds("p2").then((defenders) => {
        const wall = defenders[4];
        if (wall === undefined) throw new Error("combat: p2 holds no wall to hit");
        cy.attack(charger, { card: wall });
        cy.unitIds("p2").then((rest) => {
          const still = rest[4];
          if (still === undefined) throw new Error("combat: p2 holds no wall to hit");
          cy.attack(second, { card: still });
        });
      });
    });
    // The first preference spell in hand, through the play picker as any player casts it;
    // a targeted one declares the opposing hero, which outlives the window while no set
    // trap answers a spell on the hero.
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
          cy.playCard(instanceId, { answers: [{ kind: "target", hero: "opponent" }] });
        } else {
          cy.playCard(instanceId);
        }
      });
      cy.get(PROMPT).should("not.exist");
    });
    cy.then({ timeout: PROBE_TIMEOUT_MS }, () => {
      if (collect === null) throw new Error("combat: the probe never started");
      return collect();
    }).then((sample: FrameSample) => {
      const report = reportPhase("combat", sample, COMBAT_BUDGET);
      logReport({ seed: BUSY_BOARD_SEED, phases: [report], idleAnimations: null });
      // The verdict throw runs as its own command so the numbers above print first:
      // a throw inside this callback would skip the queued `log` task.
      cy.then(() => expectPass(report));
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
