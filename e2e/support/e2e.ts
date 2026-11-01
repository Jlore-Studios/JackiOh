// Loaded before every spec (cypress.config.ts -> supportFile).

import "./commands.ts";

// BUILD M8: "No fixed `cy.wait(ms)`; waits are on `data-animating` clearing or on testids."
// The rule is enforced, not just documented: a numeric wait fails the spec that used it.
const refuseFixedWait = (
  originalFn: (...args: unknown[]) => unknown,
  ...args: unknown[]
): unknown => {
  if (typeof args[0] === "number") {
    throw new Error(
      "BUILD M8 forbids a fixed cy.wait(ms). Wait on `data-animating` clearing (cy.settled()) " +
        "or on a testid instead.",
    );
  }
  return originalFn(...args);
};

// `as never` only silences the overload gymnastics of Commands.overwrite; the guard itself is
// fully typed above.
Cypress.Commands.overwrite("wait", refuseFixedWait as never);

// Every TEST starts with no socket of its own and no match left running: `support/tasks/index.ts`
// resets the Node clients once per spec FILE, not once per attempt (`retries.runMode` is 1), so
// without this a retried networked spec opens against the match its first attempt left behind, and
// a `409 already_in_match` buries the real failure.
beforeEach(() => {
  cy.task("wsPlayer", { action: "reset" }, { log: false });
});

// An uncaught error in the client is a spec failure; nothing here swallows it (Cypress's default,
// made explicit for reviewers).
Cypress.on("uncaught:exception", () => true);
