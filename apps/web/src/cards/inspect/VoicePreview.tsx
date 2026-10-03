// A card's voice lines, previewed from its detail view (SPEC §10.10, R630): a mic dropdown, one
// option per line the card has, playing through the audio engine like the settings panel's own
// preview does. Presentation only (CLAUDE.md rule 7): it reads the public voice table through
// `entryFor`, which is null for the hidden sentinel and for a fused transient definition, so R203
// holds, and it changes none of R204's in-game speech moments. Muted or voice-off refusal lives
// inside the engine (`playVoice` returns false), so there are no settings checks here.

import type { ChangeEvent, ReactElement } from "react";

import { VOICE_PRIORITY } from "../../audio/constants.ts";
import { getAudioEngine } from "../../audio/engine.ts";
import type { VoiceLineKind } from "../../audio/types.ts";
import { entryFor, VOICE_LINES } from "../../audio/voiceData.ts";

function MicIcon(): ReactElement {
  return (
    <svg
      viewBox="0 0 24 24"
      width="16"
      height="16"
      aria-hidden="true"
      focusable="false"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <rect x="9" y="3" width="6" height="11" rx="3" />
      <path d="M5 11a7 7 0 0 0 14 0" />
      <path d="M12 18v3" />
    </svg>
  );
}

const UNIT_LINES: readonly VoiceLineKind[] = ["play", "death"];
const CAST_LINES: readonly VoiceLineKind[] = ["cast"];

const LINE_LABELS: Record<VoiceLineKind, string> = {
  play: "Play",
  death: "Death",
  cast: "Cast",
};

export function VoicePreview({ defId }: { defId: string }): ReactElement | null {
  const entry = entryFor(VOICE_LINES, defId);
  if (entry === null) return null;
  const lines = entry.kind === "unit" ? UNIT_LINES : CAST_LINES;
  const onChange = (event: ChangeEvent<HTMLSelectElement>): void => {
    const line = event.currentTarget.value;
    if (line === "") return;
    getAudioEngine().playVoice(defId, line as VoiceLineKind, 0, VOICE_PRIORITY.summon);
    event.currentTarget.value = "";
  };
  return (
    <label className="voice-preview">
      <MicIcon />
      <select data-testid="voice-preview" aria-label="Preview voice line" defaultValue="" onChange={onChange}>
        <option value="">Voice</option>
        {lines.map((line) => (
          <option key={line} value={line}>
            {LINE_LABELS[line]}
          </option>
        ))}
      </select>
    </label>
  );
}
