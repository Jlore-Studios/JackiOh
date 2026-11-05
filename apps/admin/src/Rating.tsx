// Rating: the account's own rating on each ladder, set by hand or put back to what its games earned.
// The server holds the range (a refusal says it) and keeps the earned rating while one is hand-set.

import { useId, useState, type ReactElement } from "react";

import { shown, useRead, VOLATILITY_DECIMALS, type SectionProps } from "./section.ts";
import { testids } from "./testids.ts";
import type { AdminRating, Ladder } from "./types.ts";

function earnedText(rating: AdminRating): string {
  if (rating.earned === null) return "None kept";
  const { rating: value, deviation, volatility } = rating.earned;
  return `${shown(value)} (deviation ${shown(deviation)}, volatility ${shown(volatility, VOLATILITY_DECIMALS)})`;
}

export function Rating({ api, revision, working, run }: SectionProps): ReactElement {
  const ids = useId();
  const read = useRead(() => api.rating(), [api, revision]);
  const [typed, setTyped] = useState<Partial<Record<Ladder, string>>>({});

  return (
    <section data-testid={testids.rating} aria-labelledby={`${ids}-heading`}>
      <h2 id={`${ids}-heading`}>Rating</h2>
      <p>On a strict server a hand-set rating is off the public ladder until reset.</p>
      {read.error === null ? null : (
        <p className="error" data-testid={testids.loadError}>
          {read.error}
        </p>
      )}
      {read.value === null ? <p>…</p> : null}
      <div className="cards">
        {(read.value?.ladders ?? []).map((rating) => {
          const { ladder } = rating;
          return (
            <div key={ladder} className="card" data-testid={testids.ratingLadder(ladder)}>
              <h3>{ladder === "constructed" ? "Constructed" : "Random"}</h3>
              <dl className="facts">
                <dt>Rating</dt>
                <dd data-testid={testids.ratingValue(ladder)}>{shown(rating.rating)}</dd>
                <dt>Deviation</dt>
                <dd data-testid={testids.ratingDeviation(ladder)}>{shown(rating.deviation)}</dd>
                <dt>Volatility</dt>
                <dd data-testid={testids.ratingVolatility(ladder)}>{shown(rating.volatility, VOLATILITY_DECIMALS)}</dd>
                <dt>Hand-set</dt>
                <dd data-testid={testids.ratingHandSet(ladder)}>{rating.handSet ? "Yes" : "No"}</dd>
                <dt>Earned</dt>
                <dd data-testid={testids.ratingEarned(ladder)}>{earnedText(rating)}</dd>
              </dl>
              <label htmlFor={`${ids}-${ladder}`}>New rating</label>
              <div className="row">
                <input
                  id={`${ids}-${ladder}`}
                  data-testid={testids.ratingInput(ladder)}
                  type="number"
                  inputMode="decimal"
                  value={typed[ladder] ?? ""}
                  disabled={working}
                  onChange={(event) => {
                    const text = event.target.value;
                    setTyped((previous) => ({ ...previous, [ladder]: text }));
                  }}
                />
                <button
                  type="button"
                  data-testid={testids.ratingSet(ladder)}
                  disabled={working}
                  onClick={() => {
                    run(async () => {
                      await api.setRating(ladder, Number(typed[ladder] ?? ""));
                      return "Done.";
                    });
                  }}
                >
                  Set
                </button>
                {rating.handSet ? (
                  <button
                    type="button"
                    data-testid={testids.ratingReset(ladder)}
                    disabled={working}
                    onClick={() => {
                      run(async () => {
                        await api.resetRating(ladder);
                        return "Done.";
                      });
                    }}
                  >
                    Reset to earned
                  </button>
                ) : null}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
