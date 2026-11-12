// #3674 (R3351000, R3340000, R3344001): one pointermove listener per card host for what a face draws from the pointer:
// its live art's aim (artLive.ts) and its tilt (tilt.ts). Each reader takes the event and reads the boxes it needs, and
// only once every one of them has read does each write what it read. A write before another reader's read would make
// that read lay the face's style out again, once per move: on a throttled phone, with the pointer over a hand card,
// about a fifth of a frame. Presentation only (CLAUDE.md rule 7): nothing here reaches the engine, an action or the log.

/** A reader of one move: it reads the event and the boxes it needs and returns what it writes, if anything. */
export type HostMoveReader = (event: PointerEvent) => (() => void) | undefined;

type Hub = { readonly readers: Set<HostMoveReader>; readonly listener: (event: PointerEvent) => void };

const hubs = new WeakMap<HTMLElement, Hub>();

/**
 * #3674: `reader` hears every pointermove on `host`, its write run after every reader of the same move has read. Returns
 * what stops it; the host's one listener goes with its last reader.
 */
export function hearHostMove(host: HTMLElement, reader: HostMoveReader): () => void {
  let hub = hubs.get(host);
  if (hub === undefined) {
    const readers = new Set<HostMoveReader>();
    const listener = (event: PointerEvent): void => {
      const writes: (() => void)[] = [];
      for (const read of [...readers]) {
        const write = read(event);
        if (write !== undefined) writes.push(write);
      }
      for (const write of writes) write();
    };
    hub = { readers, listener };
    hubs.set(host, hub);
    host.addEventListener("pointermove", listener, { passive: true });
  }
  const own = hub;
  own.readers.add(reader);
  return () => {
    own.readers.delete(reader);
    if (own.readers.size > 0 || hubs.get(host) !== own) return;
    host.removeEventListener("pointermove", own.listener);
    hubs.delete(host);
  };
}
