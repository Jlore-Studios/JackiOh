import { createRef } from "react";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Tabs, type TabItem } from "./Tabs.tsx";

afterEach(cleanup);

describe("#3222 Tabs component", () => {
  const items: TabItem[] = [
    { id: "one", label: "Tab One", testId: "tab-one", panel: <div>Panel One</div>, panelTestId: "panel-one" },
    { id: "two", label: "Tab Two", testId: "tab-two", panel: <div>Panel Two</div>, panelTestId: "panel-two" },
    { id: "three", label: "Tab Three", testId: "tab-three", panel: <div>Panel Three</div>, panelTestId: "panel-three" },
  ];

  it("#3222 renders tablist with aria-label, tabs with role='tab', aria-selected reflecting value, and roving tabindex", () => {
    render(<Tabs items={items} value="one" onChange={vi.fn()} label="Test Tabs" idPrefix="test" testId="test-tablist" />);

    const list = screen.getByRole("tablist", { name: "Test Tabs" });
    expect(list).toHaveAttribute("data-testid", "test-tablist");

    const tabs = within(list).getAllByRole("tab");
    expect(tabs).toHaveLength(3);

    expect(tabs[0]).toHaveAttribute("aria-selected", "true");
    expect(tabs[0]).toHaveAttribute("tabindex", "0");
    expect(tabs[0]).toHaveAttribute("aria-controls", "test-panel-one");

    expect(tabs[1]).toHaveAttribute("aria-selected", "false");
    expect(tabs[1]).toHaveAttribute("tabindex", "-1");
    expect(tabs[1]).toHaveAttribute("aria-controls", "test-panel-two");

    expect(tabs[2]).toHaveAttribute("aria-selected", "false");
    expect(tabs[2]).toHaveAttribute("tabindex", "-1");
  });

  it("#3222 clicking a tab invokes onChange callback with the tab's id", () => {
    const onChange = vi.fn();
    render(<Tabs items={items} value="one" onChange={onChange} label="Test Tabs" idPrefix="test" />);

    fireEvent.click(screen.getByTestId("tab-two"));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith("two");
  });

  it("#3222 ArrowRight and ArrowLeft move active tab, wrapping around boundaries, and focus the new tab", () => {
    const onChange = vi.fn();
    const { rerender } = render(<Tabs items={items} value="one" onChange={onChange} label="Test Tabs" idPrefix="test" testId="tabs" />);

    const list = screen.getByTestId("tabs");

    fireEvent.keyDown(list, { key: "ArrowRight" });
    expect(onChange).toHaveBeenCalledWith("two");

    rerender(<Tabs items={items} value="two" onChange={onChange} label="Test Tabs" idPrefix="test" testId="tabs" />);
    fireEvent.keyDown(list, { key: "ArrowRight" });
    expect(onChange).toHaveBeenCalledWith("three");

    rerender(<Tabs items={items} value="three" onChange={onChange} label="Test Tabs" idPrefix="test" testId="tabs" />);
    // Wrapping around to the first tab
    fireEvent.keyDown(list, { key: "ArrowRight" });
    expect(onChange).toHaveBeenCalledWith("one");

    rerender(<Tabs items={items} value="one" onChange={onChange} label="Test Tabs" idPrefix="test" testId="tabs" />);
    // Wrapping backward to the last tab
    fireEvent.keyDown(list, { key: "ArrowLeft" });
    expect(onChange).toHaveBeenCalledWith("three");
  });

  it("#3222 Home and End keys jump to the first and last tabs", () => {
    const onChange = vi.fn();
    const { rerender } = render(<Tabs items={items} value="two" onChange={onChange} label="Test Tabs" idPrefix="test" testId="tabs" />);

    const list = screen.getByTestId("tabs");

    fireEvent.keyDown(list, { key: "Home" });
    expect(onChange).toHaveBeenCalledWith("one");

    rerender(<Tabs items={items} value="one" onChange={onChange} label="Test Tabs" idPrefix="test" testId="tabs" />);
    fireEvent.keyDown(list, { key: "End" });
    expect(onChange).toHaveBeenCalledWith("three");
  });

  it("#3222 unhandled keys are ignored and default action is not prevented", () => {
    const onChange = vi.fn();
    render(<Tabs items={items} value="one" onChange={onChange} label="Test Tabs" idPrefix="test" testId="tabs" />);

    const list = screen.getByTestId("tabs");
    const event = new KeyboardEvent("keydown", { key: "Tab", cancelable: true });
    list.dispatchEvent(event);

    expect(onChange).not.toHaveBeenCalled();
    expect(event.defaultPrevented).toBe(false);
  });

  it("#3222 disabled tab is skipped during arrow navigation and click does not invoke onChange", () => {
    const itemsWithDisabled: TabItem[] = [
      { id: "one", label: "Tab One", testId: "tab-one" },
      { id: "two", label: "Tab Two", testId: "tab-two", disabled: true },
      { id: "three", label: "Tab Three", testId: "tab-three" },
    ];
    const onChange = vi.fn();
    render(<Tabs items={itemsWithDisabled} value="one" onChange={onChange} label="Test Tabs" idPrefix="test" testId="tabs" />);

    fireEvent.click(screen.getByTestId("tab-two"));
    expect(onChange).not.toHaveBeenCalled();

    const list = screen.getByTestId("tabs");
    fireEvent.keyDown(list, { key: "ArrowRight" });
    // Skips disabled tab "two" and selects "three"
    expect(onChange).toHaveBeenCalledWith("three");
  });

  it("#3222 keepMounted renders all panels with hidden attribute; default mounts only the active panel", () => {
    const { rerender } = render(
      <Tabs items={items} value="one" onChange={vi.fn()} label="Test Tabs" idPrefix="test" keepMounted={false} />,
    );

    expect(screen.getByTestId("panel-one")).toBeInTheDocument();
    expect(screen.queryByTestId("panel-two")).toBeNull();
    expect(screen.queryByTestId("panel-three")).toBeNull();

    rerender(<Tabs items={items} value="one" onChange={vi.fn()} label="Test Tabs" idPrefix="test" keepMounted={true} />);

    const panelOne = screen.getByTestId("panel-one");
    const panelTwo = screen.getByTestId("panel-two");
    const panelThree = screen.getByTestId("panel-three");

    expect(panelOne).toBeInTheDocument();
    expect(panelOne).not.toHaveAttribute("hidden");

    expect(panelTwo).toBeInTheDocument();
    expect(panelTwo).toHaveAttribute("hidden");

    expect(panelThree).toBeInTheDocument();
    expect(panelThree).toHaveAttribute("hidden");
  });

  it("#3222 supports panelAs, panelsClassName, and panelsRef", () => {
    const panelsRef = createRef<HTMLDivElement>();
    render(
      <Tabs
        items={items}
        value="one"
        onChange={vi.fn()}
        label="Test Tabs"
        idPrefix="test"
        panelAs="section"
        panelsClassName="my-panels"
        panelsRef={panelsRef}
      />,
    );

    const panel = screen.getByTestId("panel-one");
    expect(panel.tagName.toLowerCase()).toBe("section");
    expect(panelsRef.current).toHaveClass("my-panels");
  });
});
