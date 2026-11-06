// A WebSocket a test drives by hand (`SocketLike`, `game/net.ts`): what the client sends is kept in
// `sent`, and the test opens it, delivers frames and drops it. `fakeSockets` is a socket factory that
// keeps every socket it made, newest last (R3214000's social socket takes one).

import type { SocketFactory, SocketLike } from "../game/net.ts";

export class FakeSocket implements SocketLike {
  readyState = 0;
  onopen: ((event: unknown) => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onclose: ((event: { code: number; reason: string; wasClean: boolean }) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;

  readonly sent: string[] = [];
  closedWith: { code?: number; reason?: string } | null = null;

  constructor(
    readonly url: string,
    readonly protocols: readonly string[] = [],
  ) {}

  send(data: string): void {
    this.sent.push(data);
  }

  close(code?: number, reason?: string): void {
    this.readyState = 3;
    this.closedWith = { ...(code === undefined ? {} : { code }), ...(reason === undefined ? {} : { reason }) };
  }

  // Test drivers

  open(): void {
    this.readyState = 1;
    this.onopen?.({});
  }

  deliver(frame: unknown): void {
    this.onmessage?.({ data: typeof frame === "string" ? frame : JSON.stringify(frame) });
  }

  drop(code = 1006, reason = ""): void {
    this.readyState = 3;
    this.onclose?.({ code, reason, wasClean: false });
  }

  frames(): Record<string, unknown>[] {
    return this.sent.map((text) => JSON.parse(text) as Record<string, unknown>);
  }
}

/** A factory of `FakeSocket`s and every socket it made, in order. */
export function fakeSockets(): { factory: SocketFactory; sockets: FakeSocket[] } {
  const sockets: FakeSocket[] = [];
  return {
    factory: (url, protocols) => {
      const socket = new FakeSocket(url, protocols);
      sockets.push(socket);
      return socket;
    },
    sockets,
  };
}
