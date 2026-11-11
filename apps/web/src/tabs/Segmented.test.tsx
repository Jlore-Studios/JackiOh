import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Segmented, type SegmentOption } from "./Segmented.tsx";

afterEach(cleanup);

describe("#3222 Segmented component", () => {
  const options: SegmentOption[] = [
    { value: "first", label: "First Option", testId: "opt-first" },
    { value: "second", label: "Second Option", testId: "opt-second" },
    { value: "third", label: "Third Option", testId: "opt-third" },
  ];

  it("#3222 renders group with aria-label, role='group', and buttons with aria-pressed reflecting value", () => {
    render(<Segmented options={options} value="first" onChange={vi.fn()} label="Test Mode" testId="test-group" />);

    const group = screen.getByRole("group", { name: "Test Mode" });
    expect(group).toHaveAttribute("data-testid", "test-group");

    const buttons = within(group).getAllByRole("button");
    expect(buttons).toHaveLength(3);

    expect(buttons[0]).toHaveAttribute("aria-pressed", "true");
    expect(buttons[1]).toHaveAttribute("aria-pressed", "false");
    expect(buttons[2]).toHaveAttribute("aria-pressed", "false");
  });

  it("#3222 selected segment does not use button-primary class", () => {
    render(<Segmented options={options} value="first" onChange={vi.fn()} label="Test Mode" />);

    const first = screen.getByTestId("opt-first");
    expect(first).not.toHaveClass("button-primary");
    expect(first).toHaveClass("segmented__item--selected");
  });

  it("#3222 clicking a segment invokes onChange callback with the option value", () => {
    const onChange = vi.fn();
    render(<Segmented options={options} value="first" onChange={onChange} label="Test Mode" />);

    fireEvent.click(screen.getByTestId("opt-second"));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith("second");
  });

  it("#3222 disabled option cannot be clicked and carries disabled attribute", () => {
    const optionsWithDisabled: SegmentOption[] = [
      { value: "first", label: "First", testId: "opt-first" },
      { value: "second", label: "Second", testId: "opt-second", disabled: true },
      { value: "third", label: "Third", testId: "opt-third" },
    ];
    const onChange = vi.fn();
    render(<Segmented options={optionsWithDisabled} value="first" onChange={onChange} label="Test Mode" />);

    const second = screen.getByTestId("opt-second");
    expect(second).toBeDisabled();

    fireEvent.click(second);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("#3222 entire component disabled prop disables all options", () => {
    const onChange = vi.fn();
    render(<Segmented options={options} value="first" onChange={onChange} label="Test Mode" disabled />);

    for (const opt of options) {
      const btn = screen.getByTestId(opt.testId!);
      expect(btn).toBeDisabled();
      fireEvent.click(btn);
    }
    expect(onChange).not.toHaveBeenCalled();
  });

  it("#3222 ArrowRight and ArrowLeft keys navigate between segments and focus the selected option", () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <Segmented options={options} value="first" onChange={onChange} label="Test Mode" testId="group" />,
    );

    const group = screen.getByTestId("group");

    fireEvent.keyDown(group, { key: "ArrowRight" });
    expect(onChange).toHaveBeenCalledWith("second");

    rerender(<Segmented options={options} value="second" onChange={onChange} label="Test Mode" testId="group" />);
    fireEvent.keyDown(group, { key: "ArrowRight" });
    expect(onChange).toHaveBeenCalledWith("third");

    rerender(<Segmented options={options} value="third" onChange={onChange} label="Test Mode" testId="group" />);
    // Wraps around to first
    fireEvent.keyDown(group, { key: "ArrowRight" });
    expect(onChange).toHaveBeenCalledWith("first");

    rerender(<Segmented options={options} value="first" onChange={onChange} label="Test Mode" testId="group" />);
    // Wraps backward to third
    fireEvent.keyDown(group, { key: "ArrowLeft" });
    expect(onChange).toHaveBeenCalledWith("third");
  });

  it("#3222 Home and End keys jump to the first and last options", () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <Segmented options={options} value="second" onChange={onChange} label="Test Mode" testId="group" />,
    );

    const group = screen.getByTestId("group");

    fireEvent.keyDown(group, { key: "Home" });
    expect(onChange).toHaveBeenCalledWith("first");

    rerender(<Segmented options={options} value="first" onChange={onChange} label="Test Mode" testId="group" />);
    fireEvent.keyDown(group, { key: "End" });
    expect(onChange).toHaveBeenCalledWith("third");
  });
});
