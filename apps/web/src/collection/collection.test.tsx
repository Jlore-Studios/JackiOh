// The collection screen's panels (#1307, SPEC §14), from fixture responses: the completion counts
// and locked cards of the owned-cards panel, the wallet, and the tracks' levels, XP bars, rewards
// and locks. The server's answers are the fixtures; nothing here asks it anything.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLOBAL_PACK_NAME, SET_TRACK_LEVELS, SET_TRACK_UNLOCK_LEVEL } from "@jackioh/server-config";
import { SHIPPED_SETS, newestShippedSet, type CardDef } from "@jackioh/shared";

import { INSPECT_CLOSE, INSPECT_DETAIL, closeInspect } from "../cards/index.ts";
import { packsInfo } from "../packs/fixtures.ts";
import { packName } from "../packs/packs.ts";
import { cosmeticPacksInfo, walletInfo } from "../shop/fixtures.ts";
import OwnedCards from "./OwnedCards.tsx";
import TracksPanel from "./TracksPanel.tsx";
import WalletPanel from "./WalletPanel.tsx";
import { COLLECTED_DEFS, collectionTestid as T, setCompletions } from "./collection.ts";
import { collectionOwning, fullCollection, track, tracksOf } from "./fixtures.ts";

afterEach(() => {
  closeInspect();
  cleanup();
});

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`${id} is not in the catalog`);
  return def;
}

/** Two Core cards, one Classic card and a Core token the completion count must skip. */
const DEFS = [defOf("core-001"), defOf("core-002"), defOf("classic-001"), defOf("core-051-1")];

describe("#1307 setCompletions", () => {
  it("counts each shipped set's non-token cards and those owned", () => {
    const sets = setCompletions(collectionOwning(["core-001", "core-051-1"]), DEFS);

    expect(sets.map((entry) => entry.set)).toEqual([...SHIPPED_SETS]);
    expect(sets.map((entry) => [entry.owned, entry.total])).toEqual([
      [1, 2],
      [0, 1],
      [0, 0],
      [0, 0],
      [0, 0],
    ]);
    expect(sets[0]?.cards.map((card) => [card.def.id, card.owned])).toEqual([
      ["core-001", true],
      ["core-002", false],
    ]);
  });

  it("a quantity of 0 is not owned", () => {
    const sets = setCompletions({ catalogVersion: "test", entries: [{ cardId: "core-001", quantity: 0 }] }, DEFS);

    expect(sets[0]?.owned).toBe(0);
  });

  it("the launch grant owns every collected card", () => {
    const sets = setCompletions(fullCollection());

    expect(sets.every((entry) => entry.owned === entry.total)).toBe(true);
    for (const entry of sets) {
      const inCatalog = Object.values(CATALOG).filter((def) => def.set === entry.set && !def.token);
      expect(entry.total).toBe(inCatalog.length);
    }
    expect(sets.reduce((sum, entry) => sum + entry.total, 0)).toBe(COLLECTED_DEFS.length);
  });
});

describe("#1307 OwnedCards", () => {
  it("shows each set's completion count", () => {
    render(<OwnedCards collection={collectionOwning(["core-001"])} defs={DEFS} />);

    expect(screen.getByTestId(T.completion("Core"))).toHaveTextContent("1 of 2");
    expect(screen.getByTestId(T.completion("Classic"))).toHaveTextContent("0 of 1");
    expect(screen.queryByTestId(T.card("core-001"))).toBeNull();
  });

  it("draws an unowned card as a locked silhouette", () => {
    render(<OwnedCards collection={collectionOwning(["core-001"])} defs={DEFS} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));

    expect(screen.getByTestId(T.setToggle("Core"))).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByTestId(T.card("core-001"))).toHaveAttribute("data-owned", "true");
    expect(screen.getByTestId(T.card("core-001"))).not.toHaveTextContent("Locked");
    expect(screen.getByTestId(T.card("core-002"))).toHaveAttribute("data-owned", "false");
    expect(screen.getByTestId(T.card("core-002"))).toHaveTextContent("Locked");
    expect(screen.queryByTestId(T.card("core-051-1"))).toBeNull();
  });

  it("opens one set at a time", () => {
    render(<OwnedCards collection={collectionOwning([])} defs={DEFS} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.setToggle("Classic")));

    expect(screen.queryByTestId(T.card("core-001"))).toBeNull();
    expect(screen.getByTestId(T.card("classic-001"))).toBeInTheDocument();
  });

  it("a locked card still opens its inspect view", () => {
    render(<OwnedCards collection={collectionOwning(["core-001"])} defs={DEFS} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card("core-002")));

    const detail = screen.getByTestId(INSPECT_DETAIL);
    expect(detail).toHaveAttribute("data-card", "core-002");
    expect(detail).toHaveTextContent("Not owned yet");

    fireEvent.click(screen.getByTestId(INSPECT_CLOSE));
    expect(screen.queryByTestId(INSPECT_DETAIL)).toBeNull();
  });

  it("an owned card's inspect view says so", () => {
    render(<OwnedCards collection={collectionOwning(["core-001"])} defs={DEFS} />);
    fireEvent.click(screen.getByTestId(T.setToggle("Core")));
    fireEvent.click(screen.getByTestId(T.card("core-001")));

    expect(screen.getByTestId(INSPECT_DETAIL)).toHaveTextContent("Owned");
  });
});

describe("#1307 WalletPanel", () => {
  it("shows the balance and every unopened pack", () => {
    render(<WalletPanel wallet={walletInfo()} packs={packsInfo()} cosmeticPacks={cosmeticPacksInfo()} />);

    expect(screen.getByTestId(T.jlore)).toHaveTextContent("37 Jlore");
    expect(screen.getByTestId(T.packs(GLOBAL_PACK_NAME))).toHaveTextContent("Global pack: 1 unopened");
    expect(screen.getByTestId(T.packs("Core"))).toHaveTextContent("Core pack: 2 unopened");
    expect(screen.getByTestId(T.cosmeticPacks)).toHaveTextContent("Cosmetic packs: 2 unopened");
    expect(screen.queryByTestId(T.noPacks)).toBeNull();
  });

  it("says when no pack is held", () => {
    render(
      <WalletPanel
        wallet={walletInfo()}
        packs={{ ...packsInfo(), unopened: [] }}
        cosmeticPacks={{ ...cosmeticPacksInfo(), unopened: 0 }}
      />,
    );

    expect(screen.getByTestId(T.noPacks)).toHaveTextContent("No unopened packs.");
    expect(screen.queryByTestId(T.cosmeticPacks)).toBeNull();
  });
});

describe("#1307 TracksPanel", () => {
  it("shows an unlocked track's level, XP bar and next rewards", () => {
    const newest = newestShippedSet();
    render(<TracksPanel tracks={tracksOf().tracks} />);

    expect(screen.getByTestId(T.track(newest))).toHaveAttribute("data-unlocked", "true");
    expect(screen.getByTestId(T.trackLevel(newest))).toHaveTextContent(`Level 7 of ${String(SET_TRACK_LEVELS)}`);
    const bar = screen.getByRole("progressbar", { name: `${newest} track XP` });
    expect(bar).toHaveAttribute("max", "700");
    expect(bar).toHaveAttribute("value", "50");
    expect(screen.getByTestId(T.trackXp(newest))).toHaveTextContent("650 XP to the next level");
    expect(screen.getByTestId(T.trackRewards(newest))).toHaveTextContent(
      `Next rewards: 25 Jlore, 2 × ${packName(newest)}, 3 × card token`,
    );
    expect(screen.queryByTestId(T.trackLock(newest))).toBeNull();
  });

  it("marks a locked track and says how it unlocks", () => {
    render(<TracksPanel tracks={tracksOf().tracks} />);

    expect(screen.getByTestId(T.track("Core"))).toHaveAttribute("data-unlocked", "false");
    expect(screen.getByTestId(T.trackLock("Core"))).toHaveTextContent(
      `Locked: unlocks once the ${newestShippedSet()} track reaches level ${String(SET_TRACK_UNLOCK_LEVEL)}.`,
    );
    expect(screen.getByTestId(T.trackRewards("Core"))).toHaveTextContent("Next rewards: 40 Jlore");
    expect(screen.getAllByRole("listitem").map((row) => row.getAttribute("data-testid"))).toEqual(
      SHIPPED_SETS.map((set) => T.track(set)),
    );
  });

  it("a track on its last level reads no next level", () => {
    render(
      <TracksPanel
        tracks={[
          track({
            level: SET_TRACK_LEVELS,
            levelXp: null,
            xpToNext: null,
            nextRewards: null,
            rewardsClaimed: SET_TRACK_LEVELS,
            unlocked: true,
          }),
        ]}
      />,
    );

    expect(screen.getByTestId(T.trackXp("Core"))).toHaveTextContent("Last level reached.");
    expect(screen.getByTestId(T.trackRewards("Core"))).toHaveTextContent("All rewards claimed.");
    expect(screen.queryByRole("progressbar")).toBeNull();
  });
});
