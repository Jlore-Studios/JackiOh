// The panel's controls for the stores tasks 1 and 6 own (slots.ts mounts them). Each reads its own
// store and writes straight back, like the built-in switches, so a change applies at once: the
// animation runner reads the effects speed at every enqueue (R201), the effects layer re-renders on
// the intensity, and every card face on the foil switch.

import { useId, type CSSProperties, type ReactElement } from "react";

import { CARD_SETTINGS_FIELDS, useCardSettings, writeCardSettings } from "../cards/settings.ts";
import { FX_SPEED_DEFAULT, FX_SPEED_MAX, FX_SPEED_MIN, FX_SPEED_STEP } from "../fx/constants.ts";
import { useFxSettings, type FxIntensity } from "../fx/settings.ts";

/** Every intensity the effects layer knows, in the order the picker lists them. */
const INTENSITIES: readonly { value: FxIntensity; label: string }[] = [
  { value: "off", label: "Off" },
  { value: "low", label: "Low" },
  { value: "normal", label: "Normal" },
  { value: "high", label: "High" },
];

/** "1.5×", "0.25×": the speed as the slider's readout and its value text say it (R435). */
export function speedLabel(speed: number): string {
  return `${String(Math.round(speed * 100) / 100)}×`;
}

/** Where on the track `speed` sits, 0 to 1, for the filled part of the slider. */
function trackFill(speed: number): number {
  return (speed - FX_SPEED_MIN) / (FX_SPEED_MAX - FX_SPEED_MIN);
}

/**
 * R435: the effects speed as a slider from FX_SPEED_MIN to FX_SPEED_MAX in FX_SPEED_STEP steps, with
 * its value beside it. A native range input, so the arrow keys, Page Up/Down, Home and End move it and
 * assistive tech reads it; the row is the 44px touch target. Ticks mark the slowest, the default and
 * the fastest.
 */
function SpeedSlider(props: { speed: number; onChange: (speed: number) => void }): ReactElement {
  const id = useId();
  const style = { "--fx-speed-fill": String(trackFill(props.speed)) } as CSSProperties;
  return (
    <div className="settings-row settings-row--slider">
      <div className="settings-control settings-control--slider">
        <label className="settings-label" htmlFor={`${id}-range`}>
          Effects speed
        </label>
        <output className="settings-slider-value" htmlFor={`${id}-range`} data-testid="setting-fxSpeed-value" aria-live="off">
          {speedLabel(props.speed)}
        </output>
      </div>
      <input
        id={`${id}-range`}
        type="range"
        className="settings-slider"
        data-testid="setting-fxSpeed"
        min={FX_SPEED_MIN}
        max={FX_SPEED_MAX}
        step={FX_SPEED_STEP}
        value={props.speed}
        aria-valuetext={speedLabel(props.speed)}
        aria-describedby={`${id}-hint`}
        style={style}
        list={`${id}-ticks`}
        onChange={(event) => {
          props.onChange(Number(event.currentTarget.value));
        }}
      />
      <datalist id={`${id}-ticks`}>
        <option value={FX_SPEED_MIN} label={speedLabel(FX_SPEED_MIN)} />
        <option value={FX_SPEED_DEFAULT} label={speedLabel(FX_SPEED_DEFAULT)} />
        <option value={FX_SPEED_MAX} label={speedLabel(FX_SPEED_MAX)} />
      </datalist>
      <div className="settings-slider-scale" aria-hidden="true">
        <span>{speedLabel(FX_SPEED_MIN)}</span>
        <span>{speedLabel(FX_SPEED_MAX)}</span>
      </div>
      <p className="settings-hint" id={`${id}-hint`}>
        How fast cards move, hit and die. Faster also shortens the pauses between them.
      </p>
    </div>
  );
}

/** A labelled native select: the platform's own picker on a phone, a 44 px row everywhere. */
function SelectRow(props: {
  label: string;
  hint: string;
  testId: string;
  value: string;
  options: readonly { value: string; label: string }[];
  onChange: (value: string) => void;
}): ReactElement {
  const id = useId();
  return (
    <div className="settings-row">
      <label className="settings-control" htmlFor={`${id}-select`}>
        <span className="settings-label">{props.label}</span>
        <select
          id={`${id}-select`}
          className="settings-select"
          data-testid={props.testId}
          value={props.value}
          aria-describedby={`${id}-hint`}
          onChange={(event) => {
            props.onChange(event.currentTarget.value);
          }}
        >
          {props.options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </label>
      <p className="settings-hint" id={`${id}-hint`}>
        {props.hint}
      </p>
    </div>
  );
}

/** Task 1's effects speed (R201, R435) and intensity. */
export function FxControls(): ReactElement {
  const [settings, set] = useFxSettings();
  return (
    <>
      <SpeedSlider
        speed={settings.speed}
        onChange={(speed) => {
          set({ speed });
        }}
      />
      <SelectRow
        label="Effects intensity"
        hint="Fire, sparks, light rays and the board shake. Off keeps the card motion only."
        testId="setting-fxIntensity"
        value={settings.intensity}
        options={INTENSITIES}
        onChange={(value) => {
          const next = INTENSITIES.find((option) => option.value === value);
          if (next !== undefined) set({ intensity: next.value });
        }}
      />
    </>
  );
}

/** Task 6's words for its foil switch (cards/settings.ts), so the panel and the module agree. */
const FOIL_FIELD = CARD_SETTINGS_FIELDS.find((field) => field.key === "animatedFoil");

/** Task 6's animated foil on Mythic and Radiant faces. */
export function AnimatedFoilSwitch(): ReactElement {
  const settings = useCardSettings();
  const hintId = useId();
  return (
    <div className="settings-row">
      <label className="settings-control">
        <span className="settings-label">{FOIL_FIELD?.label ?? "Animated foil"}</span>
        <input
          type="checkbox"
          role="switch"
          className="settings-switch"
          data-testid="setting-animatedFoil"
          checked={settings.animatedFoil}
          aria-describedby={hintId}
          onChange={(event) => {
            writeCardSettings({ animatedFoil: event.currentTarget.checked });
          }}
        />
      </label>
      <p className="settings-hint" id={hintId}>
        {FOIL_FIELD?.description}
      </p>
    </div>
  );
}
