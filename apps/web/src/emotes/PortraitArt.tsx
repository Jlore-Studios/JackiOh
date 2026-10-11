// A portrait's art with its vivid layers (R1332): the `CardArt` oval (R641) under a ground of the
// portrait's own lit from the upper left, a light that breathes, a few drifting motes and a rim
// light. The same element draws on the board's hero, in the hero's inspect view (R1330) and in the
// deck builder's picker, so a portrait looks alike wherever it is. The picture stays the card's own
// art and everything laid over it is a pure function of the portrait's id (`portraitLook.ts`);
// `portrait.css` sets the colours, the idle and Reduce Motion's stillness.

import type { CSSProperties, ReactElement } from "react";

import type { PortraitId } from "@jackioh/shared";

import { CardArt } from "../cards/art/CardArt.tsx";
import { PORTRAIT_BREATH_MS, PORTRAIT_MOTE_DRIFT_MS } from "./config.ts";
import { PORTRAIT_LOOKS, PORTRAIT_MOTES } from "./portraitLook.ts";
import { PORTRAIT_DEFS } from "./portraits.ts";
import "./portrait.css";

export function PortraitArt({
  portrait,
  className,
}: {
  portrait: PortraitId;
  className?: string;
}): ReactElement {
  const { defId, def } = PORTRAIT_DEFS[portrait];
  const look = PORTRAIT_LOOKS[portrait];
  const style = {
    "--portrait-ground-1": look.ground[0],
    "--portrait-ground-2": look.ground[1],
    "--portrait-light": look.light,
    "--portrait-rim": look.rim,
    "--portrait-mote": look.mote,
    "--portrait-breath": `${PORTRAIT_BREATH_MS}ms`,
    "--portrait-drift": `${PORTRAIT_MOTE_DRIFT_MS}ms`,
  } as CSSProperties;
  return (
    <span
      className={className === undefined ? "portrait-art" : `portrait-art ${className}`}
      data-portrait-art={portrait}
      aria-hidden="true"
      style={style}
    >
      <CardArt
        defId={defId}
        radiant={false}
        tags={def.tags}
        type={def.type}
        shape="oval"
        name={def.name}
      />
      <span className="portrait-art-light" />
      <span className="portrait-art-breath" />
      <span className="portrait-art-motes">
        {PORTRAIT_MOTES[portrait].map((mote, index) => (
          <span
            // The motes are a fixed list per portrait, so the index is a stable key.
            key={index}
            className="portrait-art-mote"
            style={
              {
                "--mote-x": `${mote.x}%`,
                "--mote-y": `${mote.y}%`,
                "--mote-size": `${mote.size}%`,
                "--mote-delay": `-${mote.delayMs}ms`,
              } as CSSProperties
            }
          />
        ))}
      </span>
      <span className="portrait-art-rim" />
    </span>
  );
}
