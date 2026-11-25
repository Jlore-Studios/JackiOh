import { afterEach, expect, it } from "vitest";

import { commandActions } from "../routes/commandEntries.ts";
import { setAssaultOpenForTests } from "./config.ts";

afterEach(() => { setAssaultOpenForTests(false); });

it("the command menu lists Assault only while ASSAULT_OPEN is true", () => {
  expect(commandActions().some((entry) => entry.id === "action-assault")).toBe(false);
  setAssaultOpenForTests(true);
  expect(commandActions()).toContainEqual({
    id: "action-assault",
    kind: "action",
    label: "Infinite Assault",
    path: "/assault",
  });
});
