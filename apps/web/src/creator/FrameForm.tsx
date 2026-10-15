// The creator's frame form (#920, §12.2): name, type, cost, rarity, a Unit's stats and keywords,
// the text and its numbers. The inputs offer the ranges the limits name, but nothing here refuses a
// card (CLAUDE.md rule 7): each issue shown under its field is the engine validator's.

import type { ReactElement, ReactNode } from "react";

import {
  CUSTOM_ATTACK_MAX,
  CUSTOM_COST_MAX,
  CUSTOM_HEALTH_MAX,
  CUSTOM_KEYWORDS,
  CUSTOM_NAME_MAX_LENGTH,
  CUSTOM_PARAM_MAX,
  CUSTOM_RADIANT_STAT_MAX,
  CUSTOM_TEXT_MAX_LENGTH,
} from "@jackioh/engine/config";
import type { CustomFrame, CustomIssue, CustomParam, PrintedRarity } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import { creatorTestid } from "./testids.ts";

const RARITIES: readonly PrintedRarity[] = ["Common", "Rare", "Epic", "Legendary", "Mythic"];

function Issues({ issues, field }: { issues: readonly CustomIssue[]; field: string }): ReactElement | null {
  const mine = issues.filter((issue) => issue.field === field);
  if (mine.length === 0) return null;
  return (
    <ul className="creator-issues">
      {mine.map((issue, index) => (
        <li key={index} className="creator-issues__item" data-testid={creatorTestid.frameIssue} data-field={field}>
          {issue.message}
        </li>
      ))}
    </ul>
  );
}

function Field({ label, children, issues, field }: { label: string; children: ReactNode; issues: readonly CustomIssue[]; field: string }): ReactElement {
  return (
    <div className="creator-field">
      <label>
        <span className="creator-field__label">{label}</span>
        {children}
      </label>
      <Issues issues={issues} field={field} />
    </div>
  );
}

function integer(value: string): number {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

export type FrameFormProps = {
  frame: CustomFrame;
  issues: readonly CustomIssue[];
  /** In the builder, the numbers are the blocks' and the table is the builder's; in code, the player's. */
  editParams: boolean;
  onChange: (frame: CustomFrame) => void;
};

export function FrameForm({ frame, issues, editParams, onChange }: FrameFormProps): ReactElement {
  const unit = frame.type === "Unit";
  const keywords = frame.keywords ?? [];
  const params = frame.params ?? [];
  const radiantStats = frame.radiantAttack !== undefined || frame.radiantHealth !== undefined;
  const setParams = (next: CustomParam[]): void => onChange({ ...frame, params: next });
  return (
    <div className="creator-frame">
      <Field label={t("Name")} issues={issues} field="name">
        <input
          type="text"
          data-testid={creatorTestid.name}
          value={frame.name}
          maxLength={CUSTOM_NAME_MAX_LENGTH}
          onChange={(event) => onChange({ ...frame, name: event.target.value })}
        />
      </Field>
      <div className="creator-row">
        <Field label={t("Type")} issues={issues} field="type">
          <select
            data-testid={creatorTestid.type}
            value={frame.type}
            onChange={(event) => onChange({ ...frame, type: event.target.value === "Spell" ? "Spell" : "Unit" })}
          >
            <option value="Unit">{t("Unit")}</option>
            <option value="Spell">{t("Spell")}</option>
          </select>
        </Field>
        <Field label={t("Cost")} issues={issues} field="cost">
          <input
            type="number"
            data-testid={creatorTestid.cost}
            min={0}
            max={CUSTOM_COST_MAX}
            value={frame.cost}
            onChange={(event) => onChange({ ...frame, cost: integer(event.target.value) })}
          />
        </Field>
        <Field label={t("Rarity")} issues={issues} field="rarity">
          <select
            data-testid={creatorTestid.rarity}
            value={frame.rarity}
            onChange={(event) => onChange({ ...frame, rarity: event.target.value as PrintedRarity })}
          >
            {RARITIES.map((rarity) => (
              <option key={rarity} value={rarity}>
                {t(rarity)}
              </option>
            ))}
          </select>
        </Field>
      </div>
      {unit ? (
        <>
          <div className="creator-row">
            <Field label={t("Attack")} issues={issues} field="attack">
              <input
                type="number"
                data-testid={creatorTestid.attack}
                min={0}
                max={CUSTOM_ATTACK_MAX}
                value={frame.attack ?? 0}
                onChange={(event) => onChange({ ...frame, attack: integer(event.target.value) })}
              />
            </Field>
            <Field label={t("Health")} issues={issues} field="health">
              <input
                type="number"
                data-testid={creatorTestid.health}
                min={1}
                max={CUSTOM_HEALTH_MAX}
                value={frame.health ?? 1}
                onChange={(event) => onChange({ ...frame, health: integer(event.target.value) })}
              />
            </Field>
          </div>
          <label className="creator-check">
            <input
              type="checkbox"
              checked={radiantStats}
              onChange={(event) => {
                if (event.target.checked) {
                  onChange({ ...frame, radiantAttack: (frame.attack ?? 0) * 2, radiantHealth: (frame.health ?? 1) * 2 });
                } else {
                  const { radiantAttack: _attack, radiantHealth: _health, ...rest } = frame;
                  onChange(rest);
                }
              }}
            />
            {t("Its Radiant face has its own stats (otherwise twice these)")}
          </label>
          {radiantStats ? (
            <div className="creator-row">
              <Field label={t("Radiant attack")} issues={issues} field="radiantAttack">
                <input
                  type="number"
                  data-testid={creatorTestid.radiantAttack}
                  min={0}
                  max={CUSTOM_RADIANT_STAT_MAX}
                  value={frame.radiantAttack ?? 0}
                  onChange={(event) => onChange({ ...frame, radiantAttack: integer(event.target.value) })}
                />
              </Field>
              <Field label={t("Radiant health")} issues={issues} field="radiantHealth">
                <input
                  type="number"
                  data-testid={creatorTestid.radiantHealth}
                  min={1}
                  max={CUSTOM_RADIANT_STAT_MAX}
                  value={frame.radiantHealth ?? 1}
                  onChange={(event) => onChange({ ...frame, radiantHealth: integer(event.target.value) })}
                />
              </Field>
            </div>
          ) : null}
          <fieldset className="creator-keywords">
            <legend>{t("Keywords")}</legend>
            {CUSTOM_KEYWORDS.map((keyword) => (
              <label key={keyword} className="creator-check">
                <input
                  type="checkbox"
                  data-testid={creatorTestid.keyword}
                  value={keyword}
                  checked={keywords.includes(keyword)}
                  onChange={(event) =>
                    onChange({
                      ...frame,
                      keywords: event.target.checked ? [...keywords, keyword] : keywords.filter((held) => held !== keyword),
                    })
                  }
                />
                {t(keyword)}
              </label>
            ))}
            <Issues issues={issues} field="keywords" />
          </fieldset>
        </>
      ) : null}
      <Field label={t("Text")} issues={issues} field="text">
        <textarea
          data-testid={creatorTestid.text}
          rows={3}
          maxLength={CUSTOM_TEXT_MAX_LENGTH}
          value={frame.text}
          onChange={(event) => onChange({ ...frame, text: event.target.value })}
        />
      </Field>
      {editParams ? (
        <div className="creator-params" data-testid={creatorTestid.params}>
          <p className="creator-hint">{t("Each {number} in the text is one of these. Nerfs and Buffs move it; its Radiant face prints the Radiant value.")}</p>
          {params.map((param, index) => (
            <div key={index} className="creator-row">
              <input
                type="text"
                data-testid={creatorTestid.paramKey}
                aria-label={t("Number name")}
                value={param.key}
                onChange={(event) => setParams(params.map((old, at) => (at === index ? { ...old, key: event.target.value } : old)))}
              />
              <input
                type="number"
                data-testid={creatorTestid.paramBase}
                aria-label={t("Base")}
                min={0}
                max={CUSTOM_PARAM_MAX}
                value={param.base}
                onChange={(event) => setParams(params.map((old, at) => (at === index ? { ...old, base: integer(event.target.value) } : old)))}
              />
              <input
                type="number"
                data-testid={creatorTestid.paramRadiant}
                aria-label={t("Radiant")}
                min={0}
                max={CUSTOM_PARAM_MAX}
                value={param.radiant}
                onChange={(event) => setParams(params.map((old, at) => (at === index ? { ...old, radiant: integer(event.target.value) } : old)))}
              />
              <button
                type="button"
                className="button-secondary creator-icon-button"
                data-testid={creatorTestid.removeParam}
                onClick={() => setParams(params.filter((_, at) => at !== index))}
              >
                {t("Remove")}
              </button>
            </div>
          ))}
          <button
            type="button"
            className="button-secondary"
            data-testid={creatorTestid.addParam}
            onClick={() => setParams([...params, { key: `n${String(params.length + 1)}`, base: 1, radiant: 2 }])}
          >
            {t("Add a number")}
          </button>
          <Issues issues={issues} field="params" />
        </div>
      ) : (
        <Issues issues={issues} field="params" />
      )}
    </div>
  );
}
