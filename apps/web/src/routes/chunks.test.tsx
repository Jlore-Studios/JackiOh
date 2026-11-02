// R2116000 (#2116, #2036): a screen's code starts loading on intent toward a control leading there,
// once per route per page load, with the engine for a game's screen. `wasm/index.ts` is mocked as
// `routes/engine-gate.test.tsx` does, so the setup file's real module stays underneath.

import { Suspense, type ReactElement, type ReactNode } from "react";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const loadWasm = vi.fn((): Promise<void> => Promise.resolve());
vi.doMock("../wasm/index.ts", async (importOriginal) => ({
  ...(await importOriginal<Record<string, unknown>>()),
  loadWasm,
}));
const { resetRoutePrefetchForTests, routeChunk, routeChunks, watchRouteIntent } = await import("./chunks.tsx");

function Stub(): ReactElement {
  return <p data-testid="stub-screen">stub</p>;
}
const stubModule = { default: Stub };

let stop: () => void = () => undefined;

beforeEach(() => {
  resetRoutePrefetchForTests();
  loadWasm.mockClear();
  stop = watchRouteIntent(document);
});

afterEach(() => {
  stop();
  cleanup();
  Reflect.deleteProperty(navigator, "connection");
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
});

describe("R2116000 prefetching a screen on intent", () => {
  it("R2116000 hovering a control calls its route's import once, and a second hover starts no new fetch", () => {
    const load = vi.spyOn(routeChunks.privacy, "load").mockResolvedValue(stubModule);
    render(
      <a href="/privacy">
        <span>Privacy</span>
      </a>,
    );
    const inner = screen.getByText("Privacy");

    fireEvent.pointerEnter(inner);
    expect(load).toHaveBeenCalledTimes(1);

    fireEvent.pointerEnter(inner);
    fireEvent.focus(inner);
    fireEvent.touchStart(inner);
    expect(load).toHaveBeenCalledTimes(1);
  });

  it("R2116000 focus or a touch on a button that names its route in data-route prefetches that route", () => {
    const terms = vi.spyOn(routeChunks.terms, "load").mockResolvedValue(stubModule);
    const almanac = vi.spyOn(routeChunks.almanac, "load").mockResolvedValue(stubModule);
    render(
      <>
        <button type="button" data-route="/terms">
          Terms
        </button>
        <button type="button" data-route="/almanac?card=c001">
          Almanac
        </button>
      </>,
    );

    fireEvent.focus(screen.getByText("Terms"));
    expect(terms).toHaveBeenCalledTimes(1);

    fireEvent.touchStart(screen.getByText("Almanac"));
    expect(almanac).toHaveBeenCalledTimes(1);
  });

  it("R2116000 a link to another site, or to a screen in the entry chunk, prefetches nothing", () => {
    const loads = Object.values(routeChunks)
      .filter((chunk) => chunk !== null)
      .map((chunk) => vi.spyOn(chunk, "load"));
    render(
      <>
        <a href="https://example.com/privacy">Elsewhere</a>
        <a href="/">Home</a>
        <a href="/account">Account</a>
      </>,
    );

    fireEvent.pointerEnter(screen.getByText("Elsewhere"));
    fireEvent.pointerEnter(screen.getByText("Home"));
    fireEvent.pointerEnter(screen.getByText("Account"));

    for (const load of loads) expect(load).not.toHaveBeenCalled();
    expect(loadWasm).not.toHaveBeenCalled();
  });

  it("R2116000 a prefetch and the route's own load share one import, and a failed one is asked again", async () => {
    const importScreen = vi
      .fn<() => Promise<typeof stubModule>>()
      .mockRejectedValueOnce(new Error("flaky network"))
      .mockResolvedValue(stubModule);
    const chunk = routeChunk(importScreen);

    await expect(chunk.load()).rejects.toThrow("flaky network");
    const retry = chunk.load();
    await expect(retry).resolves.toBe(stubModule);
    expect(importScreen).toHaveBeenCalledTimes(2);
  });

  it("R2116000 a prefetched route renders without showing the Suspense fallback", async () => {
    const fallbackSpy = vi.fn();
    function Fallback(): ReactNode {
      fallbackSpy();
      return null;
    }

    const cold = routeChunk(() => Promise.resolve(stubModule));
    render(
      <Suspense fallback={<Fallback />}>
        <cold.Screen />
      </Suspense>,
    );
    await screen.findByTestId("stub-screen");
    expect(fallbackSpy).toHaveBeenCalled();
    cleanup();
    fallbackSpy.mockClear();

    const warm = routeChunk(() => Promise.resolve(stubModule));
    await act(() => warm.load());
    render(
      <Suspense fallback={<Fallback />}>
        <warm.Screen />
      </Suspense>,
    );
    expect(screen.getByTestId("stub-screen")).toBeInTheDocument();
    expect(fallbackSpy).not.toHaveBeenCalled();
  });

  it("R2116000 Save-Data turns all prefetching off", () => {
    Object.defineProperty(navigator, "connection", { value: { saveData: true }, configurable: true });
    const load = vi.spyOn(routeChunks.practice, "load").mockResolvedValue(stubModule);
    render(<a href="/practice">Practice</a>);
    const link = screen.getByText("Practice");

    fireEvent.pointerEnter(link);
    fireEvent.focus(link);
    fireEvent.touchStart(link);

    expect(load).not.toHaveBeenCalled();
    expect(loadWasm).not.toHaveBeenCalled();
  });

  it("R2116000 intent toward Practice starts loadWasm(), and intent toward a page with no game does not", () => {
    const practice = vi.spyOn(routeChunks.practice, "load").mockResolvedValue(stubModule);
    const copyright = vi.spyOn(routeChunks.copyright, "load").mockResolvedValue(stubModule);
    render(
      <>
        <a href="/copyright">Copyright</a>
        <a href="/practice">Practice</a>
      </>,
    );

    fireEvent.pointerEnter(screen.getByText("Copyright"));
    expect(copyright).toHaveBeenCalledTimes(1);
    expect(loadWasm).not.toHaveBeenCalled();

    fireEvent.pointerEnter(screen.getByText("Practice"));
    expect(practice).toHaveBeenCalledTimes(1);
    expect(loadWasm).toHaveBeenCalledTimes(1);
  });

  it("R2116000 a production build has no dev-only route to prefetch", async () => {
    vi.stubEnv("MODE", "production");
    vi.resetModules();
    const production = await import("./chunks.tsx");

    expect(production.routeChunks.hotseat).toBeNull();
    expect(production.routeChunks.duosDev).toBeNull();
    expect(() => production.prefetchRoute("/dev/hotseat")).not.toThrow();
  });
});
