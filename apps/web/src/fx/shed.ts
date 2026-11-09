// R3351000: the living game's shared shed level (issue #3351, part of #3330). On a phone the
// moving faces, tile auras, backdrops, particles and tilt draw from per-layer budgets that scale
// with the phone caps — the faces from the motion slots (COSMETIC_FOIL_ANIMATED_MAX, 6, on every
// viewport), tile and aura motes from auraBudget and tileMoteBudget against the phone particle cap
// (capacityFor), tilt holding one claim — and past the phone budget (PHONE_MOTION_BUDGET, 6) the
// director sheds in the ruled order — motes first, then far auras, then the moving faces furthest
// from the pointer — and never a set piece's main beats. The director publishes its stage here;
// the aura layers and the motion slots read it. Presentation only (CLAUDE.md rule 7): frame
// times and the pointer, never game state.

/** The shed stages: full, motes, far auras, faces. */
export const SHED_FULL = 0;
export const SHED_MOTES = 1;
export const SHED_AURAS = 2;
export const SHED_FACES = 3;
/** The deepest stage: every shed scale bottoms out here. */
export const SHED_MAX = 3;

let level = SHED_FULL;
let pointer: { x: number; y: number } | null = null;
const listeners = new Set<() => void>();

function notify(): void {
  for (const listener of [...listeners]) listener();
}

export function subscribeShed(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** R3351000: the stage the director has shed to: 0 full, 1 motes, 2 far auras, 3 faces. */
export function shedLevel(): number {
  return level;
}

/** R3351000: the director's stage after a slow or a healthy window; clamped to 0..SHED_MAX. */
export function setShedLevel(next: number): void {
  const clamped = Math.min(SHED_MAX, Math.max(SHED_FULL, Math.floor(next)));
  if (clamped === level) return;
  level = clamped;
  notify();
}

/** R3351000: stage 1 halves mote emission; every earlier stage emits whole. */
export function shedEmissionScale(at: number = level): number {
  return at >= SHED_MOTES ? 0.5 : 1;
}

/** R3351000: stage 2 halves aura intensity; every earlier stage draws whole. */
export function shedAuraScale(at: number = level): number {
  return at >= SHED_AURAS ? 0.5 : 1;
}

/** R3351000: stage 3 stills the moving faces past the pointer's own. */
export function shedStillsFaces(at: number = level): boolean {
  return at >= SHED_FACES;
}

/** R3351000: where the pointer was last seen, in page pixels (as getBoundingClientRect reads); null while none is known. */
export function shedPointer(): { x: number; y: number } | null {
  return pointer;
}

/** R3351000: the effects layer feeds every pointer move here, or null when it unmounts. */
export function setShedPointer(next: { x: number; y: number } | null): void {
  if (next === null && pointer === null) return;
  if (next !== null && pointer !== null && next.x === pointer.x && next.y === pointer.y) return;
  pointer = next;
  notify();
}

/** Test seam: drops the stage, the pointer and every listener. */
export function resetShedForTests(): void {
  level = SHED_FULL;
  pointer = null;
  listeners.clear();
}
