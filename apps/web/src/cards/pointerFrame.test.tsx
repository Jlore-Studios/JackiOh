// #3610 (R3351000): a pointer follower keeps the newest pointermove and handles it once an animation frame
// (pointerFrame.ts). The frame source is never a timer (R200). The follower's own readers are hostMove.test.tsx's,
// artLive.test.tsx's, tilt.test.tsx's and FxLayer.test.tsx's.

import { describe, expect, it, vi } from "vitest";

import type { FxFrameSource } from "../fx/types.ts";
import { followPointer, pointerFrames } from "./pointerFrame.ts";

/** A frame source a test runs by hand: each request gets a handle, `frame()` runs the ones still waiting. */
function fakeSource() {
  let next = 1;
  const waiting = new Map<number, (timestampMs: number) => void>();
  const request = vi.fn((callback: (timestampMs: number) => void): number => {
    const handle = next;
    next += 1;
    waiting.set(handle, callback);
    return handle;
  });
  const cancel = vi.fn((handle: number): void => {
    waiting.delete(handle);
  });
  const source: FxFrameSource = { request, cancel };
  return {
    source,
    request,
    cancel,
    frame(): void {
      const due = [...waiting.values()];
      waiting.clear();
      for (const callback of due) callback(0);
    },
  };
}

/** A pointermove at (`x`, 0). */
const move = (x: number): PointerEvent => new MouseEvent("pointermove", { clientX: x }) as PointerEvent;

describe("R3351000 followPointer handles the newest move once a frame (#3610)", () => {
  it("R3351000 ten pushes ask for one frame, and the frame hands over the last event once", () => {
    const fake = fakeSource();
    const handle = vi.fn();
    const follow = followPointer(fake.source, handle);
    const events = Array.from({ length: 10 }, (_unused, index) => move(index));
    for (const event of events) follow.push(event);
    expect(fake.request).toHaveBeenCalledTimes(1);
    expect(handle).not.toHaveBeenCalled();
    fake.frame();
    expect(handle).toHaveBeenCalledTimes(1);
    expect(handle).toHaveBeenCalledWith(events[9]);
  });

  it("R3351000 a push after the frame has run asks for a new one", () => {
    const fake = fakeSource();
    const handle = vi.fn<(event: PointerEvent) => void>();
    const follow = followPointer(fake.source, handle);
    follow.push(move(1));
    fake.frame();
    follow.push(move(2));
    follow.push(move(3));
    expect(fake.request).toHaveBeenCalledTimes(2);
    fake.frame();
    expect(handle).toHaveBeenCalledTimes(2);
    expect(handle.mock.calls.map(([event]) => event.clientX)).toEqual([1, 3]);
  });

  it("R3351000 cancel drops the waiting frame and its event, and the next push asks for a new frame", () => {
    const fake = fakeSource();
    const handle = vi.fn<(event: PointerEvent) => void>();
    const follow = followPointer(fake.source, handle);
    follow.push(move(1));
    follow.push(move(2));
    follow.cancel();
    expect(fake.cancel).toHaveBeenCalledTimes(1);
    expect(fake.cancel).toHaveBeenCalledWith(1);
    fake.frame();
    expect(handle).not.toHaveBeenCalled();
    follow.push(move(3));
    expect(fake.request).toHaveBeenCalledTimes(2);
    fake.frame();
    expect(handle).toHaveBeenCalledTimes(1);
    expect(handle.mock.calls[0]?.[0].clientX).toBe(3);
  });

  it("R3351000 cancel with nothing waiting asks the source for nothing", () => {
    const fake = fakeSource();
    const follow = followPointer(fake.source, vi.fn());
    follow.cancel();
    follow.push(move(1));
    fake.frame();
    follow.cancel();
    expect(fake.cancel).not.toHaveBeenCalled();
  });

  it("R3351000 a frame handle of 0 still counts as a waiting frame", () => {
    // pointerFrames() gives 0 where it never fires: a second push must not ask again, and cancel must cancel it.
    const request = vi.fn((): number => 0);
    const cancel = vi.fn();
    const follow = followPointer({ request, cancel }, vi.fn());
    follow.push(move(1));
    follow.push(move(2));
    expect(request).toHaveBeenCalledTimes(1);
    follow.cancel();
    expect(cancel).toHaveBeenCalledWith(0);
  });
});

describe("R3351000 pointerFrames is requestAnimationFrame, and never a timer (#3610, R200)", () => {
  it("R3351000 where the window has no requestAnimationFrame it never fires, and no timer is used", () => {
    const original = window.requestAnimationFrame;
    const timer = vi.spyOn(window, "setTimeout");
    const interval = vi.spyOn(window, "setInterval");
    try {
      Object.defineProperty(window, "requestAnimationFrame", { value: undefined, configurable: true, writable: true });
      const source = pointerFrames();
      const callback = vi.fn();
      const handle = source.request(callback);
      source.cancel(handle);
      expect(callback).not.toHaveBeenCalled();
      expect(timer).not.toHaveBeenCalled();
      expect(interval).not.toHaveBeenCalled();
    } finally {
      Object.defineProperty(window, "requestAnimationFrame", { value: original, configurable: true, writable: true });
      timer.mockRestore();
      interval.mockRestore();
    }
  });

  it("R3351000 where there is one it asks the window's, and cancels the window's", () => {
    const request = vi.spyOn(window, "requestAnimationFrame").mockReturnValue(7);
    const cancel = vi.spyOn(window, "cancelAnimationFrame").mockImplementation(() => undefined);
    try {
      const source = pointerFrames();
      const callback = vi.fn();
      expect(source.request(callback)).toBe(7);
      expect(request).toHaveBeenCalledWith(callback);
      source.cancel(7);
      expect(cancel).toHaveBeenCalledWith(7);
    } finally {
      request.mockRestore();
      cancel.mockRestore();
    }
  });
});
