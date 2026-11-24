import { afterEach, expect, it } from "vitest";

import { setAssaultOpenForTests } from "./config.ts";
import { playCards } from "../routes/playModes.ts";

afterEach(() => { setAssaultOpenForTests(false); });

it("the assault card hides while ASSAULT_OPEN is false and sits after Dungeon run when a test sets it", () => {
  expect(playCards({ signedIn: false, dailyWaiting: 0 }).some((card) => card.id === "assault")).toBe(false);

  setAssaultOpenForTests(true);
  const cards = playCards({ signedIn: false, dailyWaiting: 0 });
  const ids = cards.map((card) => card.id);
  expect(ids).toContain("assault");
  expect(ids.indexOf("assault")).toBe(ids.indexOf("dungeon") + 1);
  expect(cards.find((card) => card.id === "assault")).toEqual(expect.objectContaining({ testid: "play-assault" }));
});
