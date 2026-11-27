import { afterEach, expect, it } from "vitest";

import { setAssaultOpenForTests } from "./config.ts";
import { playCards } from "../routes/playModes.ts";

afterEach(() => { setAssaultOpenForTests(true); });

it("the assault card shows since the release, after Dungeon run, and hides while a test closes ASSAULT_OPEN", () => {
  const cards = playCards({ signedIn: false, dailyWaiting: 0 });
  const ids = cards.map((card) => card.id);
  expect(ids).toContain("assault");
  expect(ids.indexOf("assault")).toBe(ids.indexOf("dungeon") + 1);
  expect(cards.find((card) => card.id === "assault")).toEqual(expect.objectContaining({ testid: "play-assault" }));

  setAssaultOpenForTests(false);
  expect(playCards({ signedIn: false, dailyWaiting: 0 }).some((card) => card.id === "assault")).toBe(false);
});
