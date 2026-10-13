// The accessibility check spec 38 runs on each route (issue #757): axe-core in the page, WCAG 2.2
// A and AA rules only, and a failure for every serious or critical violation that is not listed in
// `axe-exceptions.ts` for that route.
//
// axe-core is injected from e2e's own node_modules (a dev dependency of this package, never of the
// client), once per page: a `cy.visit` makes a new window, and the check injects again.

import type { AxeResults, ImpactValue, NodeResult, Result } from "axe-core";

import { AXE_EXCEPTIONS, type AxeException } from "./axe-exceptions.ts";
import { timeouts } from "./config.ts";

/** The WCAG 2.0, 2.1 and 2.2 A and AA rule tags (axe-core's own names). */
export const AXE_WCAG_TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22a", "wcag22aa"] as const;

/** The impacts that fail the spec: the issue's "serious or critical". */
export const AXE_BLOCKING_IMPACTS: readonly ImpactValue[] = ["serious", "critical"];

const AXE_SOURCE = "node_modules/axe-core/axe.min.js";

/** The nodes the terminal shows of each violation: enough to find them, few enough to read. */
const AXE_NODES_SHOWN = 5;

type AxeWindow = Cypress.AUTWindow & {
  axe?: { run: (context: Document, options: unknown) => Promise<AxeResults> };
};

/** One violation as the terminal shows it: the rule, its impact, and its first nodes' selectors. */
function describeViolation(violation: Result): string {
  const nodes = violation.nodes
    .slice(0, AXE_NODES_SHOWN)
    .map((node) => `    ${node.target.join(" ")}: ${(node.failureSummary ?? "").replace(/\s+/g, " ")}`);
  const more = violation.nodes.length - AXE_NODES_SHOWN;
  if (more > 0) nodes.push(`    and ${String(more)} more`);
  return [`  ${violation.id} (${violation.impact ?? "?"}): ${violation.help}`, ...nodes].join("\n");
}

/** Whether `exception` covers `node`: the element axe names matches the exception's selector. */
function covers(win: AxeWindow, exception: AxeException, node: NodeResult): boolean {
  const selector = node.target[0];
  if (typeof selector !== "string" || node.target.length !== 1) return false;
  return win.document.querySelector(selector)?.matches(exception.selector) === true;
}

/**
 * Run axe on the page as it stands and fail on a serious or critical violation `route` does not
 * except, or on an exception for `route` that no longer fires.
 */
export function expectNoBlockingViolations(route: string): void {
  cy.readFile(AXE_SOURCE, { log: false }).then((source: string) => {
    cy.window({ log: false }).then((win: AxeWindow) => {
      if (win.axe === undefined) win.eval(source);
      const axe = win.axe;
      if (axe === undefined) throw new Error("axe-core did not load into the page");
      const run = axe.run(win.document, {
        runOnly: { type: "tag", values: [...AXE_WCAG_TAGS] },
        resultTypes: ["violations"],
      });
      cy.wrap<Promise<AxeResults>, AxeResults>(run, { timeout: timeouts.axe, log: false }).then((results) => {
        checkResults(win, route, results);
      });
    });
  });
}

function checkResults(win: AxeWindow, route: string, results: AxeResults): void {
  const blocking = results.violations.filter(
    (violation) => violation.impact != null && AXE_BLOCKING_IMPACTS.includes(violation.impact),
  );
  const excepted = AXE_EXCEPTIONS.filter((exception) => exception.route === route);
  const exceptionsFor = (violation: Result): AxeException[] =>
    excepted.filter((exception) => exception.rule === violation.id);
  // A violation is excepted node by node: a node no exception covers still fails.
  const unexcepted = blocking
    .map((violation) => ({
      ...violation,
      nodes: violation.nodes.filter((node) => !exceptionsFor(violation).some((exception) => covers(win, exception, node))),
    }))
    .filter((violation) => violation.nodes.length > 0);
  const stale = excepted.filter(
    (exception) =>
      !blocking.some(
        (violation) => violation.id === exception.rule && violation.nodes.some((node) => covers(win, exception, node)),
      ),
  );
  if (blocking.length > 0) {
    const report = [
      `[axe] ${route}: ${String(blocking.length)} serious or critical, ${String(unexcepted.length)} not excepted`,
      ...unexcepted.map(describeViolation),
    ].join("\n");
    cy.task("log", report, { log: false });
  }
  cy.then(() => {
    expect(
      unexcepted.map((violation) => violation.id),
      `${route}: serious or critical axe violations (the terminal lists their nodes)`,
    ).to.deep.equal([]);
    expect(
      stale.map((exception) => `${exception.rule} ${exception.selector}`),
      `${route}: exceptions in axe-exceptions.ts that no longer fire`,
    ).to.deep.equal([]);
  });
}
