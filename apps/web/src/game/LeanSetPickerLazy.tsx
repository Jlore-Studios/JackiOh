// R2031000: `LeanNewest.tsx`'s "More cards from" picker for a component the shell mounts on every
// screen (the challenge dialog and toast, `together/`), screens that may paint before the engine's
// WebAssembly module is in (#778). The picker states the share of the deck a pick asks for, an AI
// number read from WASM (CLAUDE.md rule 9), so this loads the module and the picker's code, and shows
// nothing until both are in. Whoever shows it is about to start a game, which needs the engine anyway.
// If the module never arrives the picker stays away; the device's pick (`playLean.ts`) still travels.

import { Suspense, lazy, type ComponentType, type ReactElement } from "react";

import { loadWasm } from "../wasm/index.ts";
import type { LeanSetPickerProps } from "./LeanNewest.tsx";

const Picker = lazy<ComponentType<LeanSetPickerProps>>(() =>
  Promise.all([loadWasm(), import("./LeanNewest.tsx")]).then(
    ([, module]) => ({ default: module.LeanSetPicker }),
    () => ({ default: () => null }),
  ),
);

export function LeanSetPickerLazy(props: LeanSetPickerProps): ReactElement {
  return (
    <Suspense fallback={null}>
      <Picker {...props} />
    </Suspense>
  );
}
