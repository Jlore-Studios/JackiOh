// R4680000: the catalogs the Experimental tab's Versions view draws its old versions from, each loaded only once a row
// on screen, or a version's drill-down, needs it: the patch history the browser carries (`patches/source.ts`, one
// lazily loaded file per shipped version, as the Almanac's History and /patch-notes read it), and for a version that
// history does not hold, `GET /api/catalog/:version`. Never the whole history at once.

import { useCallback, useRef, useState } from "react";

import { getCatalogAt } from "../net/api.ts";
import { usePatchSource } from "../patches/context.tsx";
import type { PatchSource, Snapshot } from "../patches/source.ts";

/** R4680000: the catalog a shipped version left: the browser's own history's file, else the server's. */
async function snapshotAt(source: PatchSource, version: string): Promise<Snapshot> {
  return (await source.snapshot(version)) ?? (await getCatalogAt(version)).defs;
}

/** R4680000: each version's catalog once it has loaded, null where it failed; a version not in it is not loaded yet. */
export type VersionSnapshots = {
  readonly snapshots: ReadonlyMap<string, Snapshot | null>;
  /** Loads a version's catalog unless it is loaded or loading; one that failed is asked again. */
  readonly need: (version: string) => void;
};

export function useVersionSnapshots(): VersionSnapshots {
  const source = usePatchSource();
  const [snapshots, setSnapshots] = useState<ReadonlyMap<string, Snapshot | null>>(new Map());
  // Asked and not failed: loading or loaded. A failure leaves it, so a later need asks again.
  const asked = useRef(new Set<string>());
  const need = useCallback(
    (version: string) => {
      if (asked.current.has(version)) return;
      asked.current.add(version);
      const settle = (snapshot: Snapshot | null): void => {
        setSnapshots((was) => new Map(was).set(version, snapshot));
      };
      snapshotAt(source, version).then(settle, (error: unknown) => {
        // Kept for whoever opens the console; the row or the drill-down says it didn't load.
        console.error(`The catalog of ${version} did not load`, error);
        asked.current.delete(version);
        settle(null);
      });
    },
    [source],
  );
  return { snapshots, need };
}
