import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";

import { assaultTestid } from "../testids.ts";
import { CONTENT, DAY } from "../test/fixtures.ts";
import { Briefing } from "./Briefing.tsx";

afterEach(cleanup);

it("names the enemy, the doctrine line, the Commander's two cards and the new upgrades", () => {
  const level = DAY.levels[1];
  if (level === undefined) throw new Error("fixture level 2 is missing");
  render(<Briefing content={CONTENT} level={level} onFight={() => undefined} onAbandon={() => undefined} />);

  const briefing = screen.getByTestId(assaultTestid.briefing);
  expect(briefing).toHaveTextContent("Bastion Node");
  expect(briefing).toHaveTextContent("Walls, shields and Traps punish attacks.");
  expect(briefing).toHaveTextContent("Big D-fender");
  expect(briefing).toHaveTextContent("Speciesist");
  const fresh = within(briefing).getByText(/Hardened Shell/).closest("li");
  expect(fresh).toHaveAttribute("data-new", "true");
  expect(screen.getByTestId(assaultTestid.fightStart)).toHaveTextContent("Fight");
});
