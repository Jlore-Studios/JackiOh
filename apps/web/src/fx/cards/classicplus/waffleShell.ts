// R3399000: Classic+ Anti-Waffle Shell (`classicplus-012-5`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const waffleIron: Entrance = () => [];

export const waffleGrid: CardRecipe = () => null;

export function waffleGridFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
