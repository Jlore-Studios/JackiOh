import { afterEach, expect, it } from "vitest";

import { commandActions } from "../routes/commandEntries.ts";
import { setAssaultOpenForTests } from "./config.ts";

afterEach(() => { setAssaultOpenForTests(true); });

it("the command menu lists Assault since the release, and not while a test closes ASSAULT_OPEN", () => {
  expect(commandActions()).toContainEqual({
    id: "action-assault",
    kind: "action",
    label: "Infinite Assault",
    path: "/assault",
  });
  setAssaultOpenForTests(false);
  expect(commandActions().some((entry) => entry.id === "action-assault")).toBe(false);
});
