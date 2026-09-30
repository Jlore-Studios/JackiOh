// `/patch-notes`: the public Patch notes page (brief B4.2 item 5, R388, R507). Public, like the
// landing page and the privacy policy: the catalog and its history are public (§5.1), the data is
// static files in the bundle's own chunks (patches/source.ts), and no account or server is needed.
//
// The page is read-only and decides nothing (CLAUDE.md rule 7). It sits in the tavern like the other
// screens on the way in (auth/tavern.css), with the corner's Back and the settings gear.

import type { ReactElement } from "react";

import { PatchNotes } from "../patches/PatchNotes.tsx";
import { patchTestid } from "../patches/testids.ts";
import { BackLink } from "./nav.tsx";

import "../auth/tavern.css";

export default function PatchNotesRoute(): ReactElement {
  return (
    <div className="app-shell tavern patch-notes-screen" data-testid={patchTestid.screen}>
      <BackLink />
      <main className="patch-notes" aria-labelledby="patch-notes-title">
        <header className="patch-notes__head">
          <h1 id="patch-notes-title" className="patch-notes__title">
            Patch notes
          </h1>
          <p className="patch-notes__intro">
            Every change to JackiOh&rsquo;s cards, newest first. Open a patch to see the cards it touched, and a card to
            see its whole history.
          </p>
        </header>
        <PatchNotes />
      </main>
    </div>
  );
}
