// R3399000: Classic+ Felinor Flagbearer (`classicplus-046`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const bannerPlant: Entrance = () => [];

export const bannerFold: CardRecipe = () => null;

export function bannerFoldFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
