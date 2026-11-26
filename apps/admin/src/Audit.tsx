// Audit: every admin action the server recorded, newest first: who, which route, what was asked, and the
// state before and after, each as `JSON.stringify` text. A filter row narrows the list to one acting
// account, one route, or one time window, and Older appends the next page past the shown list's
// `nextBeforeId`, read with the filter that produced the shown rows; an Older answer overtaken by a
// newer first page is dropped.

import { useEffect, useId, useRef, useState, type ReactElement } from "react";

import { messageOf } from "./api.ts";
import { json, when, type SectionProps } from "./section.ts";
import { testids } from "./testids.ts";
import type { AuditAction } from "./types.ts";

/** A `datetime-local` value as the audit's epoch-millisecond bound, or no bound when it names none. */
function toMs(value: string): number | undefined {
  const trimmed = value.trim();
  if (trimmed === "") return undefined;
  const parsed = Date.parse(trimmed);
  return Number.isNaN(parsed) ? undefined : parsed;
}

export function Audit({ api, revision }: SectionProps): ReactElement {
  const ids = useId();
  const [asked, setAsked] = useState(0);
  const [profileInput, setProfileInput] = useState("");
  const [routeInput, setRouteInput] = useState("");
  const [sinceInput, setSinceInput] = useState("");
  const [untilInput, setUntilInput] = useState("");
  const [applied, setApplied] = useState<{ profileId: string; route: string; since?: number; until?: number }>({
    profileId: "",
    route: "",
  });
  const [actions, setActions] = useState<AuditAction[] | null>(null);
  const [next, setNext] = useState<number | null>(null);
  // The filter the shown rows were read with: Older continues this list, not the inputs.
  const [readFilter, setReadFilter] = useState(applied);
  const generation = useRef(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // The first page of what the filters name; Refresh and any dashboard write ask again.
  useEffect(() => {
    let live = true;
    generation.current += 1;
    const read = applied;
    setLoading(true);
    api
      .audit({
        profileId: read.profileId || undefined,
        route: read.route || undefined,
        since: read.since,
        until: read.until,
      })
      .then(
        (page) => {
          if (!live) return;
          setActions(page.actions);
          setNext(page.nextBeforeId);
          setReadFilter(read);
          setError(null);
          setLoading(false);
        },
        (cause: unknown) => {
          if (!live) return;
          setError(messageOf(cause));
          setLoading(false);
        },
      );
    return () => {
      live = false;
    };
  }, [api, revision, asked, applied]);

  const older = (): void => {
    if (next === null || loading) return;
    const my = generation.current;
    const query = readFilter;
    const cursor = next;
    setLoading(true);
    api
      .audit({
        beforeId: cursor,
        profileId: query.profileId || undefined,
        route: query.route || undefined,
        since: query.since,
        until: query.until,
      })
      .then(
        (page) => {
          if (generation.current !== my) return;
          setActions((previous) => [...(previous ?? []), ...page.actions]);
          setNext(page.nextBeforeId);
          setError(null);
          setLoading(false);
        },
        (cause: unknown) => {
          if (generation.current !== my) return;
          setError(messageOf(cause));
          setLoading(false);
        },
      );
  };

  return (
    <section data-testid={testids.audit} aria-labelledby={`${ids}-heading`}>
      <h2 id={`${ids}-heading`}>Audit</h2>
      <div className="row">
        <button
          type="button"
          data-testid={testids.auditRefresh}
          onClick={() => {
            setAsked((count) => count + 1);
          }}
        >
          Refresh
        </button>
      </div>
      <div className="row">
        <label htmlFor={`${ids}-profile`}>Account</label>
        <input
          id={`${ids}-profile`}
          data-testid={testids.auditProfile}
          value={profileInput}
          onChange={(event) => {
            setProfileInput(event.target.value);
          }}
        />
        <label htmlFor={`${ids}-route`}>Route</label>
        <input
          id={`${ids}-route`}
          data-testid={testids.auditRoute}
          value={routeInput}
          onChange={(event) => {
            setRouteInput(event.target.value);
          }}
        />
        <label htmlFor={`${ids}-since`}>Since</label>
        <input
          id={`${ids}-since`}
          type="datetime-local"
          data-testid={testids.auditSince}
          value={sinceInput}
          onChange={(event) => {
            setSinceInput(event.target.value);
          }}
        />
        <label htmlFor={`${ids}-until`}>Until</label>
        <input
          id={`${ids}-until`}
          type="datetime-local"
          data-testid={testids.auditUntil}
          value={untilInput}
          onChange={(event) => {
            setUntilInput(event.target.value);
          }}
        />
        <button
          type="button"
          data-testid={testids.auditApply}
          onClick={() => {
            setApplied({ profileId: profileInput.trim(), route: routeInput.trim(), since: toMs(sinceInput), until: toMs(untilInput) });
          }}
        >
          Apply
        </button>
        <button
          type="button"
          data-testid={testids.auditClear}
          onClick={() => {
            setProfileInput("");
            setRouteInput("");
            setSinceInput("");
            setUntilInput("");
            setApplied({ profileId: "", route: "" });
          }}
        >
          Clear
        </button>
      </div>
      {error === null ? null : (
        <p className="error" data-testid={testids.loadError}>
          {error}
        </p>
      )}
      {actions === null ? (
        <p>…</p>
      ) : actions.length === 0 ? (
        <p>No actions yet.</p>
      ) : (
        <div className="scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">When</th>
                <th scope="col">Who</th>
                <th scope="col">Route</th>
                <th scope="col">Request</th>
                <th scope="col">Before</th>
                <th scope="col">After</th>
              </tr>
            </thead>
            <tbody>
              {actions.map((action) => (
                <tr key={action.id} data-testid={testids.auditRow} data-action-id={String(action.id)}>
                  <td>{when(action.at)}</td>
                  <td>{action.username === "" ? action.profileId : action.username}</td>
                  <td>
                    <code>{action.route}</code>
                  </td>
                  <td>
                    <code className="json">{json(action.request)}</code>
                  </td>
                  <td>
                    <code className="json">{json(action.before)}</code>
                  </td>
                  <td>
                    <code className="json">{json(action.after)}</code>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {next === null ? null : (
        <div className="row">
          <button type="button" data-testid={testids.auditOlder} disabled={loading} onClick={older}>
            Older
          </button>
        </div>
      )}
    </section>
  );
}
