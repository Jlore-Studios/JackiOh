// `auth/Username.tsx`: every screen draws a player's name through it (R1436), isolated so a name
// written right to left never reorders the text around it, nor its own `#n`.

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import Username, { splitUsername } from "./Username.tsx";

afterEach(() => {
  cleanup();
});

describe("R1436 a username on screen", () => {
  it("R1436 an Arabic name sits in a left-to-right <bdi>, its base in a dir=auto <bdi> and its tag apart", () => {
    const { container } = render(
      <p>
        Max is taken, so you’d be <Username name="محمد#2" />
      </p>,
    );
    const outer = container.querySelector("p > bdi");
    expect(outer).not.toBeNull();
    expect(outer).toHaveAttribute("dir", "ltr");
    expect(outer).toHaveAttribute("title", "محمد#2");

    const base = outer?.querySelector(":scope > bdi");
    expect(base).toHaveAttribute("dir", "auto");
    expect(base?.textContent, "the base alone, without its tag").toBe("محمد");

    const tag = outer?.querySelector(":scope > .username__tag");
    expect(tag?.tagName).toBe("SPAN");
    expect(tag?.textContent).toBe("#2");
    expect(base?.contains(tag ?? null), "the tag is outside the base's own direction").toBe(false);
    expect(outer?.textContent).toBe("محمد#2");
  });

  it("R1436 a Hebrew name is isolated the same way", () => {
    const { container } = render(<Username name="שרה#14" />);
    const outer = container.querySelector("bdi.username");
    expect(outer).toHaveAttribute("dir", "ltr");
    expect(outer?.querySelector(":scope > bdi")?.textContent).toBe("שרה");
    expect(outer?.querySelector(".username__tag")?.textContent).toBe("#14");
  });

  it("R1436 a name with no tag renders no tag element, and carries its whole self on the title", () => {
    const { container } = render(<Username name="Max" />);
    const outer = container.querySelector("bdi.username");
    expect(outer).toHaveAttribute("title", "Max");
    expect(outer?.querySelector(".username__tag")).toBeNull();
    expect(outer?.textContent).toBe("Max");
  });

  it("R1436 only digits after the last # are a tag", () => {
    expect(splitUsername("Max#3")).toEqual({ base: "Max", tag: "3" });
    expect(splitUsername("Player#120")).toEqual({ base: "Player", tag: "120" });
    expect(splitUsername("Max")).toEqual({ base: "Max", tag: null });
    expect(splitUsername("#3")).toEqual({ base: "#3", tag: null });
    expect(splitUsername("Max#")).toEqual({ base: "Max#", tag: null });
    expect(splitUsername("Max#x1")).toEqual({ base: "Max#x1", tag: null });
  });
});
