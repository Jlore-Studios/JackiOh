import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from "react";
import { useRef } from "react";

import "./tabs.css";

export type SegmentOption<T extends string = string> = {
  value: T;
  label: ReactNode;
  testId?: string;
  className?: string;
  disabled?: boolean;
  title?: string;
};

export type SegmentedProps<T extends string = string> = {
  options: readonly SegmentOption<T>[];
  value: T;
  onChange: (value: T) => void;
  label: string;
  testId?: string;
  className?: string;
  itemClassName?: string;
  disabled?: boolean;
};

export function Segmented<T extends string = string>({
  options,
  value,
  onChange,
  label,
  testId,
  className,
  itemClassName,
  disabled = false,
}: SegmentedProps<T>): ReactNode {
  const btnRefs = useRef<Map<T, HTMLButtonElement>>(new Map());

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const enabled = options.filter((opt) => !(disabled || opt.disabled));
    if (enabled.length === 0) return;
    const at = enabled.findIndex((opt) => opt.value === value);
    let to: number;
    if (event.key === "ArrowRight") to = (at + 1) % enabled.length;
    else if (event.key === "ArrowLeft") to = (at - 1 + enabled.length) % enabled.length;
    else if (event.key === "Home") to = 0;
    else if (event.key === "End") to = enabled.length - 1;
    else return;
    event.preventDefault();
    const nextOption = enabled[to];
    if (nextOption !== undefined) {
      onChange(nextOption.value);
      btnRefs.current.get(nextOption.value)?.focus();
    }
  };

  return (
    <div
      role="group"
      aria-label={label}
      data-testid={testId}
      className={`segmented ${className ?? ""}`.trim()}
      onKeyDown={onKeyDown}
    >
      {options.map((option) => {
        const isSelected = option.value === value;
        const isDisabled = disabled || option.disabled;
        return (
          <button
            key={option.value}
            ref={(node) => {
              if (node === null) btnRefs.current.delete(option.value);
              else btnRefs.current.set(option.value, node);
            }}
            type="button"
            role="button"
            aria-pressed={isSelected}
            data-testid={option.testId}
            title={option.title}
            disabled={isDisabled}
            className={`segmented__item ${isSelected ? "segmented__item--selected" : ""} ${itemClassName ?? ""} ${option.className ?? ""}`.trim()}
            onClick={() => {
              if (!isDisabled) onChange(option.value);
            }}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
