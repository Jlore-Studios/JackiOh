// R3335001: the Fleet track panel draws `GET /api/tracks`' `fleet` as it comes: its level out of FLEET_TRACK_LEVELS,
// the level's XP as the economy's bar at rest, today's XP out of FLEET_TRACK_XP_DAILY_MAX and the next rewards as
// their looks, a Starship reward as a Starship.

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { FLEET_TRACK_LEVELS, FLEET_TRACK_XP_DAILY_MAX } from "@jackioh/server-config";

import type { FleetTrackView } from "../wire/generated/FleetTrackView.ts";
import { collectionTestid as T } from "./collection.ts";
import FleetTrackPanel from "./FleetTrackPanel.tsx";

afterEach(cleanup);

const FLEET: FleetTrackView = {
  level: 7,
  xp: 5200,
  levelXp: 800,
  xpToNext: 300,
  xpToday: 450,
  nextRewards: ["card-back-stinkyscape", "starship-livery-rust", "starship-trail-comet"],
};

describe("R3335001 the Fleet track panel", () => {
  it("R3335001 level, today's XP and next rewards", () => {
    render(<FleetTrackPanel fleet={FLEET} />);
    const panel = screen.getByTestId(T.fleet);

    expect(within(panel).getByTestId(T.fleetLevel)).toHaveTextContent(`Level 7 of ${String(FLEET_TRACK_LEVELS)}`);
    // The level's bar: levelXp − xpToNext of levelXp, at rest, as the set tracks' is.
    const bar = within(panel).getByRole("progressbar", { name: "Fleet track XP" });
    expect(bar).toHaveAttribute("max", "800");
    expect(bar).toHaveAttribute("value", "500");
    expect(bar.closest(".fill-bar")).toHaveAttribute("data-state", "resting");
    expect(within(panel).getByTestId(T.fleetXp)).toHaveTextContent("300 XP to the next level");
    expect(within(panel).getByTestId(T.fleetToday)).toHaveTextContent(`Today: 450 of ${FLEET_TRACK_XP_DAILY_MAX.toLocaleString("en-US")} Fleet XP`);

    // The next rewards, nearest first, each its look and name: a card back as a back, a Starship one as a Starship.
    const rewards = within(within(panel).getByTestId(T.fleetRewards)).getAllByRole("listitem");
    expect(rewards.map((reward) => reward.getAttribute("data-testid"))).toEqual(FLEET.nextRewards.map((id) => T.fleetReward(id)));
    const back = screen.getByTestId(T.fleetReward("card-back-stinkyscape"));
    expect(back).toHaveTextContent("Stinkyscape");
    expect(back).toHaveTextContent("Fleet track level 8");
    expect(back.querySelector('.cf-back[data-card-back="card-back-stinkyscape"]')).not.toBeNull();
    const livery = screen.getByTestId(T.fleetReward("starship-livery-rust"));
    expect(livery).toHaveTextContent("Rust bucket");
    expect(livery.querySelector("img.collection-starship")).toHaveAttribute("data-livery", "starship-livery-rust");
    expect(screen.getByTestId(T.fleetReward("starship-trail-comet")).querySelector("img.collection-starship")).toHaveAttribute(
      "data-trail",
      "starship-trail-comet",
    );
  });

  it("R3335001 the last level draws no bar, and a claimed track no rewards; an id this client does not know shows nothing", () => {
    render(<FleetTrackPanel fleet={{ ...FLEET, level: FLEET_TRACK_LEVELS, levelXp: null, xpToNext: null, xpToday: 0, nextRewards: [] }} />);
    expect(screen.queryByRole("progressbar")).toBeNull();
    expect(screen.getByTestId(T.fleetXp)).toHaveTextContent("Last level reached.");
    expect(screen.getByTestId(T.fleetRewards)).toHaveTextContent("All rewards claimed.");
    cleanup();

    render(<FleetTrackPanel fleet={{ ...FLEET, nextRewards: ["starship-hull-from-a-newer-client", "starship-hull-scout"] }} />);
    expect(screen.queryByTestId(T.fleetReward("starship-hull-from-a-newer-client"))).toBeNull();
    expect(screen.getByTestId(T.fleetReward("starship-hull-scout"))).toHaveTextContent("Scout");
  });
});
