// A player's username, as every screen shows one (R1436). A profile id keys a list and names a
// request but is never shown, nor anything made from it; a player is shown by the username the server
// gave them (`Max`, or `Max#3` when the base was taken), always through this one component.
//
// ISOLATED: a name written right to left would reorder the text around it, or pull its own `#n` into
// its run. So the whole name sits in a left-to-right `<bdi>`, the base in an inner `dir="auto"` one and
// the tag apart. The full name rides on `title`, since a long one is cut with an ellipsis
// (`username.css`) at the base's own end, never in the tag.
//
// A server from before usernames sends no `username` (the client can go out ahead of the server):
// the row then shows no name rather than the screen failing to load.

import type { ReactElement } from "react";

import "./username.css";

/** A tag is what follows the last `#`, when that is all ASCII digits and a base stands before it. */
const TAG = /^[0-9]+$/;

/** `Max#3` -> `{ base: "Max", tag: "3" }`; a name with no tag keeps all of itself as the base. */
export function splitUsername(name: string): { base: string; tag: string | null } {
  const at = name.lastIndexOf("#");
  if (at <= 0 || !TAG.test(name.slice(at + 1))) return { base: name, tag: null };
  return { base: name.slice(0, at), tag: name.slice(at + 1) };
}

export default function Username({ name }: { name: string | undefined }): ReactElement | null {
  if (typeof name !== "string") return null;
  const { base, tag } = splitUsername(name);
  return (
    <bdi className="username" dir="ltr" title={name}>
      <bdi className="username__base" dir="auto">
        {base}
      </bdi>
      {tag === null ? null : <span className="username__tag">#{tag}</span>}
    </bdi>
  );
}
