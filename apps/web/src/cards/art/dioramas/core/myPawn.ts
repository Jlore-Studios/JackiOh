// R3350000: Core #96 My Pawn (`core-096`)'s diorama: the hooded mastermind looming over the mirrored sea at the back, the glass
// chessboard bent like a floor under the enemy's pieces, the gloved hand and its strings swaying from the sky above the fallen
// hero and the king, and the shards, motes and sparks falling at the front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "core-096",
  slices: [
    { name: "night sea", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "mastermind", plane: "far", depth: 0.14, pick: ["scene:far:pose-stand:", "scene:far:outfit-void:", "scene:far:head-void:", "scene:far:hood:"], loop: { move: "breathe", seconds: 9.6, origin: [50, 20] } },
    { name: "chessboard", plane: "middle", depth: 0.28, pick: ["scene:mid:chessboard:"], bend: { axis: "x", deg: 12 } },
    { name: "enemy pieces", plane: "middle", depth: 0.4, pick: ["scene:mid:chess-piece:", "scene:mid:soldier:"] },
    { name: "puppet hand", plane: "middle", depth: 0.52, pick: ["scene:subject:puppet-hand:", "scene:subject:strings:"], loop: { move: "sway", seconds: 6.8, origin: [50, 2] } },
    { name: "fallen hero", plane: "near", depth: 0.66, pick: ["scene:subject:soldier-fallen:", "scene:subject:chess-piece:", "scene:subject:pose-lunge:", "scene:subject:outfit-layered-plate:", "scene:subject:head-human:", "scene:subject:steel-helmet:", "scene:subject:item-sword:", "emblem"] },
    { name: "near pieces", plane: "nearest", depth: 0.82, pick: ["scene:near:", "scene:light:"] },
    { name: "falling shards", plane: "nearest", depth: 1, pick: ["scene:particles:"], loop: { move: "fall", seconds: 5.6, stagger: true } },
  ],
};
