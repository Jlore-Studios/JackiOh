// A player's username, as every screen shows one (R1436).
//
// IDS UNDERNEATH, USERNAMES ON SCREEN. A profile id keys a list and names a request; it is never
// shown, nor anything made from it. A player is shown by the username the server gave them: a base
// name and, when the base was taken, a numeric tag (`Max`, `Max#3`). Every screen that names a player
// renders it through this one component.
//
// ISOLATED. A username can be written right to left (Arabic, Hebrew), and an unisolated one reorders
// the text around it ("Max is taken, so you'd be محمد#2" would scramble), or pulls its own `#n` into
// its run. So the whole name sits in a `<bdi>` laid out left to right, the base in an inner `<bdi>`
// whose direction is its own (`dir="auto"`), and the tag after it as an element of its own. The
// full name rides on `title`, since a long one is cut with an ellipsis (`username.css`).

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

export default function Username({ name }: { name: string }): ReactElement {
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
