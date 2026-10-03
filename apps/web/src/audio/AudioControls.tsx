// The full audio panel (SPEC §10.11 "Settings"): master, effects, voice and music volume, mute, voice
// lines on or off, and the music's station, dynamic music, ducking and whether it plays on in a
// background tab (R631). Task 7's
// settings panel mounts it at integration. Every control reads and writes the audio settings store,
// which is the only place these values live.
//
// A volume is set by ear (B54): moving the master or effects slider ticks at the new level (the
// engine's retrigger guard keeps a drag from rattling), and letting go of the voice slider speaks a
// sample line at the lowest priority, so it never talks over a card.

import { useId, type ChangeEvent, type ReactElement } from "react";

import { MUSIC_STATIONS, VOICE_PREVIEW_DEF_ID, VOICE_PRIORITY } from "./constants.ts";
import { getAudioEngine } from "./engine.ts";
import { useAudioSettings, writeAudioSettings } from "./settings.ts";
import type { AudioSettings, MusicStation } from "./types.ts";
import "./audio.css";

export type AudioControlsProps = { className?: string };

/** A volume slider runs 0–100 in steps of 5; the store holds the fraction. */
const RANGE_MAX = 100;
const RANGE_STEP = 5;

type VolumeKey = "master" | "sfx" | "voice" | "music";

function preview(key: VolumeKey): void {
  if (key === "voice") getAudioEngine().playVoice(VOICE_PREVIEW_DEF_ID, "play", 0, VOICE_PRIORITY.summon);
  // The music is its own preview: whatever plays changes level as the slider moves.
  else if (key !== "music") getAudioEngine().playSfx("uiClick");
}

const VOLUMES: readonly { key: VolumeKey; label: string; testid: string }[] = [
  { key: "master", label: "Master volume", testid: "audio-master" },
  { key: "sfx", label: "Effects volume", testid: "audio-sfx" },
  { key: "voice", label: "Voice volume", testid: "audio-voice" },
  { key: "music", label: "Music volume", testid: "audio-music" },
];

/** R631: what each station sounds like, as the picker names it. */
const STATION_LABELS: Record<MusicStation, string> = {
  tavern: "Tavern",
  edm: "EDM",
  lofi: "Lo-fi",
  epic: "Epic Orchestral",
};

type MusicFlag = "dynamicMusic" | "duckMusic" | "playMusicInBackground";

const MUSIC_FLAGS: readonly { key: MusicFlag; label: string; testid: string; hint?: string }[] = [
  { key: "dynamicMusic", label: "Dynamic music", testid: "audio-dynamic-music" },
  { key: "duckMusic", label: "Lower music under voices and big moments", testid: "audio-duck-music" },
  { key: "playMusicInBackground", label: "Keep playing music when this tab is in the background", testid: "audio-music-background",
    hint: "Off: the music fades out when you switch to another tab or window, and back in when you return. Sound effects and voices stay quiet either way.",
  },
];

function percent(settings: AudioSettings, key: VolumeKey): number {
  return Math.round(settings[key] * RANGE_MAX);
}

export default function AudioControls({ className }: AudioControlsProps): ReactElement {
  const [settings] = useAudioSettings();
  const id = useId();

  return (
    <fieldset
      className={className === undefined || className === "" ? "audio-controls" : `audio-controls ${className}`}
      data-testid="audio-controls"
    >
      <legend>Audio</legend>

      {VOLUMES.map(({ key, label, testid }) => {
        const inputId = `${id}-${key}`;
        const value = percent(settings, key);
        return (
          <div className="audio-controls__row" key={key}>
            <label htmlFor={inputId}>{label}</label>
            <input
              id={inputId}
              type="range"
              min={0}
              max={RANGE_MAX}
              step={RANGE_STEP}
              value={value}
              data-testid={testid}
              onChange={(event: ChangeEvent<HTMLInputElement>) => {
                const patch: Partial<AudioSettings> = {};
                patch[key] = Number(event.currentTarget.value) / RANGE_MAX;
                writeAudioSettings(patch);
                if (key !== "voice") preview(key);
              }}
              onPointerUp={key === "voice" ? () => preview(key) : undefined}
              onKeyUp={key === "voice" ? () => preview(key) : undefined}
            />
            <output className="audio-controls__value" htmlFor={inputId} aria-hidden="true">
              {value}%
            </output>
          </div>
        );
      })}

      <div className="audio-controls__row audio-controls__row--check">
        <input
          id={`${id}-mute`}
          type="checkbox"
          checked={settings.muted}
          data-testid="audio-mute"
          onChange={(event: ChangeEvent<HTMLInputElement>) => {
            writeAudioSettings({ muted: event.currentTarget.checked });
          }}
        />
        <label htmlFor={`${id}-mute`}>Mute all sound</label>
      </div>

      <div className="audio-controls__row audio-controls__row--check">
        <input
          id={`${id}-voice-on`}
          type="checkbox"
          checked={settings.voiceOn}
          data-testid="audio-voice-on"
          onChange={(event: ChangeEvent<HTMLInputElement>) => {
            writeAudioSettings({ voiceOn: event.currentTarget.checked });
          }}
        />
        <label htmlFor={`${id}-voice-on`}>Voice lines</label>
      </div>

      <div className="audio-controls__row">
        <label htmlFor={`${id}-station`}>Music station</label>
        <select
          id={`${id}-station`}
          value={settings.station}
          data-testid="audio-station"
          onChange={(event: ChangeEvent<HTMLSelectElement>) => {
            const station = MUSIC_STATIONS.find((s) => s === event.currentTarget.value);
            if (station !== undefined) writeAudioSettings({ station });
          }}
        >
          {MUSIC_STATIONS.map((station) => (
            <option key={station} value={station}>
              {STATION_LABELS[station]}
            </option>
          ))}
        </select>
      </div>

      {MUSIC_FLAGS.map(({ key, label, testid, hint }) => (
        <div className="audio-controls__row audio-controls__row--check" key={key}>
          <input
            id={`${id}-${key}`}
            type="checkbox"
            checked={settings[key]}
            data-testid={testid}
            aria-describedby={hint === undefined ? undefined : `${id}-${key}-hint`}
            onChange={(event: ChangeEvent<HTMLInputElement>) => {
              const patch: Partial<AudioSettings> = {};
              patch[key] = event.currentTarget.checked;
              writeAudioSettings(patch);
            }}
          />
          <label htmlFor={`${id}-${key}`}>{label}</label>
          {hint === undefined ? null : (
            <p className="audio-controls__hint" id={`${id}-${key}-hint`}>
              {hint}
            </p>
          )}
        </div>
      ))}
    </fieldset>
  );
}
