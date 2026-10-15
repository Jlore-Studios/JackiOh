// The creator's no-code builder (#920, K3): the card as blocks picked from menus, every word one of
// the card kit's (`builder.ts`). A Cry, a Death, a start or end of turn each hold effects and may
// hold a condition; a Unit may also have an aura and an Activate. Each amount is one of the card's
// numbers, whose Radiant value the numbers table sets.

import type { ReactElement } from "react";

import { CUSTOM_COST_MAX, CUSTOM_KEYWORDS, CUSTOM_PARAM_MAX } from "@jackioh/engine/config";
import type { CardType } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import {
  AURA_WORDS,
  COUNT_WORDS,
  ONE_WORDS,
  OTHER_WORDS,
  TOKEN_WORDS,
  TRIGGERS,
  numbersOf,
  type BuilderActivate,
  type BuilderAura,
  type BuilderCondition,
  type BuilderHook,
  type BuilderModel,
  type BuilderStep,
  type BuilderTarget,
  type BuilderVerb,
  type Trigger,
} from "./builder.ts";
import { creatorTestid } from "./testids.ts";

const TRIGGER_LABEL: Readonly<Record<Trigger, string>> = {
  cry: "Cry (when played)",
  death: "Death",
  startOfTurn: "Start of your turn",
  endOfTurn: "End of your turn",
};

const VERBS: readonly BuilderVerb[] = ["damage", "heal", "destroy", "bounce", "buff", "give", "draw", "discard", "summon", "armor", "mana"];

const VERB_LABEL: Readonly<Record<BuilderVerb, string>> = {
  damage: "Deal damage",
  heal: "Restore health",
  destroy: "Destroy",
  bounce: "Return to hand",
  buff: "Give +attack/+health",
  give: "Give a keyword",
  draw: "Draw cards",
  discard: "Discard random cards",
  summon: "Summon tokens",
  armor: "Gain Armor",
  mana: "Gain mana",
};


function defaultStep(verb: BuilderVerb, declared: boolean): BuilderStep {
  const aim = (words: (typeof ONE_WORDS)[number]): BuilderTarget => (declared ? { kind: "declared" } : { kind: "ask", words });
  switch (verb) {
    case "damage":
      return { verb, target: aim("an enemy unit"), amount: 1 };
    case "heal":
      return { verb, target: aim("a friendly unit"), amount: 2 };
    case "destroy":
    case "bounce":
      return { verb, target: aim("an enemy unit") };
    case "buff":
      return { verb, target: aim("a friendly unit"), attack: 1, health: 1 };
    case "give":
      return { verb, target: aim("a friendly unit"), keyword: CUSTOM_KEYWORDS[0] ?? "Taunt" };
    case "draw":
    case "armor":
    case "mana":
      return { verb, amount: 1 };
    case "discard":
      return { verb, amount: 1, who: "the enemy" };
    case "summon":
      return { verb, token: TOKEN_WORDS[0], amount: 1 };
  }
}

function targetValue(target: BuilderTarget): string {
  return target.kind === "declared" ? "declared" : `${target.kind}:${target.words}`;
}

function targetOf(value: string): BuilderTarget {
  if (value === "declared") return { kind: "declared" };
  const [kind, ...rest] = value.split(":");
  const words = rest.join(":");
  if (kind === "ask" && (ONE_WORDS as readonly string[]).includes(words)) return { kind: "ask", words: words as (typeof ONE_WORDS)[number] };
  return { kind: "words", words: words as (typeof OTHER_WORDS)[number] };
}

function NumberInput({
  value,
  testid,
  label,
  min = 0,
  max = CUSTOM_PARAM_MAX,
  onChange,
}: {
  value: number;
  testid: string;
  label: string;
  min?: number;
  max?: number;
  onChange: (value: number) => void;
}): ReactElement {
  return (
    <input
      type="number"
      className="creator-number"
      data-testid={testid}
      aria-label={label}
      min={min}
      max={max}
      value={value}
      onChange={(event) => {
        const parsed = Number.parseInt(event.target.value, 10);
        onChange(Number.isFinite(parsed) ? parsed : 0);
      }}
    />
  );
}

function TargetSelect({
  target,
  declared,
  onChange,
}: {
  target: BuilderTarget;
  declared: string | null;
  onChange: (target: BuilderTarget) => void;
}): ReactElement {
  return (
    <select
      data-testid={creatorTestid.stepTarget}
      aria-label={t("Target")}
      value={targetValue(target)}
      onChange={(event) => onChange(targetOf(event.target.value))}
    >
      {declared === null ? null : <option value="declared">{t("The card's target ({words})", { words: t(declared) })}</option>}
      <optgroup label={t("The player chooses")}>
        {ONE_WORDS.map((words) => (
          <option key={words} value={`ask:${words}`}>
            {t(words)}
          </option>
        ))}
      </optgroup>
      <optgroup label={t("No choice")}>
        {OTHER_WORDS.map((words) => (
          <option key={words} value={`words:${words}`}>
            {t(words)}
          </option>
        ))}
      </optgroup>
    </select>
  );
}

function StepEditor({
  step,
  declared,
  onChange,
  onRemove,
}: {
  step: BuilderStep;
  declared: string | null;
  onChange: (step: BuilderStep) => void;
  onRemove: () => void;
}): ReactElement {
  const fields = ((): ReactElement => {
    switch (step.verb) {
      case "damage":
      case "heal":
        return (
          <>
            <TargetSelect target={step.target} declared={declared} onChange={(target) => onChange({ ...step, target })} />
            <NumberInput value={step.amount} testid={creatorTestid.stepAmount} label={t("Amount")} onChange={(amount) => onChange({ ...step, amount })} />
          </>
        );
      case "destroy":
      case "bounce":
        return <TargetSelect target={step.target} declared={declared} onChange={(target) => onChange({ ...step, target })} />;
      case "buff":
        return (
          <>
            <TargetSelect target={step.target} declared={declared} onChange={(target) => onChange({ ...step, target })} />
            <NumberInput value={step.attack} testid={creatorTestid.stepAttack} label={t("Attack")} onChange={(attack) => onChange({ ...step, attack })} />
            <NumberInput value={step.health} testid={creatorTestid.stepHealth} label={t("Health")} onChange={(health) => onChange({ ...step, health })} />
          </>
        );
      case "give":
        return (
          <>
            <TargetSelect target={step.target} declared={declared} onChange={(target) => onChange({ ...step, target })} />
            <select
              data-testid={creatorTestid.stepKeyword}
              aria-label={t("Keyword")}
              value={step.keyword}
              onChange={(event) => onChange({ ...step, keyword: event.target.value })}
            >
              {CUSTOM_KEYWORDS.map((keyword) => (
                <option key={keyword} value={keyword}>
                  {t(keyword)}
                </option>
              ))}
            </select>
          </>
        );
      case "draw":
      case "armor":
      case "mana":
        return <NumberInput value={step.amount} testid={creatorTestid.stepAmount} label={t("Amount")} onChange={(amount) => onChange({ ...step, amount })} />;
      case "discard":
        return (
          <>
            <NumberInput value={step.amount} testid={creatorTestid.stepAmount} label={t("Amount")} onChange={(amount) => onChange({ ...step, amount })} />
            <select
              data-testid={creatorTestid.stepWho}
              aria-label={t("Who discards")}
              value={step.who}
              onChange={(event) => onChange({ ...step, who: event.target.value === "you" ? "you" : "the enemy" })}
            >
              <option value="the enemy">{t("The enemy")}</option>
              <option value="you">{t("You")}</option>
            </select>
          </>
        );
      case "summon":
        return (
          <>
            <select
              data-testid={creatorTestid.stepToken}
              aria-label={t("Token")}
              value={step.token}
              onChange={(event) => onChange({ ...step, token: event.target.value as (typeof TOKEN_WORDS)[number] })}
            >
              {TOKEN_WORDS.map((token) => (
                <option key={token} value={token}>
                  {t(token)}
                </option>
              ))}
            </select>
            <NumberInput value={step.amount} testid={creatorTestid.stepAmount} label={t("Amount")} onChange={(amount) => onChange({ ...step, amount })} />
          </>
        );
    }
  })();
  return (
    <li className="creator-step" data-testid={creatorTestid.step}>
      <select
        data-testid={creatorTestid.stepVerb}
        aria-label={t("Effect")}
        value={step.verb}
        onChange={(event) => onChange(defaultStep(event.target.value as BuilderVerb, declared !== null))}
      >
        {VERBS.map((verb) => (
          <option key={verb} value={verb}>
            {t(VERB_LABEL[verb])}
          </option>
        ))}
      </select>
      {fields}
      <button type="button" className="button-secondary creator-icon-button" data-testid={creatorTestid.removeStep} onClick={onRemove}>
        {t("Remove")}
      </button>
    </li>
  );
}

function StepsEditor({
  steps,
  declared,
  onChange,
}: {
  steps: readonly BuilderStep[];
  declared: string | null;
  onChange: (steps: BuilderStep[]) => void;
}): ReactElement {
  return (
    <>
      <ol className="creator-steps">
        {steps.map((step, index) => (
          <StepEditor
            key={index}
            step={step}
            declared={declared}
            onChange={(next) => onChange(steps.map((old, at) => (at === index ? next : old)))}
            onRemove={() => onChange(steps.filter((_, at) => at !== index))}
          />
        ))}
      </ol>
      <button
        type="button"
        className="button-secondary"
        data-testid={creatorTestid.addStep}
        onClick={() => onChange([...steps, defaultStep("damage", declared !== null)])}
      >
        {t("Add an effect")}
      </button>
    </>
  );
}

function ConditionEditor({
  condition,
  onChange,
}: {
  condition: BuilderCondition | undefined;
  onChange: (condition: BuilderCondition | undefined) => void;
}): ReactElement {
  if (condition === undefined) {
    return (
      <button
        type="button"
        className="button-secondary"
        data-testid={creatorTestid.addCondition}
        onClick={() => onChange({ count: COUNT_WORDS[0], op: ">=", value: 1 })}
      >
        {t("Add a condition")}
      </button>
    );
  }
  return (
    <div className="creator-condition">
      <span>{t("Only if")}</span>
      <select
        data-testid={creatorTestid.conditionCount}
        aria-label={t("What to count")}
        value={condition.count}
        onChange={(event) => onChange({ ...condition, count: event.target.value as (typeof COUNT_WORDS)[number] })}
      >
        {COUNT_WORDS.map((words) => (
          <option key={words} value={words}>
            {t(words)}
          </option>
        ))}
      </select>
      <select
        data-testid={creatorTestid.conditionOp}
        aria-label={t("Comparison")}
        value={condition.op}
        onChange={(event) => onChange({ ...condition, op: event.target.value === "<=" ? "<=" : ">=" })}
      >
        <option value=">=">{t("at least")}</option>
        <option value="<=">{t("at most")}</option>
      </select>
      <NumberInput
        value={condition.value}
        testid={creatorTestid.conditionValue}
        label={t("Count")}
        onChange={(value) => onChange({ ...condition, value })}
      />
      <button type="button" className="button-secondary creator-icon-button" data-testid={creatorTestid.removeCondition} onClick={() => onChange(undefined)}>
        {t("Remove")}
      </button>
    </div>
  );
}

function HookEditor({
  hook,
  triggers,
  declared,
  onChange,
  onRemove,
}: {
  hook: BuilderHook;
  triggers: readonly Trigger[];
  declared: string | null;
  onChange: (hook: BuilderHook) => void;
  onRemove: () => void;
}): ReactElement {
  // The card's declared target is chosen as it is played, so only its Cry can aim at it.
  const aimed = hook.trigger === "cry" ? declared : null;
  return (
    <section className="creator-hook" data-testid={creatorTestid.hook}>
      <header className="creator-hook__head">
        <select
          data-testid={creatorTestid.hookTrigger}
          aria-label={t("When")}
          value={hook.trigger}
          onChange={(event) => onChange({ ...hook, trigger: event.target.value as Trigger })}
        >
          {triggers.map((trigger) => (
            <option key={trigger} value={trigger}>
              {t(TRIGGER_LABEL[trigger])}
            </option>
          ))}
        </select>
        <button type="button" className="button-secondary creator-icon-button" data-testid={creatorTestid.removeHook} onClick={onRemove}>
          {t("Remove")}
        </button>
      </header>
      <ConditionEditor
        condition={hook.condition}
        onChange={(condition) => {
          const { condition: _old, ...rest } = hook;
          onChange(condition === undefined ? rest : { ...rest, condition });
        }}
      />
      <StepsEditor steps={hook.steps} declared={aimed} onChange={(steps) => onChange({ ...hook, steps })} />
    </section>
  );
}

function AuraEditor({ aura, onChange }: { aura: BuilderAura; onChange: (aura: BuilderAura | null) => void }): ReactElement {
  return (
    <section className="creator-hook" data-testid={creatorTestid.aura}>
      <header className="creator-hook__head">
        <strong>{t("Aura")}</strong>
        <button type="button" className="button-secondary creator-icon-button" data-testid={creatorTestid.removeAura} onClick={() => onChange(null)}>
          {t("Remove")}
        </button>
      </header>
      <div className="creator-condition">
        <select aria-label={t("Who it changes")} value={aura.to} onChange={(event) => onChange({ ...aura, to: event.target.value as BuilderAura["to"] })}>
          {AURA_WORDS.map((words) => (
            <option key={words} value={words}>
              {t(words)}
            </option>
          ))}
        </select>
        <span>{t("have")}</span>
        <NumberInput value={aura.attack} testid={creatorTestid.stepAttack} label={t("Attack")} onChange={(attack) => onChange({ ...aura, attack })} />
        <NumberInput value={aura.health} testid={creatorTestid.stepHealth} label={t("Health")} onChange={(health) => onChange({ ...aura, health })} />
      </div>
      <fieldset className="creator-keywords">
        <legend>{t("and the keywords")}</legend>
        {CUSTOM_KEYWORDS.map((keyword) => (
          <label key={keyword} className="creator-check">
            <input
              type="checkbox"
              checked={aura.give.includes(keyword)}
              onChange={(event) =>
                onChange({ ...aura, give: event.target.checked ? [...aura.give, keyword] : aura.give.filter((given) => given !== keyword) })
              }
            />
            {t(keyword)}
          </label>
        ))}
      </fieldset>
    </section>
  );
}

function ActivateEditor({
  activate,
  onChange,
}: {
  activate: BuilderActivate;
  onChange: (activate: BuilderActivate | null) => void;
}): ReactElement {
  return (
    <section className="creator-hook" data-testid={creatorTestid.activate}>
      <header className="creator-hook__head">
        <strong>{t("Activate")}</strong>
        <button type="button" className="button-secondary creator-icon-button" data-testid={creatorTestid.removeActivate} onClick={() => onChange(null)}>
          {t("Remove")}
        </button>
      </header>
      <div className="creator-condition">
        <label>
          {t("Button")}{" "}
          <input type="text" value={activate.label} onChange={(event) => onChange({ ...activate, label: event.target.value })} />
        </label>
        <label>
          {t("Mana")}{" "}
          <NumberInput value={activate.cost} testid={creatorTestid.activateCost} label={t("Mana")} max={CUSTOM_COST_MAX} onChange={(cost) => onChange({ ...activate, cost })} />
        </label>
      </div>
      <StepsEditor steps={activate.steps} declared={null} onChange={(steps) => onChange({ ...activate, steps })} />
    </section>
  );
}

export type BuilderViewProps = {
  model: BuilderModel;
  type: CardType;
  onChange: (model: BuilderModel) => void;
};

export function BuilderView({ model, type, onChange }: BuilderViewProps): ReactElement {
  const unit = type === "Unit";
  // A Spell resolves once, as it is cast: its Cry is all it has.
  const triggers: readonly Trigger[] = unit ? TRIGGERS : ["cry"];
  const used = new Set(model.hooks.map((hook) => hook.trigger));
  const free = triggers.find((trigger) => !used.has(trigger));
  const numbers = numbersOf(model);
  return (
    <div className="creator-builder" data-testid={creatorTestid.builder}>
      <label className="creator-field">
        <span>{t("Target chosen as it is played")}</span>
        <select
          data-testid={creatorTestid.target}
          value={model.target ?? ""}
          onChange={(event) => {
            const value = event.target.value;
            const target = (ONE_WORDS as readonly string[]).includes(value) ? (value as (typeof ONE_WORDS)[number]) : null;
            // A block aimed at the declared target loses it with the target, and asks instead.
            const hooks =
              target !== null
                ? model.hooks
                : model.hooks.map((hook) => ({
                    ...hook,
                    steps: hook.steps.map((step) =>
                      "target" in step && step.target.kind === "declared" ? { ...step, target: { kind: "ask" as const, words: ONE_WORDS[0] } } : step,
                    ),
                  }));
            onChange({ ...model, target, hooks });
          }}
        >
          <option value="">{t("None")}</option>
          {ONE_WORDS.map((words) => (
            <option key={words} value={words}>
              {t(words)}
            </option>
          ))}
        </select>
      </label>

      {model.hooks.map((hook, index) => (
        <HookEditor
          key={index}
          hook={hook}
          triggers={triggers}
          declared={model.target}
          onChange={(next) => onChange({ ...model, hooks: model.hooks.map((old, at) => (at === index ? next : old)) })}
          onRemove={() => onChange({ ...model, hooks: model.hooks.filter((_, at) => at !== index) })}
        />
      ))}
      {free === undefined ? null : (
        <button
          type="button"
          className="button-secondary"
          data-testid={creatorTestid.addHook}
          onClick={() => onChange({ ...model, hooks: [...model.hooks, { trigger: free, steps: [] }] })}
        >
          {t("Add a trigger")}
        </button>
      )}

      {unit && model.aura !== null ? <AuraEditor aura={model.aura} onChange={(aura) => onChange({ ...model, aura })} /> : null}
      {unit && model.activate !== null ? <ActivateEditor activate={model.activate} onChange={(activate) => onChange({ ...model, activate })} /> : null}
      {unit ? (
        <div className="creator-row">
          {model.aura === null ? (
            <button
              type="button"
              className="button-secondary"
              data-testid={creatorTestid.addAura}
              onClick={() => onChange({ ...model, aura: { attack: 1, health: 0, give: [], to: AURA_WORDS[0] } })}
            >
              {t("Add an aura")}
            </button>
          ) : null}
          {model.activate === null ? (
            <button
              type="button"
              className="button-secondary"
              data-testid={creatorTestid.addActivate}
              onClick={() => onChange({ ...model, activate: { label: t("Activate"), cost: 1, steps: [] } })}
            >
              {t("Add an Activate")}
            </button>
          ) : null}
        </div>
      ) : null}

      {numbers.length === 0 ? null : (
        <table className="creator-numbers">
          <caption>{t("The card's numbers")}</caption>
          <thead>
            <tr>
              <th scope="col">{t("Number")}</th>
              <th scope="col">{t("Base")}</th>
              <th scope="col">{t("Radiant")}</th>
            </tr>
          </thead>
          <tbody>
            {numbers.map((number) => (
              <tr key={number.key}>
                <th scope="row">
                  <code>{`{${number.key}}`}</code>
                </th>
                <td>{number.base}</td>
                <td>
                  <NumberInput
                    value={number.radiant}
                    testid={creatorTestid.numberRadiant}
                    label={t("Radiant value of {key}", { key: number.key })}
                    onChange={(radiant) => onChange({ ...model, radiant: { ...model.radiant, [number.key]: radiant } })}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
