// #2160: the one testid selector and lookup the card bar, the drag layer and the coach share.

import { afterEach, describe, expect, it } from "vitest";

import { byTestid, testidSelector } from "./contract.ts";

afterEach(() => {
  document.body.innerHTML = "";
});

describe("testidSelector and byTestid", () => {
  it("find the element a testid names, its quotes and backslashes escaped", () => {
    const plain = document.createElement("div");
    plain.setAttribute("data-testid", "card-u1");
    const odd = document.createElement("div");
    odd.setAttribute("data-testid", 'odd"id\\x');
    document.body.append(plain, odd);

    expect(testidSelector("card-u1")).toBe('[data-testid="card-u1"]');
    expect(byTestid("card-u1")).toBe(plain);
    expect(testidSelector('odd"id\\x')).toBe('[data-testid="odd\\"id\\\\x"]');
    expect(byTestid('odd"id\\x')).toBe(odd);
    expect(byTestid("card-u2")).toBeNull();
  });
});
