// R3399000: Classic+ Call to Chaos (Classic+ Edition) (`classicplus-073`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe } from "../recipe.ts";

export const chaosLever: CardRecipe = () => null;

export function chaosLeverFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
