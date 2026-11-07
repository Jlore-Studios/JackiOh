// R3399000: Classic+ Legion of the Hungry (`classicplus-012-7`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe } from "../recipe.ts";

export const hungryPot: CardRecipe = () => null;

export function hungryPotFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
