// Component specs load the production global CSS for real layout.

import { mount } from "cypress/react";

import "../../apps/web/src/index.css";

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

// An uncaught error fails the spec: the default, made explicit as support/e2e.ts does.
Cypress.on("uncaught:exception", () => true);
