// #3633: the custom properties the board's animations read off the game root (Game.tsx's, and the
// effects layer's `--anim-squeeze`), written onto the elements that read them instead of onto the root.
// Set on the root, each one restyled every element of the game whenever an entry began or ended: Blink
// computes a descendant's style again when an inherited custom property above it changes, whether it
// reads the property or not, and that was eighty to a hundred milliseconds a time on a phone throttled
// 4x (R3351000). A rule scoped to the root names each variable's readers, so a change restyles them
// alone, and each variable has a sheet of its own: Blink restyles what every rule of a changed sheet
// names, so a damage pop's scale changing restyles the pops and no card.
//
// `ROOT_VARS` is where each variable is read: the elements that read it, and for some the root's own
// `::after`. `--anim-squeeze` is read on its readers' descendants too (`[data-animating="x"] .tag`), so
// it inherits from them. Every other one is read on the reader itself and is registered not to inherit
// (animations.css, `@property`), so setting it restyles no reader's subtree. rootVars.test.ts holds
// every stylesheet's reads to these readers.

import { useInsertionEffect, useLayoutEffect, useRef, type RefObject } from "react";

/** Each variable the effects layer and Game write per entry: who reads it, and whether it inherits. */
export const ROOT_VARS: Readonly<Record<string, { inherits: boolean; readers: readonly string[]; rootAfter?: true }>> = {
  "--anim-squeeze": {
    inherits: true,
    readers: ["[data-animating]", ".burn-notice", ".pile-notice", ".prompt-scrim", ".result-scrim", '[data-testid="result-overlay"]', ".damage-pop", ".heal-pop", ".loss-pop"],
  },
  "--impact-number-scale": { inherits: false, readers: [".damage-pop"] },
  "--impact-number-linger": { inherits: false, readers: [".damage-pop"] },
  "--slam-anticipation": { inherits: false, readers: ["[data-animating]", ".card"], rootAfter: true },
  "--slam-land-ms": { inherits: false, readers: [".card"], rootAfter: true },
  "--set-piece-hold": { inherits: false, readers: ["[data-animating]", ".card", ".zone", ".field"], rootAfter: true },
  "--set-piece-skip-delay": { inherits: false, readers: [".set-piece-skip"] },
  "--takeover-land-ms": { inherits: false, readers: [".card"] },
  "--riffle-ms": { inherits: false, readers: [".pile-riffle-leaf"] },
  "--riffle-delay": { inherits: false, readers: [".pile-riffle-leaf"] },
  "--deal-ms": { inherits: false, readers: [".hand-slot"] },
};

/** The attribute that names a root's scope; its value is the root's own. */
export const ROOT_VARS_ATTRIBUTE = "data-root-vars";

export type RootVars = {
  /** Sets a variable for the root's readers; an empty value removes it, as `style.setProperty` does. */
  set(name: string, value: string): void;
  remove(name: string): void;
  /** Lets go of the root: the last owner's release removes its rules and its scope. */
  release(): void;
};

type Scope = { id: string; root: HTMLElement; sheets: Map<string, HTMLStyleElement>; values: Map<string, string>; owners: number };

const scopes = new WeakMap<Element, Scope>();
let nextScope = 0;

/** The rule that gives `name` its `value` on its readers under the scope `id` (and on the root's `::after`). */
export function rootVarCss(id: string, name: string, value: string): string {
  const where = ROOT_VARS[name];
  if (where === undefined) throw new Error(`${name} is no root variable (rootVars.ts, ROOT_VARS)`);
  const scope = `[${ROOT_VARS_ATTRIBUTE}="${id}"]`;
  const rule = `${scope} :is(${where.readers.join(", ")}) { ${name}: ${value}; }`;
  return where.rootAfter === true ? `${rule}\n${scope}::after { ${name}: ${value}; }` : rule;
}

function write(scope: Scope, name: string, value: string | undefined): void {
  let sheet = scope.sheets.get(name);
  if (sheet === undefined) {
    if (value === undefined) return;
    sheet = scope.root.ownerDocument.createElement("style");
    sheet.setAttribute(ROOT_VARS_ATTRIBUTE, `${scope.id} ${name}`);
    scope.root.ownerDocument.head.appendChild(sheet);
    scope.sheets.set(name, sheet);
  }
  sheet.textContent = value === undefined ? "" : rootVarCss(scope.id, name, value);
}

/** The variables of `root`, shared by every owner of the same root (Game and its effects layer). */
export function rootVars(root: HTMLElement): RootVars {
  let scope = scopes.get(root);
  if (scope === undefined) {
    nextScope += 1;
    scope = { id: `g${String(nextScope)}`, root, sheets: new Map(), values: new Map(), owners: 0 };
    root.setAttribute(ROOT_VARS_ATTRIBUTE, scope.id);
    scopes.set(root, scope);
  }
  const own = scope;
  own.owners += 1;
  let released = false;
  const remove = (name: string): void => {
    if (!own.values.delete(name)) return;
    write(own, name, undefined);
  };
  return {
    set(name, value) {
      if (value === "") remove(name);
      else if (own.values.get(name) !== value) {
        own.values.set(name, value);
        write(own, name, value);
      }
    },
    remove,
    release() {
      if (released) return;
      released = true;
      own.owners -= 1;
      if (own.owners > 0) return;
      for (const sheet of own.sheets.values()) sheet.remove();
      root.removeAttribute(ROOT_VARS_ATTRIBUTE);
      scopes.delete(root);
    },
  };
}

/** A variable's value on `root` as its readers get it, or "" when it is unset (tests, and the dev gallery). */
export function readRootVar(root: Element, name: string): string {
  return scopes.get(root)?.values.get(name) ?? "";
}

/** Owns the variables of the element `root` holds while it is mounted; the handle is null until then. */
export function useRootVars(root: RefObject<HTMLElement | null>): RefObject<RootVars | null> {
  const handle = useRef<RootVars | null>(null);
  useLayoutEffect(() => {
    const element = root.current;
    if (element === null) return undefined;
    const owned = rootVars(element);
    handle.current = owned;
    return () => {
      owned.release();
      if (handle.current === owned) handle.current = null;
    };
  }, [root]);
  return handle;
}

function sameValues(a: Readonly<Record<string, string>>, b: Readonly<Record<string, string>>): boolean {
  const keys = Object.keys(a);
  return keys.length === Object.keys(b).length && keys.every((key) => b[key] === a[key]);
}

/**
 * Writes `values` for the readers in the commit that first renders them, and removes them in the one
 * that drops them; a render that leaves them as they were writes nothing, so none is dropped mid-motion.
 * An insertion effect, so they change with the DOM of the same commit, before any layout effect reads
 * the page: a later write would restyle what that read had just restyled. The layout effect writes what
 * the first commit rendered, which comes before the root is owned.
 */
export function useRootVarValues(handle: RefObject<RootVars | null>, values: Readonly<Record<string, string>>): void {
  const kept = useRef(values);
  if (!sameValues(kept.current, values)) kept.current = values;
  const current = kept.current;
  useInsertionEffect(() => {
    for (const [name, value] of Object.entries(current)) handle.current?.set(name, value);
    return () => {
      for (const name of Object.keys(current)) handle.current?.remove(name);
    };
  }, [handle, current]);
  useLayoutEffect(() => {
    for (const [name, value] of Object.entries(current)) handle.current?.set(name, value);
  }, [handle, current]);
}
