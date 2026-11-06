// Component specs load the production global CSS for real layout.

import { mount } from "cypress/react";

import "../../apps/web/src/index.css";
import { loadWasm } from "../../apps/web/src/wasm/index.ts";

declare global {
  // eslint-disable-next-line @typescript-eslint/no-namespace -- the shape Cypress documents.
  namespace Cypress {
    interface Chainable {
      /** Mount a React element into `[data-cy-root]` (support/component-index.html). */
      mount: typeof mount;
    }
  }
}

Cypress.Commands.add("mount", mount);

// A card face is worked out in Rust (jackioh-shared, #3106) and read through the WebAssembly module, so every
// component spec that draws one needs the module loaded; `loadWasm()` is idempotent, and a spec that loads it
// itself waits on the same promise.
before(() => {
  cy.wrap(loadWasm(), { timeout: 60_000 });
});

// An uncaught error fails the spec: the default, made explicit as support/e2e.ts does.
Cypress.on("uncaught:exception", () => true);
