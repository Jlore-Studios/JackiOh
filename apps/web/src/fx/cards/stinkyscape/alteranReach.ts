// R3407000: STUB, to be written (#3407).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const alteranDive: Entrance = () => [];

export const starscapeJump: CardRecipe = () => null;

export function starscapeJumpFixture(_view: PlayerView, _instanceId: string, _player: PlayerId, _defId?: string): GameEvent[] {
  return [];
}

export const edgeFlee: CardRecipe = () => null;

export function edgeFleeFixture(_view: PlayerView, _instanceId: string, _player: PlayerId, _defId?: string): GameEvent[] {
  return [];
}
