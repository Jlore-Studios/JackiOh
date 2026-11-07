// R3399000: Classic+ Frostspatula (`classicplus-012-8`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const griddleFreeze: Entrance = () => [];

export const spatulaFlip: CardRecipe = () => null;

export function spatulaFlipFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
