// Audit: every admin action the server recorded, newest first: who, which route, what was asked, and the
// state before and after, each as `JSON.stringify` text.

import { useId, useState, type ReactElement } from "react";

import { json, useRead, when, type SectionProps } from "./section.ts";
import { testids } from "./testids.ts";

export function Audit({ api, revision }: SectionProps): ReactElement {
  const ids = useId();
  const [asked, setAsked] = useState(0);
  const read = useRead(() => api.audit(), [api, revision, asked]);

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
      {read.error === null ? null : (
        <p className="error" data-testid={testids.loadError}>
          {read.error}
        </p>
      )}
      {read.value === null ? (
        <p>…</p>
      ) : read.value.actions.length === 0 ? (
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
              {read.value.actions.map((action) => (
                <tr key={action.id} data-testid={testids.auditRow} data-action-id={String(action.id)}>
                  <td>{when(action.at)}</td>
                  <td>{action.username === "" ? action.profileId : action.username}</td>
                  <td>
                    <code>{action.route}</code>
                  </td>
                  <td>
                    <code>{json(action.request)}</code>
                  </td>
                  <td>
                    <code>{json(action.before)}</code>
                  </td>
                  <td>
                    <code>{json(action.after)}</code>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
