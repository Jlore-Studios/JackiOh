// The numbers of the turn clock's last stretch (R439), and nothing else. CLAUDE.md rule 9: every
// number the urgent readout and the fuse use is named here. The clock's own lengths (`TURN_CLOCK_MS`
// and the rest) are the server's, in `apps/server/src/config.ts`; these are presentation only and
// decide nothing (CLAUDE.md rule 7).

/** A second, in milliseconds: a unit, not configuration. */
export const MS_PER_SECOND = 1000;

/**
 * R439: the final stretch of a turn clock. From here the readout turns urgent: red, labelled, with a
 * gauge, and on the viewer's own clock the ember fuse and the heartbeat. The readout rounds up
 * (`formatClock`), so the stretch starts exactly when it first reads "30s".
 */
export const TURN_CLOCK_FINAL_MS = 30 * MS_PER_SECOND;

/** R439: the last stretch inside it, where the tick sharpens and the heartbeat quickens ("10s"). */
export const TURN_CLOCK_LAST_MS = 10 * MS_PER_SECOND;

/**
 * The fuse runs round the screen's four edges, clockwise from the top-left corner, and each edge is
 * an equal share of the burn whatever its length in pixels, so a corner is always a quarter.
 */
export const FUSE_EDGES = 4;

/** Percent of an edge: the unit the fuse's spark is placed in. */
export const PERCENT = 100;
