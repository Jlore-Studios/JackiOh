// R3399000: Classic+ KY's Gift (`classicplus-042-1`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const giftDesk: Entrance = () => [];

export const giftBoxes: CardRecipe = () => null;

export function giftBoxesFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
