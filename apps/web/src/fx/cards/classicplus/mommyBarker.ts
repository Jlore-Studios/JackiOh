// R3399000: Classic+ Mommy Barker (`classicplus-013`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const kitchenDoor: Entrance = () => [];

export const pancakeTray: CardRecipe = () => null;

export function pancakeTrayFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
