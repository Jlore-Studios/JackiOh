// The dashboard, one page: Account, Holdings and Dev tools, Rating, Leaderboard, News posts, Audit. One
// request at a time (every action button waits while one is out); the status line shows its answer, and
// each answer moves `revision`, so every section reads what the request left.

import { useCallback, useId, useRef, useState, type ReactElement } from "react";

import { messageOf, type AdminApi } from "./api.ts";
import { Audit } from "./Audit.tsx";
import { Holdings } from "./Holdings.tsx";
import { Leaderboard } from "./Leaderboard.tsx";
import { News } from "./News.tsx";
import { Rating } from "./Rating.tsx";
import type { Run, SectionProps } from "./section.ts";
import { testids } from "./testids.ts";
import type { AdminMe } from "./types.ts";

type Status = { kind: "idle" } | { kind: "working" } | { kind: "done"; text: string } | { kind: "failed"; text: string };

function Account({ me, onSignOut }: { me: AdminMe; onSignOut: () => void }): ReactElement {
  const ids = useId();
  return (
    <section data-testid={testids.account} aria-labelledby={`${ids}-heading`}>
      <h2 id={`${ids}-heading`}>Account</h2>
      <dl className="facts">
        <dt>Username</dt>
        <dd data-testid={testids.accountUsername}>{me.username}</dd>
        <dt>Profile</dt>
        <dd data-testid={testids.accountProfileId}>
          <code>{me.profileId}</code>
        </dd>
        <dt>Strict server</dt>
        <dd data-testid={testids.accountStrict}>
          {me.strict ? "Yes: two-step sign-in is required, and a hand-set rating is off the public ladder" : "No"}
        </dd>
        <dt>Two-step sign-in</dt>
        <dd data-testid={testids.accountSecondFactor}>{me.secondFactor ? "Yes" : "No"}</dd>
      </dl>
      <div className="row">
        <button type="button" data-testid={testids.signOut} onClick={onSignOut}>
          Sign out
        </button>
      </div>
    </section>
  );
}

export function Dashboard({ api, me, onSignOut }: { api: AdminApi; me: AdminMe; onSignOut: () => void }): ReactElement {
  const [status, setStatus] = useState<Status>({ kind: "idle" });
  const [revision, setRevision] = useState(0);
  const busy = useRef(false);

  const run = useCallback<Run>((action) => {
    if (busy.current) return;
    busy.current = true;
    setStatus({ kind: "working" });
    action().then(
      (text) => {
        busy.current = false;
        setStatus({ kind: "done", text });
        setRevision((count) => count + 1);
      },
      (cause: unknown) => {
        busy.current = false;
        setStatus({ kind: "failed", text: messageOf(cause) });
      },
    );
  }, []);

  const section: SectionProps = { api, revision, working: status.kind === "working", run };

  return (
    <main data-testid={testids.dashboard}>
      <h1>JackiOh dev dashboard</h1>
      <Account me={me} onSignOut={onSignOut} />
      <Holdings {...section} />
      <Rating {...section} />
      <Leaderboard {...section} />
      <News {...section} />
      <Audit {...section} />
      <p
        className={`status ${status.kind}`}
        data-testid={testids.status}
        data-state={status.kind}
        role="status"
        aria-live="polite"
      >
        {status.kind === "working" ? "Working…" : status.kind === "idle" ? "" : status.text}
      </p>
    </main>
  );
}
