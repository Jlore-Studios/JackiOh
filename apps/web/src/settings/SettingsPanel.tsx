// The settings dialog (docs/polish/7-mobile-ux.md S8, B23–B24): a centred modal on desktop and a
// bottom sheet on phones (settings.css). `SettingsButton` renders it through a portal while open;
// tests and integration can also render it directly.
//
// Its sections are tabs (issue #128): Gameplay, Visuals and Audio, each a `tabpanel` under one
// `tablist`. Every panel stays in the document and only the chosen one is shown, so a control keeps
// its state and a test can reach any of them by test id. A section with no switch and no slot has no
// tab. The dialog opens on the tab the player used last on this device (`tabs.ts`), or the one a
// caller names.
//
// Every switch reads the store and writes straight back to it, so there is no local draft and no
// "save": a change applies at once, including to a board rendered behind the scrim.

import {
  useEffect,
  useId,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactElement,
} from "react";

import { SETTINGS_SLOTS, type SettingsSectionId, type SettingsSlot } from "./slots.ts";
import {
  DEFAULT_SETTINGS,
  resetSettings,
  useSettings,
  writeSettings,
  type SettingKey,
  type Settings,
} from "./store.ts";
import { readRememberedTab, rememberTab } from "./tabs.ts";
import { syncPlayerStats } from "../stats/sync.ts";
import "./settings.css";

export type SettingsPanelProps = {
  onClose: () => void;
  /** The controls other tasks mount. Defaults to `SETTINGS_SLOTS`. */
  slots?: readonly SettingsSlot[];
  /** The tab to open on, ahead of the one the player used last. Ignored if the dialog has no such tab. */
  initialTab?: SettingsSectionId;
};

type SectionSpec = {
  id: SettingsSectionId;
  title: string;
  /** The built-in switches, rendered before the section's slots. */
  controls: readonly SettingKey[];
};

const SECTIONS: readonly SectionSpec[] = [
  {
    id: "gameplay",
    title: "Gameplay",
    controls: ["dragToPlay", "confirmEndTurn", "autoEndTurn", "hoverPreviews", "publicStats"],
  },
  { id: "visuals", title: "Visuals", controls: ["reduceMotion"] },
  { id: "audio", title: "Audio", controls: [] },
  { id: "account", title: "Account", controls: [] },
];

/** The label is the switch's whole accessible name; the hint is its description. */
const CONTROLS: Readonly<Record<SettingKey, { label: string; hint: string }>> = {
  dragToPlay: {
    label: "Drag to play",
    hint: "Drag a card onto the board, or a unit onto an enemy. Off: tap to select, then choose.",
  },
  confirmEndTurn: {
    label: "Confirm end turn",
    hint: "Ask again before ending the turn while you can still play or attack.",
  },
  autoEndTurn: {
    label: "End turn automatically",
    hint: "Ends the turn by itself when there is nothing left to play or attack with. Off: press End turn yourself.",
  },
  // One switch for both hover behaviours: task 7's hand lift and task 6's enlarged preview, which
  // opens only while this is on (cards/inspect/useInspectTrigger.tsx).
  hoverPreviews: {
    label: "Hover previews",
    hint: "Lift a card in your hand, and show any card enlarged, when the mouse rests on it.",
  },
  reduceMotion: {
    label: "Reduce motion",
    hint: "Turn animations off, whatever your system setting says.",
  },
  publicStats: {
    label: "Public player statistics",
    hint: "Share your games, win rate and favourite cards on the public stats page. Off: visible only to you when signed in.",
  },
};

const FOCUSABLE =
  'button:not([disabled]):not([tabindex="-1"]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])';

/** What "Reset this tab" does: the tab's own switches back to their defaults, and its slots' stores. */
function resetSection(section: SectionSpec, slots: readonly SettingsSlot[]): void {
  const patch: Partial<Settings> = {};
  for (const setting of section.controls) patch[setting] = DEFAULT_SETTINGS[setting];
  writeSettings(patch);
  for (const slot of slots) if (slot.section === section.id) slot.reset?.();
}

function SettingSwitch({ setting, checked }: { setting: SettingKey; checked: boolean }): ReactElement {
  const hintId = useId();
  const { label, hint } = CONTROLS[setting];
  return (
    <div className="settings-row">
      <label className="settings-control">
        <span className="settings-label">{label}</span>
        <input
          type="checkbox"
          role="switch"
          className="settings-switch"
          data-testid={`setting-${setting}`}
          checked={checked}
          aria-describedby={hintId}
          onChange={(event) => {
            const patch: Partial<Settings> = {};
            patch[setting] = event.currentTarget.checked;
            writeSettings(patch);
            if (setting === "publicStats") {
              void syncPlayerStats();
            }
          }}
        />
      </label>
      <p className="settings-hint" id={hintId}>
        {hint}
      </p>
    </div>
  );
}

export default function SettingsPanel({
  onClose,
  slots = SETTINGS_SLOTS,
  initialTab,
}: SettingsPanelProps): ReactElement {
  const settings = useSettings();
  const panelRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const tabRefs = useRef(new Map<SettingsSectionId, HTMLButtonElement>());
  const base = useId();

  // A section with nothing in it has no tab, so the dialog never shows an empty one.
  const sections = SECTIONS.filter(
    (section) => section.controls.length > 0 || slots.some((slot) => slot.section === section.id),
  );
  const ids = sections.map((section) => section.id);
  const [chosen, setChosen] = useState<SettingsSectionId | null>(
    () => ids.find((id) => id === initialTab) ?? readRememberedTab(ids),
  );
  // Slots can change under an open dialog; a tab that has gone falls back to the first one.
  const active: SettingsSectionId | undefined = ids.find((id) => id === chosen) ?? ids[0];
  const activeSection = sections.find((section) => section.id === active);
  // The Account tab has no switch and no store to put back: its status is the account's.
  const canReset =
    activeSection !== undefined &&
    (activeSection.controls.length > 0 ||
      slots.some((slot) => slot.section === activeSection.id && slot.reset !== undefined));

  const choose = (id: SettingsSectionId): void => {
    setChosen(id);
    rememberTab(id);
  };

  // Focus the first switch of the open tab on open (or the tab itself when it has none), so a
  // keyboard or screen-reader user lands inside the dialog.
  useEffect(() => {
    const panel = panelRef.current;
    if (panel === null) return;
    const first =
      panel.querySelector<HTMLElement>('[role="tabpanel"]:not([hidden]) input[role="switch"]') ??
      panel.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]') ??
      panel;
    first.focus({ preventScroll: true });
  }, []);

  // A new tab starts at its top.
  useEffect(() => {
    if (bodyRef.current !== null) bodyRef.current.scrollTop = 0;
  }, [active]);

  const onTabKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const at = active === undefined ? -1 : ids.indexOf(active);
    let to: number;
    if (event.key === "ArrowRight") to = (at + 1) % ids.length;
    else if (event.key === "ArrowLeft") to = (at - 1 + ids.length) % ids.length;
    else if (event.key === "Home") to = 0;
    else if (event.key === "End") to = ids.length - 1;
    else return;
    event.preventDefault();
    const id = ids[to];
    if (id === undefined) return;
    choose(id);
    tabRefs.current.get(id)?.focus();
  };

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    if (event.key === "Escape") {
      // Stopped so the board's own Escape (cancel a selection or a drag) does not also fire.
      event.stopPropagation();
      onClose();
      return;
    }
    if (event.key !== "Tab") return;
    // aria-modal: Tab and Shift+Tab cycle inside the dialog instead of walking off into the board.
    // A hidden panel's controls are not reachable, so they are not the ends.
    const panel = panelRef.current;
    if (panel === null) return;
    const items = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
      (item) => item.closest("[hidden]") === null,
    );
    const first = items[0];
    const last = items[items.length - 1];
    if (first === undefined || last === undefined) return;
    const active = document.activeElement;
    if (event.shiftKey && (active === first || active === panel)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && active === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <div
      className="settings-scrim"
      data-testid="settings-scrim"
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        ref={panelRef}
        className="settings-panel"
        data-testid="settings-panel"
        role="dialog"
        aria-modal="true"
        aria-label="Settings"
        tabIndex={-1}
        onKeyDown={onKeyDown}
      >
        <header className="settings-head">
          <h1 className="settings-title">Settings</h1>
          <button
            type="button"
            className="settings-close"
            data-testid="settings-close"
            aria-label="Close settings"
            onClick={onClose}
          >
            <span aria-hidden="true">✕</span>
          </button>
        </header>
        <div
          className="settings-tabs"
          role="tablist"
          aria-label="Settings sections"
          data-testid="settings-tablist"
          onKeyDown={onTabKeyDown}
        >
          {sections.map((section) => (
            <button
              key={section.id}
              ref={(node) => {
                if (node === null) tabRefs.current.delete(section.id);
                else tabRefs.current.set(section.id, node);
              }}
              type="button"
              role="tab"
              id={`${base}-tab-${section.id}`}
              className="settings-tab"
              data-testid={`settings-tab-${section.id}`}
              aria-selected={section.id === active}
              aria-controls={`${base}-panel-${section.id}`}
              tabIndex={section.id === active ? 0 : -1}
              onClick={() => {
                choose(section.id);
              }}
            >
              {section.title}
            </button>
          ))}
        </div>
        <div className="settings-body" ref={bodyRef}>
          {sections.map((section) => {
            const sectionSlots = slots.filter((slot) => slot.section === section.id);
            return (
              <section
                key={section.id}
                id={`${base}-panel-${section.id}`}
                className="settings-section"
                role="tabpanel"
                aria-labelledby={`${base}-tab-${section.id}`}
                hidden={section.id !== active}
                data-testid={`settings-section-${section.id}`}
              >
                <h2 className="settings-section-title">{section.title}</h2>
                {section.controls.map((setting) => (
                  <SettingSwitch key={setting} setting={setting} checked={settings[setting]} />
                ))}
                {sectionSlots.map((slot) => (
                  <div key={slot.id} className="settings-slot" data-settings-slot={slot.id}>
                    {slot.render()}
                  </div>
                ))}
              </section>
            );
          })}
        </div>
        <footer className="settings-foot">
          <button
            type="button"
            className="settings-reset settings-reset--tab"
            data-testid="settings-reset-tab"
            disabled={!canReset}
            onClick={() => {
              if (activeSection !== undefined) resetSection(activeSection, slots);
            }}
          >
            Reset this tab
          </button>
          <button
            type="button"
            className="settings-reset"
            data-testid="settings-reset"
            onClick={() => {
              resetSettings();
              // Every store the panel shows goes back to its defaults, not only this module's.
              for (const slot of slots) slot.reset?.();
            }}
          >
            Reset all
          </button>
        </footer>
      </div>
    </div>
  );
}
