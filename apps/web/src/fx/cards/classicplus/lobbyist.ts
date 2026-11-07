// R3399000: Classic+ Jlockeed's Lobbyist (`classicplus-048`): STUB, its signature is being drawn.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const briefcaseStride: Entrance = () => [];

export const sealedEnvelope: CardRecipe = () => null;

export function sealedEnvelopeFixture(_view: PlayerView, _instanceId: string, _player: PlayerId): GameEvent[] {
  return [];
}
