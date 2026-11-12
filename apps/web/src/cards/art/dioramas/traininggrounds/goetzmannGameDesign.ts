// R3350000: Training Grounds #40 Goetzmann Game Design (`traininggrounds-040`)'s diorama: the palisade and barracks at the back,
// the yard and its dummy, the cards of the design floating up from the table, the dreamed heroes and ideas breathing above
// them, the table and its napkin bent like a sheet of paper, the lantern, and the glows and pixels twinkling at the front.

import type { Diorama } from "../types.ts";

export const DIORAMA: Diorama = {
  id: "traininggrounds-040",
  slices: [
    { name: "night yard", plane: "far", depth: 0, rest: true, pick: [] },
    { name: "palisade", plane: "far", depth: 0.12, pick: ["scene:far:"] },
    { name: "yard", plane: "middle", depth: 0.26, pick: ["scene:mid:"] },
    { name: "cards", plane: "middle", depth: 0.4, pick: ["scene:subject:card:"], loop: { move: "float", seconds: 5.6 } },
    { name: "dreams", plane: "near", depth: 0.54, pick: ["scene:subject:straw-dummy:", "scene:subject:fireball:", "scene:subject:orb:", "scene:subject:pose-spread:", "scene:subject:outfit-fur:", "scene:subject:head-felinor:", "scene:subject:item-staff:", "scene:subject:pose-raise:", "scene:subject:outfit-stone:", "scene:subject:head-golem:", "emblem"], loop: { move: "breathe", seconds: 8.8, origin: [50, 46] } },
    { name: "table", plane: "near", depth: 0.7, pick: ["scene:near:table:", "scene:near:napkin:"], bend: { axis: "x", deg: 12 } },
    { name: "lantern", plane: "nearest", depth: 0.84, pick: ["scene:near:fence-post:", "scene:near:lantern:"] },
    { name: "glows and pixels", plane: "nearest", depth: 1, pick: ["scene:light:", "scene:particles:"], loop: { move: "twinkle", seconds: 4.2, stagger: true } },
  ],
};
