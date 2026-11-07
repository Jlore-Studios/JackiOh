// R3399000: Classic+ Mythic Grape (`classicplus-065-5`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe } from "../recipe.ts";

export const vineRipen: CardRecipe = () => null;

export function vineRipenFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
