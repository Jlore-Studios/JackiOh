// R3399000: Classic+ KY's Test (`classicplus-042`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe } from "../recipe.ts";

export const slateTest: CardRecipe = () => null;

export function slateTestFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
