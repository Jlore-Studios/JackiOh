// R2031000: `LeanNewest.tsx`'s "More cards from" picker for a component the shell mounts on every
// screen (`together/`), screens that may paint before the engine's WebAssembly module is in (#778).
// The picker states the share a pick asks for, an AI number read from WASM (CLAUDE.md rule 9), so this
// shows nothing until the module and the picker's code are in: whoever shows it is about to start a
// game, which needs the engine anyway. If the module never arrives the device's pick (`playLean.ts`)
// still travels.

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
