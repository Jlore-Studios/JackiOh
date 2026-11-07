import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from "react";
import { useRef } from "react";

import "./tabs.css";

export type TabItem<T extends string = string> = {
  id: T;
  label: ReactNode;
  testId?: string;
  className?: string;
  disabled?: boolean;
  panel?: ReactNode;
  panelTestId?: string;
  panelClassName?: string;
  panelAs?: "div" | "section";
};

export type TabsProps<T extends string = string> = {
  items: readonly TabItem<T>[];
  value?: T;
  onChange: (value: T) => void;
  label: string;
  testId?: string;
  idPrefix: string;
  className?: string;
  tablistClassName?: string;
  tabClassName?: string;
  panelsClassName?: string;
  panelsRef?: { current: HTMLDivElement | null } | ((instance: HTMLDivElement | null) => void) | null;
  keepMounted?: boolean;
  panelAs?: "div" | "section";
};

export function Tabs<T extends string = string>({
  items,
  value,
  onChange,
  label,
  testId,
  idPrefix,
  className,
  tablistClassName,
  tabClassName,
  panelsClassName,
  panelsRef,
  keepMounted = false,
  panelAs = "div",
}: TabsProps<T>): ReactNode {
  const tabRefs = useRef<Map<T, HTMLButtonElement>>(new Map());
  const activeItem = (value !== undefined ? items.find((item) => item.id === value) : undefined) ?? items[0];

  const onTabKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const enabled = items.filter((item) => !item.disabled);
    if (enabled.length === 0) return;
    const currentId = activeItem?.id ?? value;
    const at = enabled.findIndex((item) => item.id === currentId);
    let to: number;
    if (event.key === "ArrowRight") to = (at + 1) % enabled.length;
    else if (event.key === "ArrowLeft") to = (at - 1 + enabled.length) % enabled.length;
    else if (event.key === "Home") to = 0;
    else if (event.key === "End") to = enabled.length - 1;
    else return;
    event.preventDefault();
    const nextItem = enabled[to];
    if (nextItem !== undefined) {
      onChange(nextItem.id);
      tabRefs.current.get(nextItem.id)?.focus();
    }
  };

  const hasPanels = items.some((item) => item.panel !== undefined);

  let panelsContent: ReactNode = null;
  if (hasPanels) {
    if (keepMounted) {
      panelsContent = items
        .filter((item) => item.panel !== undefined)
        .map((item) => {
          const PanelTag = item.panelAs ?? panelAs;
          const isSelected = value !== undefined ? item.id === value : item.id === activeItem?.id;
          return (
            <PanelTag
              key={item.id}
              id={`${idPrefix}-panel-${item.id}`}
              role="tabpanel"
              aria-labelledby={`${idPrefix}-tab-${item.id}`}
              hidden={!isSelected}
              data-testid={item.panelTestId}
              className={item.panelClassName}
            >
              {item.panel}
            </PanelTag>
          );
        });
    } else if (activeItem?.panel !== undefined) {
      const PanelTag = activeItem.panelAs ?? panelAs;
      panelsContent = (
        <PanelTag
          key={activeItem.id}
          id={`${idPrefix}-panel-${activeItem.id}`}
          role="tabpanel"
          aria-labelledby={`${idPrefix}-tab-${activeItem.id}`}
          data-testid={activeItem.panelTestId}
          className={activeItem.panelClassName}
        >
          {activeItem.panel}
        </PanelTag>
      );
    }
  }

  return (
    <div className={`tabs ${hasPanels ? "tabs--has-panels" : ""} ${className ?? ""}`.trim()}>
      <div
        role="tablist"
        aria-label={label}
        data-testid={testId}
        className={`tabs__tablist ${tablistClassName ?? ""}`.trim()}
        onKeyDown={onTabKeyDown}
      >
        {items.map((item) => {
          const isSelected = value !== undefined ? item.id === value : item.id === activeItem?.id;
          return (
            <button
              key={item.id}
              ref={(node) => {
                if (node === null) tabRefs.current.delete(item.id);
                else tabRefs.current.set(item.id, node);
              }}
              type="button"
              role="tab"
              id={`${idPrefix}-tab-${item.id}`}
              aria-controls={`${idPrefix}-panel-${item.id}`}
              aria-selected={isSelected}
              tabIndex={isSelected ? 0 : -1}
              data-testid={item.testId}
              disabled={item.disabled}
              className={`tabs__tab ${tabClassName ?? ""} ${item.className ?? ""}`.trim()}
              onClick={() => {
                if (!item.disabled) onChange(item.id);
              }}
            >
              {item.label}
            </button>
          );
        })}
      </div>
      {panelsContent !== null &&
        (panelsClassName !== undefined || panelsRef !== undefined ? (
          <div className={panelsClassName} ref={panelsRef}>
            {panelsContent}
          </div>
        ) : (
          panelsContent
        ))}
    </div>
  );
}
