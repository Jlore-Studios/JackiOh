// The start screen's preset starting decks (`RUN_PRESET_DECKS`, shown through `runConstants()`):
// one tile per deck, its name, its theme line and a fan of its 10 card faces. Presentation only
// (CLAUDE.md rule 7): one click picks the deck, and the player then tweaks it in the builder.

import type { ReactElement } from "react";

import type { RunPoolCard, RunPresetDeck } from "@jackioh/run";

import { t } from "../../i18n/i18n.ts";
import { dungeonTestid } from "../testids.ts";
import { PoolFace } from "./PoolFace.tsx";

export type PresetDecksProps = {
  /** The named decks `runConstants()` states. */
  presets: readonly RunPresetDeck[];
  /** Every card the run unlocks, to resolve a preset's faces. */
  offered: readonly RunPoolCard[];
  /** The deck the builder holds now, to mark the preset it matches. */
  picked: readonly string[];
  onPick: (cards: string[]) => void;
};

export function PresetDecks({ presets, offered, picked, onPick }: PresetDecksProps): ReactElement {
  const byId = new Map(offered.map((card) => [card.id, card]));
  return (
    <section className="dungeon-presets" aria-label={t("Preset decks")} data-testid={dungeonTestid.presets}>
      <h3>{t("Preset decks")}</h3>
      <div className="dungeon-presets-grid">
        {presets.map((preset) => {
          const active = preset.cards.length === picked.length && preset.cards.every((id) => picked.includes(id));
          return (
            <button
              key={preset.id}
              type="button"
              className="dungeon-preset"
              aria-pressed={active}
              data-testid={dungeonTestid.preset(preset.id)}
              onClick={() => {
                onPick(preset.cards.filter((id) => byId.has(id)));
              }}
            >
              <span className="dungeon-preset__name">{t(preset.name)}</span>
              <span className="dungeon-preset__theme">{t(preset.theme)}</span>
              <span className="dungeon-preset__fan" aria-hidden="true">
                {preset.cards.map((id) => {
                  const card = byId.get(id);
                  return card === undefined ? null : (
                    <span key={id} className="dungeon-preset__face">
                      <PoolFace card={card} />
                    </span>
                  );
                })}
              </span>
            </button>
          );
        })}
      </div>
    </section>
  );
}
