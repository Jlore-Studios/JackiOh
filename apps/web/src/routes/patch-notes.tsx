// `/patch-notes`: the public Patch notes page (brief B4.2 item 5, R388, R507, R724000). Public, like the
// landing page and the privacy policy: the catalog and its history are public (§5.1), the data is
// static files in the bundle's own chunks (patches/source.ts), and no account or server is needed.
// The page is read-only and decides nothing (CLAUDE.md rule 7).

import type { ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import { PatchNotes } from "../patches/PatchNotes.tsx";
import { patchTestid } from "../patches/testids.ts";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";

export default function PatchNotesRoute(): ReactElement {
  return (
    <div className="app-shell tavern patch-notes-screen one-screen" data-testid={patchTestid.screen}>
      <BackLink />
      <main className="patch-notes one-screen__body" aria-labelledby="patch-notes-title">
        <header className="patch-notes__head">
          <h1 id="patch-notes-title" className="patch-notes__title">
            {t("Patch notes")}
          </h1>
        </header>
        <PatchNotes />
      </main>
    </div>
  );
}
