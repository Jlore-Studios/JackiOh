// B23 (DR14 part E, #2706): a side's relics in a tray beside its hero, one icon each in its tier's
// color, and a button, reachable by keyboard and named for a screen reader, that opens their names,
// tiers, levels and texts.

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import type { RunPassiveDef } from "@jackioh/run";

import { ArtifactTray, type TrayItem } from "./ArtifactTray.tsx";
import { fightTestid } from "./testids.ts";

const PASSIVES = [
  { id: "lucky-coin", name: "Lucky Coin", kind: "economic", text: "Gain 5 more Coins after each fight.", tier: "bronze", starter: true, unobtainable: false },
  { id: "war-drum", name: "War Drum", kind: "combat", text: "Your units have +1 Attack.", tier: "diamond", starter: false, unobtainable: false },
  { id: "blessing-of-dawn", name: "Blessing of Dawn", kind: "blessing", text: "Heal 3 after each fight.", tier: null, starter: false, unobtainable: false },
] as unknown as RunPassiveDef[];

const ITEMS: TrayItem[] = [
  { id: "lucky-coin", level: 1 },
  { id: "war-drum", level: 2 },
  { id: "blessing-of-dawn", level: null },
];

afterEach(() => {
  cleanup();
});

describe("B23 the relic tray", () => {
  it("B23 one icon per relic or blessing, each with its tier", () => {
    render(<ArtifactTray side="own" items={ITEMS} passives={PASSIVES} />);
    const tray = screen.getByTestId(fightTestid.tray("own"));
    expect(within(tray).getByTestId(fightTestid.trayIcon("own", "lucky-coin"))).toHaveAttribute("data-tier", "bronze");
    expect(within(tray).getByTestId(fightTestid.trayIcon("own", "war-drum"))).toHaveAttribute("data-tier", "diamond");
    expect(within(tray).getByTestId(fightTestid.trayIcon("own", "blessing-of-dawn"))).toHaveAttribute("data-tier", "none");
    expect(tray.querySelectorAll(".artifact-icon")).toHaveLength(3);
  });

  it("B23 a button, named for a screen reader, opens each one's name, tier, level and text", () => {
    render(<ArtifactTray side="own" items={ITEMS} passives={PASSIVES} />);
    const button = screen.getByRole("button", { name: "Your relics and blessings: 3" });
    expect(button).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByTestId(fightTestid.trayList("own"))).toBeNull();

    fireEvent.click(button);
    expect(button).toHaveAttribute("aria-expanded", "true");
    const list = screen.getByTestId(fightTestid.trayList("own"));
    expect(button).toHaveAttribute("aria-controls", list.id);
    const coin = within(list).getByTestId(fightTestid.trayItem("own", "lucky-coin"));
    expect(coin).toHaveTextContent("Lucky Coin");
    expect(coin).toHaveTextContent("Bronze");
    expect(coin).toHaveTextContent("Level 1");
    expect(coin).toHaveTextContent("Gain 5 more Coins after each fight.");
    const drum = within(list).getByTestId(fightTestid.trayItem("own", "war-drum"));
    expect(drum).toHaveTextContent("Diamond");
    expect(drum).toHaveTextContent("Level 2");
    const blessing = within(list).getByTestId(fightTestid.trayItem("own", "blessing-of-dawn"));
    expect(blessing).toHaveTextContent("Blessing of Dawn");
    expect(blessing).toHaveTextContent("Heal 3 after each fight.");
    expect(blessing).not.toHaveTextContent("Level");
  });

  it("B23 the enemy's tray names its relics, and one the content does not know shows its id", () => {
    render(<ArtifactTray side="enemy" items={[{ id: "war-drum", level: null }, { id: "looming-presence", level: null }]} passives={PASSIVES} />);
    fireEvent.click(screen.getByRole("button", { name: "The enemy's relics: 2" }));
    const list = screen.getByTestId(fightTestid.trayList("enemy"));
    expect(within(list).getByTestId(fightTestid.trayItem("enemy", "war-drum"))).toHaveTextContent("War Drum");
    expect(within(list).getByTestId(fightTestid.trayItem("enemy", "looming-presence"))).toHaveTextContent("looming-presence");
  });

  it("B23 Escape closes the list and gives the focus back to the button; a press outside closes it too", () => {
    render(<ArtifactTray side="own" items={ITEMS} passives={PASSIVES} />);
    const button = screen.getByTestId(fightTestid.trayOpen("own"));
    button.focus();
    fireEvent.click(button);
    fireEvent.keyDown(screen.getByTestId(fightTestid.trayList("own")), { key: "Escape" });
    expect(screen.queryByTestId(fightTestid.trayList("own"))).toBeNull();
    expect(button).toHaveAttribute("aria-expanded", "false");
    expect(button).toHaveFocus();

    fireEvent.click(button);
    expect(screen.getByTestId(fightTestid.trayList("own"))).toBeInTheDocument();
    fireEvent.pointerDown(document.body);
    expect(screen.queryByTestId(fightTestid.trayList("own"))).toBeNull();
  });

  it("B23 with no relic or blessing, nothing renders", () => {
    const { container } = render(<ArtifactTray side="enemy" items={[]} passives={PASSIVES} />);
    expect(container).toBeEmptyDOMElement();
  });
});
