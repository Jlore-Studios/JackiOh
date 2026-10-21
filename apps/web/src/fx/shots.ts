/**
 * Flavor shots (#1442, docs/polish/8-flavor.md): what a Ranged attacker fires at its target. Four
 * motions, each drawn procedurally on the effects canvas, never from an image (B27): an `arc` throws
 * sprites on a lob, a `bolt` is a glowing shot on a straight line with a particle trail, a `beam` is
 * an instant line and a `swipe` is a claw-slash crescent. Each shot ends exactly when its flight is
 * up, bursting its preset where it lands, so nothing outlives its entry (R200).
 *
 * Glow comes from layered strokes and circles under `lighter` blending, with no canvas blur or text,
 * and every random choice comes from the injected `FxRng`.
 */
import type { ParticleSystem, EmitOptions } from "./particles.ts";
import type { FxRng } from "./rng.ts";
import type { FxBox, FxPreset, FxShot, FxShotSprite, FxVec } from "./types.ts";

export type Shots = {
  /** Starts `shot` from `from` to `to` over `flightMs`: `colors` run hot first, `preset` bursts where it lands, `density` scales the trail and that burst. */
  add(shot: FxShot, colors: readonly string[], preset: FxPreset, from: FxVec, to: FxVec, flightMs: number, density: number): void;
  /** `dtMs` (clamped frame time) paces the trail; `ageMs` (real time) ages shots, so a stalled frame never stretches one past its end (R200). */
  step(dtMs: number, ageMs: number): void;
  draw(ctx: CanvasRenderingContext2D): void;
  alive(): number;
  clear(): void;
};

/** CLAUDE.md rule 9: every shot number in one place. Lengths are CSS px. */
const SHOT = {
  /** An arc's control point sits this fraction of the chord up the screen from the chord's midpoint. */
  arcLift: 0.3,
  /** Sprites of one volley land this far apart across the path… */
  fanPx: 26,
  /** …and a scattering volley's also drift up to this far on each axis, each arc bending this much more or less. */
  scatterPx: 30,
  scatterBendMin: 0.6,
  scatterBendMax: 1.4,
  /** A spinning sprite turns this many radians per ms, each sprite of a volley this far ahead of the last. */
  spinPerMs: 0.02,
  spinStagger: 1.3,
  /** A floating sprite bobs and tilts at this rate. */
  floatHz: 3,
  floatBobPx: 6,
  floatTilt: 0.25,
  floatStagger: 1.7,
  /** Trail particles per ms of flight, and at least this many each step. */
  trailPerMs: 0.3,
  trailMinPerStep: 1,
  trailPower: 0.35,
  /** Each trail particle is born this far (either way) off the path, so the trail has body. */
  trailJitter: 3,
  /** The burst where a shot lands: its count is shared by the sprites of a volley. */
  arrivalCount: 16,
  arrivalPower: 1.3,
  /** A bolt's head: three discs, widest first, radii and alphas. */
  haloPx: 20,
  haloAlpha: 0.25,
  bodyPx: 9,
  bodyAlpha: 0.85,
  corePx: 4,
  /** A bolt's comet tail: this many past head positions, as a tapering stroke. */
  tailPoints: 8,
  tailWidth: 12,
  tailAlpha: 0.5,
  /** A wisp weaves this many whole cycles about its line, this far to either side. */
  waveCycles: 2,
  waveAmpPx: 14,
  /** A beam: three strokes of one line, widest and faintest first: [width multiple, alpha]. */
  beamWidthPx: 6,
  beamPasses: [
    [3, 0.18],
    [1.6, 0.45],
    [0.5, 1],
  ],
  /** A swipe: three crescents side by side, each this radius, spanning ± this many radians about its heading. */
  swipeRadiusPx: 26,
  swipeSpan: 1.1,
  swipeGapPx: 8,
  swipeWidthPx: 4,
  /** A swipe's glow stroke is this much wider than its core. */
  swipeGlowMul: 2.4,
  swipeGlowAlpha: 0.35,
} as const;

/** Each sprite's length (or diameter), so a shot reads at the size of a card's corner. */
const SPRITE_PX: Readonly<Record<FxShotSprite, number>> = {
  arrow: 30,
  boulder: 26,
  book: 22,
  seed: 10,
  goo: 16,
  snack: 18,
  coin: 14,
  bill: 20,
  heart: 16,
  emoji: 18,
};

/** The outline every sprite wears, so it reads on a light or a dark board. */
const INK = "#2b1d18";

/** What the sprites' parts measure, as fractions of the sprite's size unless noted. */
const SPRITE_SHAPE = {
  outlinePx: 1.2,
  arrowShaftPx: 2,
  arrowHeadLength: 0.3,
  arrowHeadHalf: 0.17,
  arrowFletchLength: 0.28,
  arrowFletchHalf: 0.17,
  boulderCorners: 8,
  boulderInner: 0.8,
  boulderGlint: 0.22,
  bookHeight: 0.78,
  bookSpine: 0.24,
  bookPageInset: 0.14,
  seedAspect: 0.5,
  gooBody: 0.8,
  gooDrop: 0.4,
  gooDropAt: 0.75,
  gooGlint: 0.2,
  snackChips: 3,
  snackChipAt: 0.5,
  snackChipSize: 0.28,
  snackChipTurn: 0.6,
  coinRim: 0.65,
  coinRimPx: 1,
  billHeight: 0.55,
  billInset: 0.18,
  billSeal: 0.16,
  /** A heart's two curves: the lower control points reach out `heartSide` and sit `heartCurve` below the middle, the upper `heartTop` out and `heartShoulder` above it; the tip hangs `heartTip` below and the cleft dips `heartDip` above. */
  heartSide: 1.2,
  heartCurve: 0.1,
  heartTop: 0.8,
  heartShoulder: 0.9,
  heartTip: 0.9,
  heartDip: 0.35,
  emojiEyeX: 0.28,
  emojiEyeY: 0.2,
  emojiEye: 0.1,
  emojiSmile: 0.5,
  emojiSmileFrom: 0.15,
  emojiSmileTo: 0.85,
  emojiSmilePx: 1.2,
} as const;

/** Anything within this of its end time has ended, so summed frame steps cannot strand a shot. */
const END_EPSILON_MS = 1e-6;
const TAU = Math.PI * 2;
const HALF = 0.5;
const MS_PER_SECOND = 1000;

/** One sprite, bolt head or crescent in flight: a quadratic Bézier from `from` through `c` to `to`. */
type Head = {
  fx: number;
  fy: number;
  cx: number;
  cy: number;
  tx: number;
  ty: number;
  sprite: FxShotSprite | null;
  /** Where it is now, with a wisp's weave. */
  hx: number;
  hy: number;
  lastT: number;
  /** A bolt's past positions, oldest first (x, y pairs). */
  tail: number[];
};

type Flight = {
  shot: FxShot;
  colors: readonly string[];
  preset: FxPreset;
  density: number;
  flightMs: number;
  elapsed: number;
  /** The unit normal across the chord, up the screen: the weave and the fan run along it. */
  nx: number;
  ny: number;
  /** The chord's heading, radians. */
  heading: number;
  heads: Head[];
};

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

/** The flight's trail, if its motion leaves one. */
function trailOf(shot: FxShot): FxPreset | undefined {
  return shot.motion === "arc" || shot.motion === "bolt" ? shot.trail : undefined;
}

export function createShots(options: { particles: ParticleSystem; rng: FxRng }): Shots {
  const { particles, rng } = options;
  const flights: Flight[] = [];

  /** One reusable emit request, so a trail step allocates nothing. */
  const emitBox: FxBox = { x: 0, y: 0, width: 0, height: 0 };
  const trailEmit: EmitOptions = { count: 1, spread: "point", box: emitBox, power: SHOT.trailPower };
  const arrivalEmit: EmitOptions = { count: SHOT.arrivalCount, spread: "point", box: emitBox, power: SHOT.arrivalPower };

  /** A head's place at `t`, written into `at`: the Bézier point, and a wisp's weave across it. */
  const at = { x: 0, y: 0 };
  const place = (fl: Flight, head: Head, t: number): void => {
    const u = 1 - t;
    at.x = u * u * head.fx + 2 * u * t * head.cx + t * t * head.tx;
    at.y = u * u * head.fy + 2 * u * t * head.cy + t * t * head.ty;
    if (fl.shot.motion === "bolt" && fl.shot.wave === true) {
      const weave = Math.sin(t * SHOT.waveCycles * TAU) * SHOT.waveAmpPx;
      at.x += fl.nx * weave;
      at.y += fl.ny * weave;
    }
  };

  const emitTrail = (preset: FxPreset, x: number, y: number): void => {
    const jx = x + (rng() * 2 - 1) * SHOT.trailJitter;
    const jy = y + (rng() * 2 - 1) * SHOT.trailJitter;
    emitBox.x = jx;
    emitBox.y = jy;
    trailEmit.count = 1;
    particles.emit(preset, jx, jy, trailEmit);
  };

  const hot = (fl: Flight): string => fl.colors[0] ?? "#ffffff";
  const body = (fl: Flight): string => fl.colors[1] ?? hot(fl);
  const glow = (fl: Flight): string => fl.colors[2] ?? body(fl);

  /** The sprite, centred on the origin and pointing along +x, in the flight's colours. */
  const drawSprite = (ctx: CanvasRenderingContext2D, fl: Flight, sprite: FxShotSprite): void => {
    const px = SPRITE_PX[sprite];
    const r = px * HALF;
    const S = SPRITE_SHAPE;
    ctx.lineJoin = "round";
    ctx.lineCap = "round";
    ctx.lineWidth = S.outlinePx;
    ctx.strokeStyle = INK;
    switch (sprite) {
      case "arrow": {
        const headLength = px * S.arrowHeadLength;
        const headHalf = px * S.arrowHeadHalf;
        const fletch = px * S.arrowFletchLength;
        const fletchHalf = px * S.arrowFletchHalf;
        ctx.strokeStyle = body(fl);
        ctx.lineWidth = S.arrowShaftPx;
        ctx.beginPath();
        ctx.moveTo(-r, 0);
        ctx.lineTo(r - headLength, 0);
        ctx.stroke();
        ctx.strokeStyle = INK;
        ctx.lineWidth = S.outlinePx;
        ctx.fillStyle = glow(fl);
        ctx.beginPath();
        ctx.moveTo(r, 0);
        ctx.lineTo(r - headLength, -headHalf);
        ctx.lineTo(r - headLength, headHalf);
        ctx.closePath();
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = hot(fl);
        for (const side of [-1, 1]) {
          ctx.beginPath();
          ctx.moveTo(-r + fletch, 0);
          ctx.lineTo(-r, side * fletchHalf);
          ctx.lineTo(-r + fletch * HALF, 0);
          ctx.closePath();
          ctx.fill();
          ctx.stroke();
        }
        return;
      }
      case "boulder": {
        ctx.fillStyle = body(fl);
        ctx.beginPath();
        for (let k = 0; k < S.boulderCorners; k++) {
          const angle = (k / S.boulderCorners) * TAU;
          const radius = r * (k % 2 === 0 ? 1 : S.boulderInner);
          const x = Math.cos(angle) * radius;
          const y = Math.sin(angle) * radius;
          if (k === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        }
        ctx.closePath();
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = hot(fl);
        ctx.beginPath();
        ctx.arc(-r * HALF, -r * HALF, r * S.boulderGlint, 0, TAU);
        ctx.fill();
        return;
      }
      case "book": {
        const height = px * S.bookHeight;
        const spine = px * S.bookSpine;
        const inset = px * S.bookPageInset;
        ctx.fillStyle = body(fl);
        ctx.beginPath();
        ctx.rect(-r, -height * HALF, px, height);
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = glow(fl);
        ctx.beginPath();
        ctx.rect(-r, -height * HALF, spine, height);
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = hot(fl);
        ctx.beginPath();
        ctx.rect(-r + spine + inset * HALF, -height * HALF + inset, px - spine - inset, height - inset * 2);
        ctx.fill();
        return;
      }
      case "seed": {
        ctx.fillStyle = body(fl);
        ctx.beginPath();
        ctx.ellipse(0, 0, r, r * S.seedAspect, 0, 0, TAU);
        ctx.fill();
        ctx.stroke();
        return;
      }
      case "goo": {
        ctx.fillStyle = body(fl);
        ctx.beginPath();
        ctx.arc(0, 0, r * S.gooBody, 0, TAU);
        ctx.fill();
        ctx.stroke();
        ctx.beginPath();
        ctx.arc(-r * S.gooDropAt, 0, r * S.gooDrop, 0, TAU);
        ctx.fill();
        ctx.fillStyle = hot(fl);
        ctx.beginPath();
        ctx.arc(r * S.gooGlint, -r * S.gooGlint, r * S.gooGlint, 0, TAU);
        ctx.fill();
        return;
      }
      case "snack": {
        ctx.fillStyle = body(fl);
        ctx.beginPath();
        ctx.arc(0, 0, r, 0, TAU);
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = glow(fl);
        for (let k = 0; k < S.snackChips; k++) {
          const angle = (k / S.snackChips) * TAU + S.snackChipTurn;
          ctx.beginPath();
          ctx.arc(Math.cos(angle) * r * S.snackChipAt, Math.sin(angle) * r * S.snackChipAt, r * S.snackChipSize, 0, TAU);
          ctx.fill();
        }
        return;
      }
      case "coin": {
        ctx.fillStyle = body(fl);
        ctx.beginPath();
        ctx.arc(0, 0, r, 0, TAU);
        ctx.fill();
        ctx.stroke();
        ctx.strokeStyle = hot(fl);
        ctx.lineWidth = S.coinRimPx;
        ctx.beginPath();
        ctx.arc(0, 0, r * S.coinRim, 0, TAU);
        ctx.stroke();
        return;
      }
      case "bill": {
        const height = px * S.billHeight;
        const inset = height * S.billInset;
        ctx.fillStyle = glow(fl);
        ctx.beginPath();
        ctx.rect(-r, -height * HALF, px, height);
        ctx.fill();
        ctx.stroke();
        ctx.strokeStyle = hot(fl);
        ctx.lineWidth = S.coinRimPx;
        ctx.beginPath();
        ctx.rect(-r + inset, -height * HALF + inset, px - inset * 2, height - inset * 2);
        ctx.stroke();
        ctx.fillStyle = hot(fl);
        ctx.beginPath();
        ctx.arc(0, 0, height * S.billSeal, 0, TAU);
        ctx.fill();
        return;
      }
      case "heart": {
        const side = r * S.heartSide;
        const top = r * S.heartTop;
        const shoulder = r * S.heartShoulder;
        const tip = r * S.heartTip;
        const dip = r * S.heartDip;
        const curve = r * S.heartCurve;
        ctx.fillStyle = body(fl);
        ctx.beginPath();
        ctx.moveTo(0, tip);
        ctx.bezierCurveTo(-side, curve, -top, -shoulder, 0, -dip);
        ctx.bezierCurveTo(top, -shoulder, side, curve, 0, tip);
        ctx.closePath();
        ctx.fill();
        ctx.stroke();
        return;
      }
      case "emoji": {
        ctx.fillStyle = glow(fl);
        ctx.beginPath();
        ctx.arc(0, 0, r, 0, TAU);
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = INK;
        for (const side of [-1, 1]) {
          ctx.beginPath();
          ctx.arc(side * r * S.emojiEyeX, -r * S.emojiEyeY, r * S.emojiEye, 0, TAU);
          ctx.fill();
        }
        ctx.lineWidth = S.emojiSmilePx;
        ctx.beginPath();
        ctx.arc(0, 0, r * S.emojiSmile, Math.PI * S.emojiSmileFrom, Math.PI * S.emojiSmileTo);
        ctx.stroke();
        return;
      }
    }
  };

  const drawArc = (ctx: CanvasRenderingContext2D, fl: Flight, shot: Extract<FxShot, { motion: "arc" }>): void => {
    const t = fl.flightMs > 0 ? Math.min(1, fl.elapsed / fl.flightMs) : 1;
    ctx.globalCompositeOperation = "source-over";
    ctx.globalAlpha = 1;
    fl.heads.forEach((head, index) => {
      if (head.sprite === null) return;
      let angle: number;
      let bob = 0;
      if (shot.turn === "point") {
        // The Bézier's tangent: 2(1 − t)(c − from) + 2t(to − c).
        const u = 1 - t;
        angle = Math.atan2(u * (head.cy - head.fy) + t * (head.ty - head.cy), u * (head.cx - head.fx) + t * (head.tx - head.cx));
      } else if (shot.turn === "spin") {
        angle = fl.elapsed * SHOT.spinPerMs + index * SHOT.spinStagger;
      } else {
        const wave = Math.sin((fl.elapsed / MS_PER_SECOND) * SHOT.floatHz * TAU + index * SHOT.floatStagger);
        angle = wave * SHOT.floatTilt;
        bob = wave * SHOT.floatBobPx;
      }
      ctx.save();
      ctx.translate(head.hx, head.hy + bob);
      ctx.rotate(angle);
      drawSprite(ctx, fl, head.sprite);
      ctx.restore();
    });
  };

  const drawBolt = (ctx: CanvasRenderingContext2D, fl: Flight): void => {
    ctx.globalCompositeOperation = "lighter";
    ctx.lineCap = "round";
    for (const head of fl.heads) {
      const tail = head.tail;
      const points = tail.length / 2;
      for (let k = 1; k < points; k++) {
        const f = k / points;
        ctx.globalAlpha = SHOT.tailAlpha * f;
        ctx.strokeStyle = k === points - 1 ? hot(fl) : body(fl);
        ctx.lineWidth = Math.max(1, SHOT.tailWidth * f);
        ctx.beginPath();
        ctx.moveTo(tail[(k - 1) * 2]!, tail[(k - 1) * 2 + 1]!);
        ctx.lineTo(tail[k * 2]!, tail[k * 2 + 1]!);
        ctx.stroke();
      }
      const discs: readonly (readonly [string, number, number])[] = [
        [glow(fl), SHOT.haloPx, SHOT.haloAlpha],
        [body(fl), SHOT.bodyPx, SHOT.bodyAlpha],
        [hot(fl), SHOT.corePx, 1],
      ];
      for (const [color, radius, alpha] of discs) {
        ctx.globalAlpha = alpha;
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.arc(head.hx, head.hy, radius, 0, TAU);
        ctx.fill();
      }
    }
  };

  const drawBeam = (ctx: CanvasRenderingContext2D, fl: Flight): void => {
    const head = fl.heads[0];
    if (head === undefined) return;
    const t = fl.flightMs > 0 ? Math.min(1, fl.elapsed / fl.flightMs) : 1;
    const fade = 1 - t;
    if (fade <= 0) return;
    ctx.globalCompositeOperation = "lighter";
    ctx.lineCap = "round";
    const colors = [glow(fl), body(fl), hot(fl)];
    SHOT.beamPasses.forEach(([widthMul, alpha], index) => {
      ctx.globalAlpha = fade * alpha;
      ctx.strokeStyle = colors[index] ?? hot(fl);
      ctx.lineWidth = SHOT.beamWidthPx * widthMul;
      ctx.beginPath();
      ctx.moveTo(head.fx, head.fy);
      ctx.lineTo(head.tx, head.ty);
      ctx.stroke();
    });
  };

  const drawSwipe = (ctx: CanvasRenderingContext2D, fl: Flight): void => {
    const head = fl.heads[0];
    if (head === undefined) return;
    ctx.globalCompositeOperation = "lighter";
    ctx.lineCap = "round";
    // The crescents bulge ahead, their centres one radius behind the head along the heading.
    const ux = Math.cos(fl.heading);
    const uy = Math.sin(fl.heading);
    for (let k = -1; k <= 1; k++) {
      const cx = head.hx - ux * SHOT.swipeRadiusPx + fl.nx * k * SHOT.swipeGapPx;
      const cy = head.hy - uy * SHOT.swipeRadiusPx + fl.ny * k * SHOT.swipeGapPx;
      ctx.beginPath();
      ctx.arc(cx, cy, SHOT.swipeRadiusPx, fl.heading - SHOT.swipeSpan, fl.heading + SHOT.swipeSpan);
      ctx.globalAlpha = SHOT.swipeGlowAlpha;
      ctx.strokeStyle = glow(fl);
      ctx.lineWidth = SHOT.swipeWidthPx * SHOT.swipeGlowMul;
      ctx.stroke();
      ctx.globalAlpha = 1;
      ctx.strokeStyle = k === 0 ? hot(fl) : body(fl);
      ctx.lineWidth = SHOT.swipeWidthPx;
      ctx.stroke();
    }
  };

  return {
    add(shot, colors, preset, from, to, flightMs, density) {
      const dx = to.x - from.x;
      const dy = to.y - from.y;
      const chord = Math.hypot(dx, dy);
      // The perpendicular that points up the screen, so every lob rises rather than dips.
      let nx = chord > 0 ? -dy / chord : 0;
      let ny = chord > 0 ? dx / chord : -1;
      if (ny > 0) {
        nx = -nx;
        ny = -ny;
      }
      const arcing = shot.motion === "arc";
      const count = arcing ? Math.max(1, Math.floor(shot.count)) : 1;
      const scatter = arcing && shot.scatter === true;
      const heads: Head[] = [];
      for (let i = 0; i < count; i++) {
        const across = (i - (count - 1) / 2) * SHOT.fanPx;
        let tx = to.x + nx * across;
        let ty = to.y + ny * across;
        let lift = arcing ? SHOT.arcLift * chord : 0;
        if (scatter) {
          tx += (rng() * 2 - 1) * SHOT.scatterPx;
          ty += (rng() * 2 - 1) * SHOT.scatterPx;
          lift *= lerp(SHOT.scatterBendMin, SHOT.scatterBendMax, rng());
        }
        const sprite = arcing ? (shot.sprites[i % shot.sprites.length] ?? null) : null;
        heads.push({
          fx: from.x,
          fy: from.y,
          cx: (from.x + tx) / 2 + nx * lift,
          cy: (from.y + ty) / 2 + ny * lift,
          tx,
          ty,
          sprite,
          hx: from.x,
          hy: from.y,
          lastT: 0,
          tail: [from.x, from.y],
        });
      }
      flights.push({
        shot,
        colors,
        preset,
        density: Math.max(0, density),
        flightMs,
        elapsed: 0,
        nx,
        ny,
        heading: Math.atan2(dy, dx),
        heads,
      });
    },

    step(dtMs, ageMs) {
      const age = Math.max(0, ageMs);
      let w = 0;
      for (const fl of flights) {
        fl.elapsed += age;
        const t = fl.flightMs > 0 ? Math.min(1, fl.elapsed / fl.flightMs) : 1;
        const trail = trailOf(fl.shot);
        const perStep = Math.max(SHOT.trailMinPerStep, dtMs * SHOT.trailPerMs);
        const n = Math.max(1, Math.round(perStep * fl.density));
        for (const head of fl.heads) {
          if (trail !== undefined) {
            for (let k = 1; k <= n; k++) {
              place(fl, head, head.lastT + ((t - head.lastT) * k) / n);
              emitTrail(trail, at.x, at.y);
            }
          }
          place(fl, head, t);
          head.hx = at.x;
          head.hy = at.y;
          head.lastT = t;
          if (fl.shot.motion === "bolt") {
            head.tail.push(at.x, at.y);
            if (head.tail.length > SHOT.tailPoints * 2) head.tail.splice(0, 2);
          }
        }
        if (fl.elapsed < fl.flightMs - END_EPSILON_MS) {
          flights[w++] = fl;
          continue;
        }
        // The shot bursts where it lands; the planner's impact (sparks, splat, shake) lands with it.
        arrivalEmit.count = Math.max(1, Math.round((SHOT.arrivalCount * fl.density) / fl.heads.length));
        for (const head of fl.heads) {
          emitBox.x = head.tx;
          emitBox.y = head.ty;
          particles.emit(fl.preset, head.tx, head.ty, arrivalEmit);
        }
      }
      flights.length = w;
    },

    draw(ctx) {
      if (flights.length === 0) return;
      for (const fl of flights) {
        switch (fl.shot.motion) {
          case "arc":
            drawArc(ctx, fl, fl.shot);
            break;
          case "bolt":
            drawBolt(ctx, fl);
            break;
          case "beam":
            drawBeam(ctx, fl);
            break;
          case "swipe":
            drawSwipe(ctx, fl);
            break;
        }
      }
      ctx.globalAlpha = 1;
      ctx.globalCompositeOperation = "source-over";
    },

    alive: () => flights.length,

    clear() {
      flights.length = 0;
    },
  };
}
