// Settings dialog: desktop modal, phone bottom sheet (S8, B23–B24). Hidden tab panels preserve
// control state and test access; settings write through immediately without a draft.
// R3221000: drawn through the one Dialog as a sheet, whose keys (Escape, Tab, Back) are its own.

import { useEffect, useId, useRef, useState, type ReactElement } from "react";

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
import { retrySettingsSync, useSettingsSyncState } from "./accountSync.ts";
import { TryAgainButton } from "../auth/AccountError.tsx";
import { Dialog } from "../game/Dialog.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { Tabs } from "../tabs/index.ts";
import "./settings.css";

export type SettingsPanelProps = {
  onClose: () => void;
  /** The controls other tasks mount. */
  slots?: readonly SettingsSlot[];
  /** Opens this tab ahead of the remembered one when it exists. */
  initialTab?: SettingsSectionId;
};

type SectionSpec = {
  id: SettingsSectionId;
  title: string;
  controls: readonly SettingKey[];
};

const SECTIONS: readonly SectionSpec[] = [
  {
    id: "gameplay",
    title: "Gameplay",
    controls: ["dragToPlay", "confirmEndTurn", "autoEndTurn", "hoverPreviews", "lethalHint", "attackOutcomes"],
  },
  { id: "visuals", title: "Visuals", controls: ["reduceMotion"] },
  { id: "audio", title: "Audio", controls: ["muteOpponentEmotes", "muteSpectatorEmotes"] },
];

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
  // One switch controls both hover behaviors.
  hoverPreviews: {
    label: "Hover previews",
    hint: "Lift a card in your hand, and show any card enlarged, when the mouse rests on it.",
  },
  // R2291000: the teaching aid of every game against the practice AI (R2102000).
  lethalHint: {
    label: "Lethal hint",
    hint: "Against the practice AI, light up the enemy hero and End turn when you can win this turn.",
  },
  // R3477000: the engine's forecast on each target of an aimed attack; it reads the face-up board alone.
  attackOutcomes: {
    label: "Show attack outcomes",
    hint: "While you aim an attack, show the damage each target takes, the damage your Unit takes back and whether either is destroyed. A face-down trap can still change it.",
  },
  reduceMotion: {
    label: "Reduce motion",
    hint: "Turn animations off, whatever your system setting says.",
  },
  muteOpponentEmotes: {
    label: "Mute opponent emotes",
    hint: "Never show or hear an opponent's emotes, in every match.",
  },
  // R2990004: the sideline strip's Mute, on from the start of every match.
  muteSpectatorEmotes: {
    label: "Mute spectator emotes",
    hint: "Never show or hear spectators' emotes, in every match.",
  },
};

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
        <span className="settings-label">{t(label)}</span>
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
          }}
        />
      </label>
      <p className="settings-hint" id={hintId}>
        {t(hint)}
      </p>
    </div>
  );
}

export default function SettingsPanel({
  onClose,
  slots = SETTINGS_SLOTS,
  initialTab,
}: SettingsPanelProps): ReactElement {
  useLocale();
  const settings = useSettings();
  const sync = useSettingsSyncState();
  const panelRef = useRef<HTMLElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const base = useId();

  const sections = SECTIONS.filter(
    (section) => section.controls.length > 0 || slots.some((slot) => slot.section === section.id),
  );
  const ids = sections.map((section) => section.id);
  const [chosen, setChosen] = useState<SettingsSectionId | null>(
    () => ids.find((id) => id === initialTab) ?? readRememberedTab(ids),
  );
  // A removed active tab falls back to the first.
  const active: SettingsSectionId | undefined = ids.find((id) => id === chosen) ?? ids[0];
  const activeSection = sections.find((section) => section.id === active);
  const canReset =
    activeSection !== undefined &&
    (activeSection.controls.length > 0 ||
      slots.some((slot) => slot.section === activeSection.id && slot.reset !== undefined));

  const choose = (id: SettingsSectionId): void => {
    setChosen(id);
    rememberTab(id);
  };

  // Start keyboard and screen-reader users inside the dialog.
  useEffect(() => {
    const panel = panelRef.current;
    if (panel === null) return;
    const first =
      panel.querySelector<HTMLElement>('[role="tabpanel"]:not([hidden]) input[role="switch"]') ??
      panel.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]') ??
      panel;
    first.focus({ preventScroll: true });
  }, []);

  useEffect(() => {
    if (bodyRef.current !== null) bodyRef.current.scrollTop = 0;
  }, [active]);

  // R2127000: Back closes the panel as Close does, through the Dialog.
  return (
    <Dialog
      role="dialog"
      sheet
      title={t("Settings")}
      onClose={onClose}
      actions={[]}
      testid="settings-panel"
      scrimTestid="settings-scrim"
      closeTestid="settings-close"
      closeLabel={t("Close settings")}
      className="settings-panel"
      panelRef={panelRef}
    >
      <Tabs
        label={t("Settings sections")}
        testId="settings-tablist"
        idPrefix={base}
        value={active}
        onChange={choose}
        keepMounted
        tablistClassName="settings-tabs"
        tabClassName="settings-tab"
        panelsClassName="settings-body"
        panelsRef={bodyRef}
        panelAs="section"
        items={sections.map((section) => {
          const sectionSlots = slots.filter((slot) => slot.section === section.id);
          return {
            id: section.id,
            label: t(section.title),
            testId: `settings-tab-${section.id}`,
            panelTestId: `settings-section-${section.id}`,
            panelClassName: "settings-section",
            panel: (
              <>
                <h2 className="settings-section-title">{t(section.title)}</h2>
                {section.controls.map((setting) => (
                  <SettingSwitch key={setting} setting={setting} checked={settings[setting]} />
                ))}
                {sectionSlots.map((slot) => (
                  <div key={slot.id} className="settings-slot" data-settings-slot={slot.id}>
                    {slot.render()}
                  </div>
                ))}
              </>
            ),
          };
        })}
      />
      {/*
        R1758000: a save to the account that failed says so, quietly, on every tab, with Try again;
        nothing while a request is out, once one succeeds, or with nothing syncing. R634's own
        retries (online, the tab shown again, the next change) are unchanged.
      */}
      {sync.phase === "error" ? (
        <div className="settings-sync">
          <p className="settings-sync__text" role="status" data-testid="settings-sync-error">
            {t("Settings didn't save to your account.")}
          </p>
          <TryAgainButton onRetry={retrySettingsSync} testId="settings-sync-retry" className="settings-sync__retry" />
        </div>
      ) : null}
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
          {t("Reset this tab")}
        </button>
        <button
          type="button"
          className="settings-reset"
          data-testid="settings-reset"
          onClick={() => {
            resetSettings();
            for (const slot of slots) slot.reset?.();
          }}
        >
          {t("Reset all")}
        </button>
      </footer>
    </Dialog>
  );
}
