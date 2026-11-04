// @vitest-environment node
// public/_headers, as each build fills it: a strict CSP whose only outside host is the build's own
// Supabase project, no framing, no referrer and no indexing.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { CONNECT_SRC_PLACEHOLDER, connectSources, filledHeaders } from "./headers.ts";

const ADMIN = dirname(fileURLToPath(import.meta.url));
const TEMPLATE = readFileSync(join(ADMIN, "public/_headers"), "utf8");

/** The value of `name` on the `/*` block's lines. */
function header(text: string, name: string): string | undefined {
  const line = text.split("\n").find((entry) => entry.trim().startsWith(`${name}:`));
  return line?.trim().slice(name.length + 1).trim();
}

function directives(csp: string): Map<string, string[]> {
  return new Map(
    csp.split(";").map((part) => {
      const [name = "", ...sources] = part.trim().split(/\s+/u);
      return [name, sources];
    }),
  );
}

const PRODUCTION = "https://fuzpvmjdejvbjcarypyi.supabase.co";
const STAGING = "https://exmjdaswedxhnzmpzqrq.supabase.co";

describe("connectSources", () => {
  it("names 'self' and the Supabase project's origin, and only 'self' without one", () => {
    expect(connectSources(`${PRODUCTION}/`)).toBe(`'self' ${PRODUCTION}`);
    expect(connectSources(undefined)).toBe("'self'");
    expect(connectSources("")).toBe("'self'");
  });
});

describe("filledHeaders", () => {
  it("refuses a template without the placeholder, and one with a placeholder it does not fill", () => {
    expect(() => filledHeaders("/*\n  X-Frame-Options: DENY\n", PRODUCTION)).toThrow(CONNECT_SRC_PLACEHOLDER);
    expect(() => filledHeaders(`connect-src ${CONNECT_SRC_PLACEHOLDER}; %OTHER%`, PRODUCTION)).toThrow("%OTHER%");
  });
});

describe.each([
  { build: "production", supabase: PRODUCTION },
  { build: "staging", supabase: STAGING },
])("public/_headers, the $build build's", ({ supabase }) => {
  const filled = filledHeaders(TEMPLATE, supabase);
  const csp = directives(header(filled, "Content-Security-Policy") ?? "");

  it("lets the page reach itself and its own Supabase project, and nothing else", () => {
    expect(csp.get("connect-src")).toEqual(["'self'", supabase]);
    expect(filled).not.toContain(CONNECT_SRC_PLACEHOLDER);
  });

  it("runs only its own scripts and styles, with no inline code, plugins or base rewrites", () => {
    expect(csp.get("default-src")).toEqual(["'self'"]);
    expect(csp.get("script-src")).toEqual(["'self'"]);
    expect(csp.get("style-src")).toEqual(["'self'"]);
    expect(csp.get("object-src")).toEqual(["'none'"]);
    expect(csp.get("base-uri")).toEqual(["'none'"]);
    expect(csp.get("form-action")).toEqual(["'self'"]);
    for (const sources of csp.values()) {
      expect(sources).not.toContain("'unsafe-inline'");
      expect(sources).not.toContain("'unsafe-eval'");
      expect(sources).not.toContain("*");
    }
  });

  it("is framed by no one and frames no one", () => {
    expect(csp.get("frame-ancestors")).toEqual(["'none'"]);
    expect(csp.get("frame-src")).toEqual(["'none'"]);
    expect(header(filled, "X-Frame-Options")).toBe("DENY");
  });

  it("sends no referrer and asks not to be indexed", () => {
    expect(header(filled, "Referrer-Policy")).toBe("no-referrer");
    expect(header(filled, "X-Robots-Tag")).toMatch(/^noindex\b/u);
    expect(header(filled, "X-Content-Type-Options")).toBe("nosniff");
  });

  it("applies to every path the build serves", () => {
    const lines = filled.split("\n");
    const at = lines.findIndex((line) => line.trim().startsWith("Content-Security-Policy:"));
    expect(lines.slice(0, at).reverse().find((line) => line.trim() !== "" && !line.startsWith(" "))).toBe("/*");
  });
});
