// Woozy X (Stinkyscape+ E5, issue #3548, R3548000–R3548001): the owner's-view-only blur.
//
// A stamped card may blur one of its words on its owner's view alone: one word of its name, its
// cost, its attack, its health, or one keyword kind it carries. The engine decides what blurs, from
// a stream of the game's seed and the card's instance, so a reload or a replay repeats it; the
// client renders the blur and never changes a rule (CLAUDE.md rule 7).

import { createContext, Fragment, useContext, type ReactElement, type ReactNode } from "react";

import type { WoozyBlur } from "@jackioh/shared";

const WoozyBlurContext = createContext<WoozyBlur | null>(null);

export function useWoozyBlur(): WoozyBlur | null {
  return useContext(WoozyBlurContext);
}

export function WoozyBlurProvider({
  blur,
  children,
}: {
  blur: WoozyBlur | null | undefined;
  children: ReactNode;
}): ReactElement {
  return <WoozyBlurContext.Provider value={blur ?? null}>{children}</WoozyBlurContext.Provider>;
}

/** R3548001: the card's name with the word the stamp blurs wrapped in `.woozy-blur`. */
export function WoozyName({ name }: { name: string }): ReactElement | string {
  const blur = useWoozyBlur();
  if (blur?.kind !== "name") return name;
  const words = name.split(" ");
  if (words.length === 0) return name;
  const at = blur.word % words.length;
  return (
    <>
      {words.map((word, index) => (
        <Fragment key={index}>
          {index > 0 ? " " : null}
          {index === at ? <span className="woozy-blur">{word}</span> : word}
        </Fragment>
      ))}
    </>
  );
}
