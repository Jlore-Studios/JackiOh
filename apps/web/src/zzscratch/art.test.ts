import { it } from "vitest";
import { stinkyscapeArtUri } from "../cards/art/svg.ts";
import { CATALOG } from "../wire/cards.ts";
it("scratch", () => {
  for (const id of ["stinkyscape-109","stinkyscape-110","stinkyscape-111","stinkyscape-148","stinkyscape-148-1","stinkyscape-t-red-narcor"]) {
    const d = CATALOG[id]!;
    const u = stinkyscapeArtUri(id, d.name, d.tags, d.type, false);
    const dec = decodeURIComponent(u);
    console.log(id, u.length, u.includes(id), dec.includes(id), /stinkyscape[a-z]*-\d/.test(u), /stinkyscape[a-z]*-\d/.test(dec), u.slice(0,60));
  }
});
