// The Jlore pill (#1659, part of #1651; SPEC §14.1): the balance `GET /api/wallet` answers, in the top
// bar of a signed-in screen. A server answer's balance (`announceWallet(jlore)`) shows at once with no
// request, `announceWallet()` and a navigation read the wallet again, and a visitor or a refused read
// shows nothing. The balance is the server's (CLAUDE.md rule 7): no test here sums anything.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { landingTestid } from "../auth/testids.ts";
import { paths } from "../net/navigate.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";
import { BackLink } from "../routes/nav.tsx";
import DevTools, { devToolsTestid } from "../routes/DevTools.tsx";
import LandingRoute from "../routes/landing.tsx";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { jsonResponse, refusal, stubServer, type StubServer } from "../test/stubServer.ts";
import { setReducedMotion } from "../test/setup.ts";
import JlorePill, { JLORE_COUNT_MS, jlorePillTestid } from "./JlorePill.tsx";
import { __resetWalletForTests, announceWallet } from "./wallet.ts";

const TOKEN = "token-1";

/** The balance the stubbed server holds; a test moves it to say what the next read answers. */
let balance = 120;
/** The status the stubbed wallet answers with: 200, or a refusal's. */
let walletStatus = 200;

function serve(): StubServer {
  return stubServer((call) => {
    if (call.method === "POST" && call.path === "/api/dev/jlore") return { status: 200, body: { jlore: 250 } };
    if (call.method === "POST" && call.path === "/api/dev/clear/jlore") {
      balance = 0;
      return { status: 200, body: { written: 1 } };
    }
    if (call.method === "GET" && call.path === "/api/quests") return { status: 200, body: { quests: [], nextDueAt: null } };
    if (call.method === "GET" && call.path === "/api/wallet") {
      return walletStatus === 200 ? { status: 200, body: { jlore: balance } } : refusal(walletStatus, "forbidden", "Not yet.");
    }
    if (call.method === "GET" && call.path === "/api/auth/me") {
      return {
        status: 200,
        body: {
          profile: { id: "profile-1", status: "active" },
          needsInviteCode: false,
          emailVerified: true,
          currentMatchId: null,
          email: "player@example.test",
        },
      };
    }
    return undefined;
  });
}

function signIn(): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: TOKEN }));
}

/** The pill, once the wallet's answer has put it on screen. */
async function pill(): Promise<HTMLElement> {
  return screen.findByTestId(jlorePillTestid);
}

beforeEach(() => {
  balance = 120;
  walletStatus = 200;
  window.localStorage.clear();
  window.history.replaceState(null, "", "/");
  setReducedMotion(false);
  __resetSettingsForTests();
  __resetWalletForTests();
});

afterEach(() => {
  cleanup();
  setReducedMotion(false);
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("the Jlore pill", () => {
  it("with a session renders the wallet answer's balance, read with the session's token", async () => {
    signIn();
    const server = serve();
    render(<JlorePill />);

    const element = await pill();
    expect(element).toHaveTextContent("120");
    expect(element).toHaveAccessibleName("120 Jlore");
    const reads = server.to("GET", "/api/wallet");
    expect(reads).toHaveLength(1);
    expect(reads[0]?.headers.authorization).toBe(`Bearer ${TOKEN}`);
  });

  it("links to /shop, in the app on a plain click and with the browser's own on a modified one", async () => {
    signIn();
    serve();
    render(<JlorePill />);
    const element = await pill();

    expect(element.closest("a")).toHaveAttribute("href", paths.shop);
    expect(fireEvent.click(element, { button: 0, ctrlKey: true })).toBe(true);
    expect(window.location.pathname).toBe(paths.landing);
    expect(fireEvent.click(element, { button: 0 })).toBe(false);
    expect(window.location.pathname).toBe(paths.shop);
  });

  it("announceWallet(140), a bought pack's answer, re-renders it to 140 with no request", async () => {
    signIn();
    setReducedMotion(true);
    const server = serve();
    render(<JlorePill />);
    await pill();
    expect(server.calls).toHaveLength(1);

    act(() => {
      announceWallet(140);
    });

    expect(screen.getByTestId(jlorePillTestid)).toHaveTextContent("140");
    expect(screen.getByTestId(jlorePillTestid)).toHaveAccessibleName("140 Jlore");
    expect(server.calls).toHaveLength(1);
  });

  it("announceWallet() reads the wallet again and shows what the server answers", async () => {
    signIn();
    setReducedMotion(true);
    const server = serve();
    render(<JlorePill />);
    await pill();

    balance = 175;
    act(() => {
      announceWallet();
    });

    await waitFor(() => {
      expect(screen.getByTestId(jlorePillTestid)).toHaveTextContent("175");
    });
    expect(server.to("GET", "/api/wallet")).toHaveLength(2);
  });

  it("reads the wallet again on a navigation and when the page becomes visible again", async () => {
    signIn();
    setReducedMotion(true);
    const server = serve();
    render(<JlorePill />);
    await pill();

    balance = 130;
    act(() => {
      window.dispatchEvent(new Event("jackioh:navigated"));
    });
    await waitFor(() => {
      expect(screen.getByTestId(jlorePillTestid)).toHaveTextContent("130");
    });
    expect(server.to("GET", "/api/wallet")).toHaveLength(2);

    balance = 90;
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });
    expect(server.to("GET", "/api/wallet")).toHaveLength(2);
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("visible");
    act(() => {
      document.dispatchEvent(new Event("visibilitychange"));
    });
    await waitFor(() => {
      expect(screen.getByTestId(jlorePillTestid)).toHaveTextContent("90");
    });
    expect(server.to("GET", "/api/wallet")).toHaveLength(3);
  });

  it("keeps an announced balance when an older read answers after it", async () => {
    signIn();
    setReducedMotion(true);
    let answer: ((jlore: number) => void) | undefined;
    vi.stubGlobal(
      "fetch",
      vi.fn(
        () =>
          new Promise<Response>((resolve) => {
            answer = (jlore) => {
              resolve(jsonResponse(200, { jlore }));
            };
          }),
      ),
    );
    render(<JlorePill />);
    await waitFor(() => {
      expect(answer).toBeDefined();
    });

    act(() => {
      announceWallet(140);
    });
    expect(await pill()).toHaveTextContent("140");
    await act(async () => {
      answer?.(100);
      await Promise.resolve();
    });

    expect(screen.getByTestId(jlorePillTestid)).toHaveTextContent("140");
  });

  it("with no session renders nothing and makes no request", async () => {
    const server = serve();
    const { container } = render(<JlorePill />);
    act(() => {
      announceWallet();
      window.dispatchEvent(new Event("jackioh:navigated"));
    });

    expect(container).toBeEmptyDOMElement();
    expect(server.calls).toHaveLength(0);
  });

  it("with a session whose wallet read is refused (a pending account) renders nothing", async () => {
    signIn();
    walletStatus = 403;
    const server = serve();
    const { container } = render(<JlorePill />);

    await waitFor(() => {
      expect(server.to("GET", "/api/wallet")).toHaveLength(1);
    });
    expect(container).toBeEmptyDOMElement();
  });

  it("with a wallet answer that is no balance renders nothing", async () => {
    signIn();
    const server = stubServer((call) => (call.path === "/api/wallet" ? { status: 200, body: { jlore: "plenty" } } : undefined));
    const { container } = render(<JlorePill />);

    await waitFor(() => {
      expect(server.to("GET", "/api/wallet")).toHaveLength(1);
    });
    await act(async () => {
      await Promise.resolve();
    });
    expect(container).toBeEmptyDOMElement();
  });
});

describe("the pill counts to a new balance", () => {
  /** `requestAnimationFrame` held until the test says when each frame runs. */
  function holdFrames(): (at: number) => void {
    const frames: FrameRequestCallback[] = [];
    vi.spyOn(window, "requestAnimationFrame").mockImplementation((callback) => frames.push(callback));
    vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => undefined);
    return (at) => {
      const callback = frames.shift();
      act(() => {
        callback?.(at);
      });
    };
  }

  it("up or down through the numbers between, ending on the new one", async () => {
    signIn();
    serve();
    render(<JlorePill />);
    const element = await pill();
    expect(element).toHaveTextContent("120");
    const frame = holdFrames();

    act(() => {
      announceWallet(200);
    });
    expect(element).toHaveTextContent("120");
    frame(1000);
    frame(1000 + JLORE_COUNT_MS / 2);
    const middle = Number(element.textContent);
    expect(middle).toBeGreaterThan(120);
    expect(middle).toBeLessThan(200);
    frame(1000 + JLORE_COUNT_MS);
    expect(element).toHaveTextContent("200");
    // The accessible name is the balance, not the count on its way.
    expect(element).toHaveAccessibleName("200 Jlore");

    act(() => {
      announceWallet(150);
    });
    frame(5000);
    frame(5000 + JLORE_COUNT_MS / 2);
    const lower = Number(element.textContent);
    expect(lower).toBeLessThan(200);
    expect(lower).toBeGreaterThan(150);
    frame(5000 + JLORE_COUNT_MS);
    expect(element).toHaveTextContent("150");
  });

  it("at once under the system's reduced motion", async () => {
    signIn();
    serve();
    render(<JlorePill />);
    await pill();
    const frames = vi.spyOn(window, "requestAnimationFrame");
    setReducedMotion(true);

    act(() => {
      announceWallet(200);
    });

    expect(screen.getByTestId(jlorePillTestid)).toHaveTextContent("200");
    expect(frames).not.toHaveBeenCalled();
  });

  it("at once under the settings panel's Reduce motion", async () => {
    signIn();
    serve();
    render(<JlorePill />);
    await pill();
    const frames = vi.spyOn(window, "requestAnimationFrame");
    act(() => {
      writeSettings({ reduceMotion: true });
    });

    act(() => {
      announceWallet(200);
    });

    expect(screen.getByTestId(jlorePillTestid)).toHaveTextContent("200");
    expect(frames).not.toHaveBeenCalled();
  });
});

describe("where the pill sits", () => {
  it("BackLink's top bar has it, before the language button and the gear", async () => {
    signIn();
    serve();
    render(<BackLink />);

    const element = await pill();
    const bar = element.closest("nav");
    expect(bar).not.toBeNull();
    const order = Array.from(bar?.querySelectorAll("[data-testid]") ?? []).map((node) => node.getAttribute("data-testid"));
    expect(order).toEqual(["nav-back", jlorePillTestid, "language-nav", "settings-open-nav"]);
  });

  it("BackLink with showSettings={false}, the board's, has no pill and reads no wallet", async () => {
    signIn();
    const server = serve();
    render(<BackLink showSettings={false} />);
    act(() => {
      announceWallet();
    });

    expect(screen.queryByTestId(jlorePillTestid)).toBeNull();
    expect(server.to("GET", "/api/wallet")).toHaveLength(0);
  });

  it("BackLink with showWallet={false}, a signed-out screen's, keeps its buttons, has no pill and reads no wallet", async () => {
    signIn();
    const server = serve();
    render(<BackLink showWallet={false} />);
    act(() => {
      announceWallet();
    });

    expect(screen.getByTestId("settings-open-nav")).toBeInTheDocument();
    expect(screen.queryByTestId(jlorePillTestid)).toBeNull();
    expect(server.to("GET", "/api/wallet")).toHaveLength(0);
  });

  it("the landing's top bar has one beside its two buttons", async () => {
    signIn();
    serve();
    render(<LandingRoute />);

    const bar = await screen.findByTestId(jlorePillTestid);
    const corner = bar.closest(".landing-corner");
    expect(corner).not.toBeNull();
    const topbar = screen.getByTestId(landingTestid.root).querySelector(".landing-topbar");
    expect(topbar).toContainElement(bar);
    expect(within(corner as HTMLElement).getByTestId("language-nav")).toBeInTheDocument();
    expect(within(corner as HTMLElement).getByTestId("settings-open-nav")).toBeInTheDocument();
  });

  it("the landing's top bar has none for a visitor", async () => {
    const server = serve();
    render(<LandingRoute />);

    await screen.findByTestId(landingTestid.signIn);
    expect(screen.queryByTestId(jlorePillTestid)).toBeNull();
    expect(server.to("GET", "/api/wallet")).toHaveLength(0);
  });
});

describe("the Dev tools tell the pill", () => {
  it("Give Jlore shows the answer's balance with no read, and a clear has the pill read the wallet again", async () => {
    signIn();
    setReducedMotion(true);
    const server = serve();
    render(
      <>
        <JlorePill />
        <DevTools token={TOKEN} />
      </>,
    );
    await pill();

    await userEvent.type(screen.getByTestId(devToolsTestid.jlore), "130");
    await userEvent.click(screen.getByTestId(devToolsTestid.giveJlore));
    await waitFor(() => {
      expect(screen.getByTestId(jlorePillTestid)).toHaveTextContent("250");
    });
    expect(server.to("GET", "/api/wallet")).toHaveLength(1);

    await userEvent.click(screen.getByTestId("dev-tools-clear-jlore"));
    await waitFor(() => {
      expect(screen.getByTestId(jlorePillTestid)).toHaveTextContent(/^0$/);
    });
    expect(server.to("GET", "/api/wallet")).toHaveLength(2);
  });
});
