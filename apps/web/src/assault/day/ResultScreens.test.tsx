import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";

import { assaultTestid } from "../testids.ts";
import { RECORD } from "../test/fixtures.ts";
import { AbandonedScreen, AfterLevelScreen, LostScreen, RepelledScreen } from "./ResultScreens.tsx";

afterEach(cleanup);

it("the after-level, repelled and abandoned screens", () => {
  const { rerender } = render(<AfterLevelScreen beaten={1} next={2} onContinue={() => undefined} />);
  expect(screen.getByTestId(assaultTestid.result)).toHaveTextContent("Level 1 beaten");

  rerender(<LostScreen record={RECORD} onTryAgain={() => undefined} onBack={() => undefined} />);
  expect(screen.getByTestId(assaultTestid.result)).toHaveTextContent("The Assault repelled you");
  expect(screen.getByTestId(assaultTestid.record)).toHaveTextContent("Attempts: 2");

  rerender(<RepelledScreen record={RECORD} turns={17} onBack={() => undefined} />);
  expect(screen.getByTestId(assaultTestid.result)).toHaveTextContent("Assault repelled");
  expect(screen.getByTestId(assaultTestid.result)).toHaveTextContent("17 turns");

  rerender(<AbandonedScreen record={RECORD} onBack={() => undefined} />);
  expect(screen.getByTestId(assaultTestid.result)).toHaveTextContent("Attempt abandoned");
});
