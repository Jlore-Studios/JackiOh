// A line an inspect overlay prints over the face it shows, in words the caller took from the view
// (R371: your own face-down trap's "Face down — your opponent can't see this card"). It sits on
// the face's top edge with the struck-through eye, so the mark does not rest on colour alone.

import type { ReactElement } from "react";
import { Icon } from "../icons.tsx";
import { INSPECT_NOTE } from "./testids.ts";

export function InspectNote({ note }: { note: string | undefined }): ReactElement | null {
  if (note === undefined || note === "") return null;
  return (
    <span className="inspect-note" data-testid={INSPECT_NOTE}>
      <Icon name="eyeOff" />
      <span className="inspect-note-text">{note}</span>
    </span>
  );
}
