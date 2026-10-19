// R1297000: the ambient backdrop behind the viewer's half of the board, one of the registry's
// `field` cosmetics (cards/cosmetics.ts). A layer of BACKDROP_PARTICLE_MAX particles whose
// positions and delays are a fixed function of their index (nothing is random); `table.css` draws
// the field's look from `data-backdrop` and moves the particles only where motion is allowed. It
// is decoration: aria-hidden, input-transparent and behind everything (CLAUDE.md rule 7: nothing
// here is state).

import type { CSSProperties, ReactElement } from "react";

import type { CosmeticId } from "../cards/cosmetics.ts";
import {
  BACKDROP_PARTICLE_DELAY_STEP_MS,
  BACKDROP_PARTICLE_MAX,
  BACKDROP_PARTICLE_X_STEP_PCT,
  BACKDROP_PARTICLE_Y_STEP_PCT,
} from "./config.ts";

type ParticleStyle = CSSProperties & { "--x": string; "--y": string; "--delay": string };

function particleStyle(index: number): ParticleStyle {
  return {
    "--x": `${String((index * BACKDROP_PARTICLE_X_STEP_PCT) % 100)}%`,
    "--y": `${String((index * BACKDROP_PARTICLE_Y_STEP_PCT) % 100)}%`,
    "--delay": `${String(index * BACKDROP_PARTICLE_DELAY_STEP_MS)}ms`,
  };
}

export default function Backdrop({ field }: { field: CosmeticId }): ReactElement {
  return (
    <div className="board-backdrop" data-backdrop={field} aria-hidden="true">
      {Array.from({ length: BACKDROP_PARTICLE_MAX }, (_, index) => (
        <span key={index} className="board-backdrop-particle" style={particleStyle(index)} />
      ))}
    </div>
  );
}
