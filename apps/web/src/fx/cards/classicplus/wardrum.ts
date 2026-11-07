// R3399000: Classic+ Wardrum (`classicplus-037`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const drumLeap: Entrance = () => [];

export const drumBeat: CardRecipe = () => null;

export function drumBeatFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
