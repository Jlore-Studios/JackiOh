// #2108, R2108000: one empty state for every menu panel, a tankard beside the words and the next step
// where there is one. Nothing moves. Presentation only (CLAUDE.md rule 7).

import type { ReactElement } from "react";

import { followInApp } from "../routes/nav.tsx";

import "./empty.css";

export const emptyStateTestid = { root: "empty-state", mark: "empty-state-mark", action: "empty-state-action" } as const;

/** The next step: a link to another screen (`to`, a `paths` path) or a button on this one. */
export type EmptyStateAction = { label: string; testId?: string } & ({ to: string } | { onPress: () => void });

export type EmptyStateProps = {
  /** What is empty, already through `t()`. */
  text: string;
  /** The words' testid: the one the panel's empty line had. */
  testId?: string;
  action?: EmptyStateAction;
};

function Tankard(): ReactElement {
  return (
    <svg viewBox="0 0 24 24" width="36" height="36" focusable="false" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 7h13" />
      <path d="M5 7v11a2 2 0 0 0 2 2h7a2 2 0 0 0 2-2V7" />
      <path d="M16 10h1.5a2.5 2.5 0 0 1 2.5 2.5v2a2.5 2.5 0 0 1-2.5 2.5H16" />
      <path d="M8.5 10.5v6M12.5 10.5v6" />
    </svg>
  );
}

export function EmptyState({ text, testId, action }: EmptyStateProps): ReactElement {
  const actionId = action?.testId ?? emptyStateTestid.action;
  return (
    <div className="empty-state" data-testid={emptyStateTestid.root}>
      <span className="empty-state__mark" aria-hidden="true" data-testid={emptyStateTestid.mark}>
        <Tankard />
      </span>
      <div className="empty-state__body">
        <p className="empty-state__text" data-testid={testId}>
          {text}
        </p>
        {action === undefined ? null : "to" in action ? (
          <a className="button-secondary empty-state__action" href={action.to} data-testid={actionId} onClick={followInApp(action.to)}>
            {action.label}
          </a>
        ) : (
          <button type="button" className="button-secondary empty-state__action" data-testid={actionId} onClick={action.onPress}>
            {action.label}
          </button>
        )}
      </div>
    </div>
  );
}
