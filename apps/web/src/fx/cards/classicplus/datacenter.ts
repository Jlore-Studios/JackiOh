// R3399000: Classic+ Claude's Datacenter (`classicplus-078`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const obeliskRise: Entrance = () => [];

export const printSlot: CardRecipe = () => null;

export function printSlotFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
