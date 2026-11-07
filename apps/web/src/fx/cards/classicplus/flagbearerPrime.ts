// R3399000: Classic+ Felinor Flagbearer Prime (`classicplus-046-1`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const greatBanner: Entrance = () => [];

export const bannerShadow: CardRecipe = () => null;

export function bannerShadowFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
