// R3399000: Classic+ The Mother Pancake (`classicplus-012`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const pancakeStack: Entrance = () => [];

export const pancakeFlip: CardRecipe = () => null;

export function pancakeFlipFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
