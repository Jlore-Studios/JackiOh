// Audit: every admin action the server recorded, newest first: who, which route, what was asked, and the
// state before and after, each as `JSON.stringify` text. A filter row narrows the list to one acting
// account or one route, and Older appends the next page past it.

import { useEffect, useId, useState, type ReactElement } from "react";

import { messageOf } from "./api.ts";
import { json, when, type SectionProps } from "./section.ts";
import { testids } from "./testids.ts";
import type { AuditAction } from "./types.ts";

export function Audit({ api, revision }: SectionProps): ReactElement {
  const ids = useId();
  const [asked, setAsked] = useState(0);
  const [profileInput, setProfileInput] = useState("");
  const [routeInput, setRouteInput] = useState("");
  const [applied, setApplied] = useState({ profileId: "", route: "" });
  const [actions, setActions] = useState<AuditAction[] | null>(null);
  const [next, setNext] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // The first page of what the filters name; Refresh and any dashboard write ask again.
  useEffect(() => {
    let live = true;
    setLoading(true);
    api.audit({ profileId: applied.profileId || undefined, route: applied.route || undefined }).then(
      (page) => {
        if (!live) return;
        setActions(page.actions);
        setNext(page.nextBeforeId);
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
    const cursor = next;
    setLoading(true);
    api
      .audit({ beforeId: cursor, profileId: applied.profileId || undefined, route: applied.route || undefined })
      .then(
        (page) => {
          setActions((previous) => [...(previous ?? []), ...page.actions]);
          setNext(page.nextBeforeId);
          setError(null);
          setLoading(false);
        },
        (cause: unknown) => {
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
        <button
          type="button"
          data-testid={testids.auditApply}
          onClick={() => {
            setApplied({ profileId: profileInput.trim(), route: routeInput.trim() });
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
