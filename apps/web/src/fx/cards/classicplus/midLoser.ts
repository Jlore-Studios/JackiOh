// R3399000: Classic+ Mid Loser (`classicplus-019-3`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const spotlightPose: Entrance = () => [];

export const coinCall: CardRecipe = () => null;

export function coinCallFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
