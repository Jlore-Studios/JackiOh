// The HUD mute button (SPEC §10.11 "Settings"): it writes the audio settings store, which the engine
// subscribes to. Unmuting answers with a click (B54): the page-wide tick (uiSounds.ts) runs in the capture
// phase, before this handler, while the store still says muted, and is refused, so this one plays its own.

import type { ReactElement } from "react";

import { getAudioEngine } from "./engine.ts";
import { useAudioSettings, writeAudioSettings } from "./settings.ts";
import { t } from "../i18n/i18n.ts";
import "./audio.css";

export type AudioToggleProps = { className?: string };

function SpeakerIcon({ muted }: { muted: boolean }): ReactElement {
  return (
    <svg
      className="audio-toggle__icon"
      viewBox="0 0 24 24"
      width="22"
      height="22"
      aria-hidden="true"
      focusable="false"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M4 9h4l5-4v14l-5-4H4z" fill="currentColor" />
      {muted ? (
        <>
          <path d="M17 9l5 6" />
          <path d="M22 9l-5 6" />
        </>
      ) : (
        <>
          <path d="M16 9.5a3.5 3.5 0 0 1 0 5" />
          <path d="M18.5 7a7 7 0 0 1 0 10" />
        </>
      )}
    </svg>
  );
}

export default function AudioToggle({ className }: AudioToggleProps): ReactElement {
  const [settings] = useAudioSettings();
  const muted = settings.muted;
  const label = muted ? t("Unmute sound") : t("Mute sound");
  return (
    <button
      type="button"
      className={className === undefined || className === "" ? "audio-toggle" : `audio-toggle ${className}`}
      data-testid="audio-toggle"
      aria-pressed={muted}
      aria-label={label}
      title={label}
      onClick={() => {
        const next = writeAudioSettings({ muted: !muted });
        if (!next.muted) getAudioEngine().playSfx("uiClick");
      }}
    >
      <SpeakerIcon muted={muted} />
    </button>
  );
}
