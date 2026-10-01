// The patch source a screen reads (source.ts), and the hook that loads from it.
//
// The real source is the default, so the deck builder's detail view and the Patch notes page need
// no provider; a test wraps what it renders in `PatchSourceProvider` with `sourceFromData` fixtures
// (or `EMPTY_PATCH_SOURCE`), which is the only way anything but the real files reaches a screen.

import { createContext, useContext, useEffect, useState, type ReactElement, type ReactNode } from "react";

import { realPatchSource, type PatchSource } from "./source.ts";

const PatchSourceContext = createContext<PatchSource>(realPatchSource);

export function PatchSourceProvider({ source, children }: { source: PatchSource; children: ReactNode }): ReactElement {
  return <PatchSourceContext.Provider value={source}>{children}</PatchSourceContext.Provider>;
}

/** The closest provider's source, else the real one. */
export function usePatchSource(): PatchSource {
  return useContext(PatchSourceContext);
}

/** An async read's state: in flight, done, or failed with a way to ask again. */
export type Loaded<T> =
  | { readonly status: "loading" }
  | { readonly status: "ready"; readonly value: T }
  | { readonly status: "error"; readonly retry: () => void };

type Settled<T> = { key: string; attempt: number } & ({ ok: true; value: T } | { ok: false });

/**
 * Runs `load` for `key` (again whenever `key` changes, or after `retry`) and reports where it is.
 * A read that settles after its key has moved on is dropped, so a quick change never shows the
 * answer to an older question.
 */
export function useLoaded<T>(key: string, load: () => Promise<T>): Loaded<T> {
  const [attempt, setAttempt] = useState(0);
  const [settled, setSettled] = useState<Settled<T> | null>(null);

  useEffect(() => {
    let live = true;
    load().then(
      (value) => {
        if (live) setSettled({ key, attempt, ok: true, value });
      },
      (error: unknown) => {
        // Kept for whoever opens the console; the screen says it didn't load and offers a retry.
        console.error("The patch history did not load", error);
        if (live) setSettled({ key, attempt, ok: false });
      },
    );
    return () => {
      live = false;
    };
    // `load` is the caller's closure over the same `key`; a new closure alone is not a new question.
  }, [key, attempt]);

  if (settled === null || settled.key !== key || settled.attempt !== attempt) return { status: "loading" };
  if (settled.ok) return { status: "ready", value: settled.value };
  return {
    status: "error",
    retry: () => {
      setAttempt((count) => count + 1);
    },
  };
}
