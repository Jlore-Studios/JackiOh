// The Dev tools panel (SPEC §14.10, R1300000): each tool is a request to its dev route, sent as
// typed, and the line under the buttons says what the server answered, a refusal in its own words.

import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import DevTools, { devToolsTestid } from "./DevTools.tsx";

const TOKEN = "token-1";

/** `fetch` answering every request with `status` and `body`. */
function answer(status: number, body?: unknown) {
  return vi.fn((_url: string, _init?: RequestInit) =>
    Promise.resolve({
      ok: status >= 200 && status < 300,
      status,
      text: () => Promise.resolve(body === undefined ? "" : JSON.stringify(body)),
    } as unknown as Response),
  );
}

/** The path and the parsed body of the `n`th request `fetchMock` saw. */
function sent(fetchMock: ReturnType<typeof answer>, n: number): { path: string; method: string | undefined; body: unknown } {
  const [url, init] = fetchMock.mock.calls[n] ?? [];
  return {
    path: new URL(String(url), "http://localhost").pathname,
    method: init?.method,
    body: typeof init?.body === "string" ? JSON.parse(init.body) : undefined,
  };
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("R1300000 the Dev tools panel", () => {
  it("R1300000 sends each tool to its dev route and shows the answer", async () => {
    const fetchMock = answer(200, { jlore: 50 });
    vi.stubGlobal("fetch", fetchMock);
    render(<DevTools token={TOKEN} />);
    const status = screen.getByTestId(devToolsTestid.status);

    await userEvent.type(screen.getByTestId(devToolsTestid.jlore), "50");
    await userEvent.click(screen.getByTestId(devToolsTestid.giveJlore));
    await waitFor(() => {
      expect(status).toHaveTextContent("Done.");
    });
    expect(sent(fetchMock, 0)).toEqual({ path: "/api/dev/jlore", method: "POST", body: { jlore: 50 } });
    const [, init] = fetchMock.mock.calls[0] ?? [];
    expect((init?.headers as Record<string, string>).authorization).toBe(`Bearer ${TOKEN}`);

    await userEvent.selectOptions(screen.getByTestId(devToolsTestid.set), "Classic");
    await userEvent.type(screen.getByTestId(devToolsTestid.xp), "4500");
    await userEvent.click(screen.getByTestId(devToolsTestid.giveXp));
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledTimes(2);
    });
    expect(sent(fetchMock, 1)).toEqual({ path: "/api/dev/xp", method: "POST", body: { setId: "Classic", xp: 4500 } });

    await userEvent.click(screen.getByTestId(devToolsTestid.fill));
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledTimes(3);
    });
    expect(sent(fetchMock, 2).path).toBe("/api/dev/fill");

    await userEvent.click(screen.getByTestId("dev-tools-clear-everything"));
    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledTimes(4);
    });
    expect(sent(fetchMock, 3)).toEqual({ path: "/api/dev/clear/everything", method: "POST", body: {} });
    for (const part of ["cards", "cosmetics", "jlore", "counters", "tracks"]) {
      expect(screen.getByTestId(`dev-tools-clear-${part}`)).toBeInTheDocument();
    }
  });

  it("R1300000 says the server's refusal in its own words", async () => {
    vi.stubGlobal(
      "fetch",
      answer(403, { error: { code: "forbidden", message: "Only a dev account may use the dev tools." } }),
    );
    render(<DevTools token={TOKEN} />);
    await userEvent.click(screen.getByTestId(devToolsTestid.fill));
    expect(await screen.findByText("Only a dev account may use the dev tools.")).toBe(
      screen.getByTestId(devToolsTestid.status),
    );
  });

  it("R1300000 holds every button while a request is out", async () => {
    let finish: (response: Response) => void = () => undefined;
    vi.stubGlobal(
      "fetch",
      vi.fn(
        () =>
          new Promise<Response>((resolve) => {
            finish = resolve;
          }),
      ),
    );
    render(<DevTools token={TOKEN} />);
    await userEvent.click(screen.getByTestId(devToolsTestid.fill));

    expect(screen.getByTestId(devToolsTestid.status)).toHaveTextContent("Working…");
    expect(screen.getByTestId(devToolsTestid.giveJlore)).toBeDisabled();
    expect(screen.getByTestId("dev-tools-clear-cards")).toBeDisabled();
    finish({ ok: true, status: 200, text: () => Promise.resolve('{"cards":0,"cosmetics":0}') } as unknown as Response);
    await waitFor(() => {
      expect(screen.getByTestId(devToolsTestid.status)).toHaveTextContent("Done.");
    });
    expect(screen.getByTestId(devToolsTestid.fill)).toBeEnabled();
  });
});
