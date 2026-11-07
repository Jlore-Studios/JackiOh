// R3399000: Classic+ Support Loser (`classicplus-019-4`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const staffShuffle: Entrance = () => [];

export const sighMend: CardRecipe = () => null;

export function sighMendFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
