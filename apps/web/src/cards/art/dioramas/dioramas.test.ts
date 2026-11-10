// R3350000 (#3350): the Mythics' dioramas as data. Every Mythic card and every token printed Mythic has one, in a file of its own
// in its set's directory; each is cut in DIORAMA_SLICES_MIN to DIORAMA_SLICES_MAX slices back to front, holds every piece of its
// card's still picture once on both faces, and runs an idle loop of its own; a slice draws nothing its picture does not (no
// colour, no text) and fits ART_DETAIL_MAX_BYTES, and a moving one moves by transform and opacity alone inside the
// reduced-motion gate (R2715001). The faces that draw them are diorama.test.tsx's.

import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { CATALOG } from "../../../wire/cards.ts";
import {
  DIORAMA_BEND_MAX_DEG,
  DIORAMA_LOOP_SLICES_MIN,
  DIORAMA_SLICES_MAX,
  DIORAMA_SLICES_MIN,
} from "../../constants.ts";
import { artMotionOf } from "../../fullArt.ts";
import { ART_MOVES, type ArtMotion } from "../motion.ts";
import { ART_PLANES, artPlaneUri } from "../pictures.ts";
import { ART_DETAIL_MAX_BYTES } from "../richness.ts";
import { group } from "../stinkyDraw.ts";
import { DIORAMAS, dioramaSliceSvg, dioramaSliceUri, slicePieces } from "./index.ts";
import { matches, picturePieces, splitNodes } from "./pieces.ts";

/**
 * R3350000: the Mythics still to be given a diorama. It only shrinks: a card leaves it when its diorama lands, and none
 * joins it. Empty: every Mythic has one.
 */
const PENDING: readonly string[] = [];

const DATA_PREFIX = "data:image/svg+xml,";
const HERE = dirname(fileURLToPath(import.meta.url));

/** Each shipped set's diorama directory, by its catalog name (fx/cards/'s names). */
const SET_DIRS: Readonly<Record<string, string>> = {
  Core: "core",
  Classic: "classic",
  "Classic+": "classicplus",
  Meditative: "meditative",
  "Meditative+": "meditativeplus",
  Stinkyscape: "stinkyscape",
  "Training Grounds": "traininggrounds",
};

const FACES = [false, true] as const;

function mythicIds(): string[] {
  return Object.values(CATALOG)
    .filter((card) => card.rarity === "Mythic" || card.printedRarity === "Mythic")
    .map((card) => card.id)
    .sort();
}

const ALL = [...DIORAMAS.values()];

function whole(id: string, radiant: boolean, motion: ArtMotion | null = null): string {
  return decodeURIComponent(artPlaneUri(id, "whole", radiant, motion).slice(DATA_PREFIX.length));
}

/** The hex colours a document paints with, lower-cased. */
function hexes(svg: string): Set<string> {
  return new Set([...svg.matchAll(/#[0-9a-fA-F]{3,8}\b/g)].map((match) => match[0].toLowerCase()));
}

/** The properties a CSS text sets. */
function properties(css: string): string[] {
  return [...css.matchAll(/([a-z-]+)\s*:/gu)].map((match) => match[1] ?? "");
}

describe("R3350000 the Mythics' dioramas", () => {
  it("R3350000 every Mythic and every token printed Mythic has a diorama, and the pending list only shrinks", () => {
    const wanted = mythicIds().filter((id) => !PENDING.includes(id));
    expect([...DIORAMAS.keys()].sort()).toEqual(wanted);
    for (const id of PENDING) expect(DIORAMAS.has(id), id).toBe(false);
    expect(PENDING).toEqual([]);
  });

  it("R3350000 each diorama is its card's own file in its set's directory", () => {
    const files = Object.values(SET_DIRS).flatMap((dir) =>
      readdirSync(join(HERE, dir))
        .filter((name) => name.endsWith(".ts") && name !== "index.ts")
        .map((name) => ({ dir, name, text: readFileSync(join(HERE, dir, name), "utf8") })),
    );
    for (const diorama of ALL) {
      const holding = files.filter((file) => file.text.includes(`id: "${diorama.id}"`));
      expect(holding.length, diorama.id).toBe(1);
      const set = CATALOG[diorama.id]?.set ?? "";
      expect(holding[0]?.dir, diorama.id).toBe(SET_DIRS[set]);
      expect(holding[0]?.text, diorama.id).toContain(`// R3350000: ${set} `);
    }
    // No file holds a diorama the registry does not list.
    expect(files.length).toBe(ALL.length);
  });

  // Each diorama's own checks, one card a group, so one card's slip never hides another's.
  for (const { id, slices } of ALL) {
    describe(`R3350000 ${id}'s diorama`, () => {
      it("R3350000 a diorama has DIORAMA_SLICES_MIN to DIORAMA_SLICES_MAX slices, back to front", () => {
        expect(slices.length, id).toBeGreaterThanOrEqual(DIORAMA_SLICES_MIN);
        expect(slices.length, id).toBeLessThanOrEqual(DIORAMA_SLICES_MAX);
        // The farthest slice holds the rest, at depth 0 in the far plane, flat.
        const [back] = slices;
        expect(back, id).toMatchObject({ plane: "far", depth: 0, rest: true, pick: [] });
        expect(back?.bend, id).toBeUndefined();
        expect(slices.filter((slice) => slice.rest === true).length, id).toBe(1);
        expect(new Set(slices.map((slice) => slice.name)).size, id).toBe(slices.length);
        slices.forEach((slice, index) => {
          const where = `${id} ${slice.name}`;
          expect(slice.depth, where).toBeGreaterThanOrEqual(0);
          expect(slice.depth, where).toBeLessThanOrEqual(1);
          if (index > 0) {
            const before = slices[index - 1];
            expect(slice.depth, where).toBeGreaterThan(before?.depth ?? 0);
            expect(ART_PLANES.indexOf(slice.plane), where).toBeGreaterThanOrEqual(ART_PLANES.indexOf(before?.plane ?? "far"));
            expect(slice.pick.length, where).toBeGreaterThan(0);
          }
          if (slice.bend !== undefined) {
            expect(slice.bend.deg, where).not.toBe(0);
            expect(Math.abs(slice.bend.deg), where).toBeLessThanOrEqual(DIORAMA_BEND_MAX_DEG);
          }
          const loop = slice.loop;
          if (loop === undefined) return;
          expect(loop.seconds, where).toBeGreaterThan(0);
          for (const coordinate of loop.origin ?? []) {
            expect(coordinate, where).toBeGreaterThanOrEqual(0);
            expect(coordinate, where).toBeLessThanOrEqual(100);
          }
          // A Stinkyscape subject's group is placed by a transform, so its loop turns about its own box.
          if (slice.pick.some((pattern) => pattern.startsWith("subject:"))) expect(ART_MOVES[loop.move].box, where).toBe("fill");
        });
        expect(slices.filter((slice) => slice.loop !== undefined).length, id).toBeGreaterThanOrEqual(DIORAMA_LOOP_SLICES_MIN);
      });

      it("R3350000 on both faces every piece is in exactly one slice and no slice is empty", () => {
        for (const radiant of FACES) {
          const where = `${id} ${radiant ? "Radiant" : "base"}`;
          const picture = picturePieces(id, radiant);
          expect(picture, where).not.toBeNull();
          for (const piece of picture?.pieces ?? []) {
            const picking = slices.filter((slice) => slice.rest !== true && slice.pick.some((pattern) => matches(pattern, piece.key)));
            expect(picking.length, `${where} ${piece.key}`).toBeLessThanOrEqual(1);
          }
          const sliced = slicePieces(id, radiant) ?? [];
          expect(sliced.length, where).toBe(slices.length);
          sliced.forEach((pieces, index) => expect(pieces.length, `${where} ${slices[index]?.name}`).toBeGreaterThan(0));
          expect(sliced.flat().length, where).toBe(picture?.pieces.length);
          // Every pattern a slice picks by picks something on this face (a Radiant face's rays aside).
          for (const slice of slices) {
            for (const pattern of slice.pick) {
              if (!radiant && pattern.startsWith("rays")) continue;
              expect(picture?.pieces.some((piece) => matches(pattern, piece.key)), `${where} ${slice.name} ${pattern}`).toBe(true);
            }
          }
          // The emblem stands with the subject.
          const emblem = sliced.findIndex((pieces) => pieces.some((piece) => piece.key === "emblem"));
          if (emblem >= 0) {
            const subjects = sliced.flatMap((pieces, index) => (pieces.some((piece) => piece.key.startsWith("scene:subject:") || piece.key.startsWith("own:")) ? [index] : []));
            expect(subjects, `${where} emblem`).toContain(emblem);
          }
        }
      });

      it("R3350000 R2725004 a slice draws no colour its picture does not, no text, and fits ART_DETAIL_MAX_BYTES", () => {
        // Still, on its loop, and carrying the card's moving art (an animated level's, a moving board face's), whose
        // picture's colours its own are.
        const motion = artMotionOf(id);
        for (const radiant of FACES) {
          for (const [label, moving, art] of [["still", false, null], ["looping", true, null], ["moving art", true, motion]] as const) {
            const colours = hexes(whole(id, radiant, art));
            slices.forEach((slice, index) => {
              const where = `${id} ${radiant ? "Radiant" : "base"} ${slice.name} ${label}`;
              const svg = dioramaSliceSvg(id, radiant, index, moving, art);
              for (const colour of hexes(svg)) expect(colours.has(colour), `${where} ${colour}`).toBe(true);
              for (const banned of ["<text", "<title", "href", "<image"]) expect(svg, where).not.toContain(banned);
              const uri = dioramaSliceUri(id, radiant, index, moving, art);
              expect(uri.startsWith(DATA_PREFIX), where).toBe(true);
              expect(uri.length, where).toBeLessThanOrEqual(ART_DETAIL_MAX_BYTES);
            });
          }
        }
      });

      it("R3350000 R2715001 a moving slice's keyframes are transform and opacity alone, gated; a still one has none", () => {
        slices.forEach((slice, index) => {
          const where = `${id} ${slice.name}`;
          expect(dioramaSliceSvg(id, false, index, false), where).not.toContain("<style");
          expect(dioramaSliceSvg(id, false, index, false), where).not.toContain('class="dio"');
          const moving = dioramaSliceSvg(id, false, index, true);
          if (slice.loop === undefined) {
            expect(moving, where).not.toContain("<style");
            return;
          }
          const style = /<style>(.*?)<\/style>/u.exec(moving)?.[1] ?? "";
          expect(style.startsWith("@media (prefers-reduced-motion: no-preference){"), where).toBe(true);
          const inner = style.slice(style.indexOf("{") + 1, style.indexOf("@keyframes"));
          for (const property of properties(inner)) expect(["transform-box", "transform-origin", "animation"], `${where}: ${property}`).toContain(property);
          const frames = style.slice(style.indexOf("@keyframes"));
          expect(frames, where).toContain(`@keyframes art-${slice.loop.move}{`);
          for (const property of properties(frames)) expect(["transform", "opacity"], `${where}: ${property}`).toContain(property);
          expect(moving, where).toContain('<g class="dio"');
        });
        // Carrying the card's moving art, each slice has the picture's own style too, gated as it is, and the nearest the
        // particles. A token with no motion of its own never carries one.
        const motion = artMotionOf(id);
        if (motion === null) return;
        slices.forEach((slice, index) => {
          const where = `${id} ${slice.name} with its art`;
          const svg = dioramaSliceSvg(id, false, index, true, motion);
          const styles = [...svg.matchAll(/<style>(.*?)<\/style>/gu)].map((match) => match[1] ?? "");
          expect(styles.length, where).toBe(slice.loop === undefined ? 1 : 2);
          for (const style of styles) {
            expect(style.startsWith("@media (prefers-reduced-motion: no-preference){"), where).toBe(true);
            const frames = style.slice(style.indexOf("@keyframes"));
            for (const property of properties(frames)) expect(["transform", "opacity"], `${where}: ${property}`).toContain(property);
          }
          expect(svg.includes('class="art-m-particle"'), where).toBe(index === slices.length - 1);
        });
      });
    });
  }

  it("R3350000 a picture's pieces joined are its still picture", () => {
    for (const id of mythicIds()) {
      for (const radiant of FACES) {
        const where = `${id} ${radiant ? "Radiant" : "base"}`;
        const picture = picturePieces(id, radiant);
        if (picture === null) throw new Error(`${where} has no pieces`);
        let body = "";
        let run: string[] = [];
        let inSubject = false;
        const flush = (): void => {
          if (run.length > 0) body += inSubject && picture.transform !== null ? group(picture.transform, run.join("")) : run.join("");
          run = [];
        };
        for (const piece of picture.pieces) {
          if (piece.inSubject !== inSubject) {
            flush();
            inSubject = piece.inSubject;
          }
          run.push(piece.markup);
        }
        flush();
        expect(`${picture.head}${body}</svg>`, where).toBe(whole(id, radiant));
        // A scene's keys add up: the sky, the glow, the Radiant rays, a ridge a shape, the emblem and the vignette.
        const keys = picture.pieces.map((piece) => piece.key);
        expect(new Set(keys).size, where).toBe(keys.length);
        expect(keys.includes("rays") || keys.some((key) => key.startsWith("rays:")), where).toBe(radiant);
      }
    }
  });

  it("R3350000 splitNodes keeps each top-level element whole, and throws on text outside one", () => {
    expect(splitNodes('<g a="1"><path d="M0 0"/><g><rect/></g></g><circle r="2"/>')).toEqual(['<g a="1"><path d="M0 0"/><g><rect/></g></g>', '<circle r="2"/>']);
    expect(() => splitNodes("<g></g>loose")).toThrow();
    expect(() => splitNodes("<g>")).toThrow();
  });

  it("R3350000 a slice picks a key, a prefix or a run", () => {
    expect(matches("emblem", "emblem")).toBe(true);
    expect(matches("scene:mid:gate:", "scene:mid:gate:3")).toBe(true);
    expect(matches("scene:mid:gate:", "scene:mid:gates:3")).toBe(false);
    expect(matches("subject:2-4", "subject:3")).toBe(true);
    expect(matches("subject:2-4", "subject:5")).toBe(false);
    expect(matches("subject:2-4", "sky:3")).toBe(false);
  });

  it("R3350000 every diorama's idle loop is its own", () => {
    const seen = new Map<string, string>();
    for (const { id, slices } of ALL) {
      const key = JSON.stringify(slices.map((slice) => slice.loop ?? null));
      expect(seen.get(key), id).toBeUndefined();
      seen.set(key, id);
    }
  });
});
