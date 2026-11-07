// R3399000: Classic+ Frozen Wastes (`classicplus-012-6`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe } from "../recipe.ts";

export const whiteout: CardRecipe = () => null;

export function whiteoutFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
