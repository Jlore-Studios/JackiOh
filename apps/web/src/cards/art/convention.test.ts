// R660: the real-art delivery convention (convention.ts, ART.md). The first block holds the real
// `apps/web/public/art/` and `ART_MANIFEST` to it, so an artist's delivery is a file plus a
// manifest line and CI says what is wrong with either. The rest prove each refusal on files made up
// here: three real WebP files, one per bitstream (encoded by ImageMagick, base64 below), and bytes
// patched from them.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { ART_MANIFEST, artFileName, artUrl, type ArtManifest } from "./manifest.ts";
import { ART_MAX_BYTES, ART_SIDE_PX, artDeliveryProblems, artFileProblem, webpSize } from "./convention.ts";

const ART_DIRECTORY = join(dirname(fileURLToPath(import.meta.url)), "../../../public/art");
const CATALOG_IDS: ReadonlySet<string> = new Set(Object.keys(CATALOG));

/** A lossy (VP8) 512x512 file. */
const LOSSY_512 = "UklGRmICAABXRUJQVlA4IFYCAAAwOwCdASoAAgACPpFIoU0lpCMiIAgAsBIJaW7hd2EaHAAAE94JyHvtk5D32ych77ZOQ99snIe+2TkPfbJyHvtk5D32ych77ZOQ99snIiB5OQ99snIe+2TkPfbKBl4uTkPfbJyHvtk5D34AgHvtk5D32ych77ZOREDych77ZOQ99snIe+2UDLxcnIe+2TkPfbJyHvwBAPfbJyHvtk5D32yciIHk5D32ych77ZOQ99soGXi5OQ99snIe+2TkPfgCAe+2TkPfbJyHvtk5EQPJyHvtk5D32ych77ZQMvFych77ZOQ99snIe/AEA99snIe+2TkPfbJyIgeTkPfbJyHvtk5D32ygZeLk5D32ych77ZOQ9+AIB77ZOQ99snIe+2TkRA8nIe+2TkPfbJyHvtlAy8XJyHvtk5D32ych78AQD32ych77ZOQ99snIiB5OQ99snIe+2TkPfbKBl4uTkPfbJyHvtk5D34AgHvtk5D32ych77ZOREDych77ZOQ99snIe+2UDLxcnIe+2TkPfbJyHvwBAPfbJyHvtk5D32yciIHk5D32ych77ZOQ99soGXi5OQ99snIe+2TkPfgCAe+2TkPfbJyHvtk5EQPJyHvtk5D32ych77ZQMvFych77ZOQ99snIbwAD+/68UX//Ys5bAvH//+5wP+5wP+5wP424WMlNoFlRJsBAh5vmKh8xzvmOd8xzvmOd8xzvmOd8xzvmOd8xzvmOd8xzvmOd8xzvmOd8xzvmOd8xzvmOd8xzvmOd8xzvmOd8xzvmOd8xzvmOd8xzvmOd8wAAA";
/** A lossless (VP8L) 512x384 file. */
const LOSSLESS_512_BY_384 = "UklGRioAAABXRUJQVlA4TB4AAAAv/8FfAAcQ0f/+BwQCyf7cOxTR/4z//Oc///nP/wE=";
/** An extended (VP8X, with alpha) 300x512 file. */
const EXTENDED_300_BY_512 = "UklGRoYBAABXRUJQVlA4WAoAAAAQAAAAKwEA/wEAQUxQSAoAAAABB1DAiAhERP8DVlA4IFYBAAAwJACdASosAQACPpFIoU0lpCMiIAgAsBIJaW7hd2Ee3AAAE9gHvtk5D32ych77ZOQ99snIe+2TkPfbJyHvtk5D32ych77ZOQ99snIe+2TkPfbJyHvtk5D32ych77ZOQ99snIe+2TkPfbJyHvtk5D32ych77ZOQ99snIe+2TkPfbJyHvtk5D32ych77ZOQ99snIe+2TkPfbJyHvtk5D32ych77ZOQ99snIe+2TkPfbJyHvtk5D32ych77ZOQ99snIe+2TkPfbJyHvtk5D32ych77ZOQ99snIe+2TkPfbJyHvtk5D32ych77ZOQ99snIe+2TkPfbJyHvtk5D32ych77ZOQ99snIe+2TkPfbJyHvtk5D32ych77ZOQ99snIe+2TkPfbJyHvtk4/gAAP7/r+uf/623fze8D//8Wh1odaGiRXdeoxgQAAAAAAAAAAAAAAAAAAAAAAA=";

function bytesOf(base64: string): Uint8Array {
  return new Uint8Array(Buffer.from(base64, "base64"));
}

/** The files in the art directory by name, or none when the directory does not exist yet. */
function deliveredFiles(): Map<string, Uint8Array> {
  const files = new Map<string, Uint8Array>();
  if (!existsSync(ART_DIRECTORY)) return files;
  for (const name of readdirSync(ART_DIRECTORY)) {
    files.set(name, new Uint8Array(readFileSync(join(ART_DIRECTORY, name))));
  }
  return files;
}

describe("R660 the art directory meets the convention", () => {
  it("R660 every manifest line names a catalog card whose files exist and meet the convention, and every file is listed", () => {
    expect(artDeliveryProblems(ART_MANIFEST, deliveredFiles(), CATALOG_IDS)).toEqual([]);
  });

  it("R660 the convention's numbers: square 512 px files of at most 96 KiB", () => {
    expect(ART_SIDE_PX).toBe(512);
    expect(ART_MAX_BYTES).toBe(98_304);
  });

  it("R660 a face's file is named for its card id, the Radiant face with -radiant, and artUrl asks for that name", () => {
    expect(artFileName("core-002", false)).toBe("core-002.webp");
    expect(artFileName("core-002", true)).toBe("core-002-radiant.webp");
    const manifest: ArtManifest = { "core-002": { base: true, radiant: true } };
    expect(artUrl("core-002", false, manifest)?.src.endsWith("art/core-002.webp")).toBe(true);
    expect(artUrl("core-002", true, manifest)?.src.endsWith("art/core-002-radiant.webp")).toBe(true);
  });
});

describe("R660 webpSize reads the picture size of each WebP bitstream", () => {
  it("R660 lossy, lossless and extended files", () => {
    expect(webpSize(bytesOf(LOSSY_512))).toEqual({ width: 512, height: 512 });
    expect(webpSize(bytesOf(LOSSLESS_512_BY_384))).toEqual({ width: 512, height: 384 });
    expect(webpSize(bytesOf(EXTENDED_300_BY_512))).toEqual({ width: 300, height: 512 });
  });

  it("R660 anything else is not a WebP file", () => {
    const png = new Uint8Array(64);
    png.set([0x89, 0x50, 0x4e, 0x47]);
    expect(webpSize(png)).toBeNull();
    expect(webpSize(new Uint8Array(0))).toBeNull();
    expect(webpSize(bytesOf(LOSSY_512).subarray(0, 20))).toBeNull();
    const unknownChunk = bytesOf(LOSSY_512);
    unknownChunk.set([0x41, 0x4c, 0x50, 0x48], 12); // "ALPH" where the bitstream's tag stands
    expect(webpSize(unknownChunk)).toBeNull();
  });
});

describe("R660 artFileProblem", () => {
  it("R660 accepts a square 512 px WebP under the budget", () => {
    expect(artFileProblem(bytesOf(LOSSY_512))).toBeNull();
  });

  it("R660 refuses the wrong size, the wrong format and a file over the budget", () => {
    expect(artFileProblem(bytesOf(LOSSLESS_512_BY_384))).toBe("is 512x384, not 512x512");
    expect(artFileProblem(bytesOf(EXTENDED_300_BY_512))).toBe("is 300x512, not 512x512");
    expect(artFileProblem(new Uint8Array(64))).toBe("is not a WebP file");
    const heavy = new Uint8Array(ART_MAX_BYTES + 1);
    heavy.set(bytesOf(LOSSY_512));
    expect(artFileProblem(heavy)).toBe(`is ${String(ART_MAX_BYTES + 1)} bytes, over ${String(ART_MAX_BYTES)}`);
  });
});

describe("R660 artDeliveryProblems", () => {
  const good = bytesOf(LOSSY_512);

  it("R660 a complete delivery has no problems, a base face alone or both faces", () => {
    const manifest: ArtManifest = { "core-002": { base: true, radiant: true }, "core-003": { base: true } };
    const files = new Map([
      ["core-002.webp", good],
      ["core-002-radiant.webp", good],
      ["core-003.webp", good],
    ]);
    expect(artDeliveryProblems(manifest, files, CATALOG_IDS)).toEqual([]);
  });

  it("R660 names a missing file, a file that breaks the convention and a file no line lists", () => {
    const manifest: ArtManifest = { "core-002": { base: true, radiant: true } };
    const files = new Map([
      ["core-002.webp", bytesOf(LOSSLESS_512_BY_384)],
      ["core-009.webp", good],
    ]);
    expect(artDeliveryProblems(manifest, files, CATALOG_IDS)).toEqual([
      "core-002-radiant.webp: listed in the manifest but missing",
      "core-002.webp: is 512x384, not 512x512",
      "core-009.webp: in the art directory but not in the manifest",
    ]);
  });

  it("R660 names a line for an id the catalog lacks and a line that lists neither face", () => {
    const manifest: ArtManifest = { "core-999": { base: true }, "core-002": {} };
    const files = new Map([["core-999.webp", good]]);
    expect(artDeliveryProblems(manifest, files, CATALOG_IDS)).toEqual([
      "manifest: core-002 lists neither face",
      "manifest: core-999 is not a catalog card or token",
    ]);
  });
});
