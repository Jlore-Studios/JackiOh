// #2116, R2116000: one lazy screen's `import()`, so a prefetch and the route make one request.
// `Screen` exists because a plain `lazy()` shows the fallback even over a resolved import. Only code is
// fetched, never game state (CLAUDE.md rule 7).
//
// This lives apart from `chunks.tsx` so a non-route chunk (the Friends drawer, #4444) can share the
// pattern without importing the route map: `chunks.tsx` reaches `dev/hotseat.tsx`, whose
// `window.__jackioh` declaration would collide with `e2e/support/types.ts` in the component specs.

import { lazy, useState, type ComponentType, type ReactElement } from "react";

type ScreenModule<P> = { default: ComponentType<P> };

export type RouteChunk<P extends object> = {
  /** One import, shared; a failed one is forgotten, so the next call asks again. */
  load: () => Promise<ScreenModule<P>>;
  /** Drawn at once when its code is in, else through `lazy()`. */
  Screen: (props: P) => ReactElement;
};

export function routeChunk<P extends object>(importScreen: () => Promise<ScreenModule<P>>): RouteChunk<P> {
  let pending: Promise<ScreenModule<P>> | null = null;
  let arrived: ComponentType<P> | null = null;
  const load = (): Promise<ScreenModule<P>> => {
    pending ??= importScreen().then(
      (module) => {
        arrived = module.default;
        return module;
      },
      (cause: unknown) => {
        pending = null;
        throw cause;
      },
    );
    return pending;
  };
  const Lazy = lazy(load);
  function Screen(props: P): ReactElement {
    // Chosen once per mount: a screen that swapped from `Lazy` to its module mid-life would remount.
    const [Shown] = useState<ComponentType<P>>(() => arrived ?? Lazy);
    return <Shown {...props} />;
  }
  return { load, Screen };
}
