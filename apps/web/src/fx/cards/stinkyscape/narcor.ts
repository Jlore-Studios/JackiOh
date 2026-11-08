// R3407000: STUB, to be written (#3407).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import type { CardRecipe, Entrance } from "../recipe.ts";

export const narcorDust: CardRecipe = () => null;

export function narcorDustFixture(_view: PlayerView, _instanceId: string, _player: PlayerId, _defId?: string): GameEvent[] {
  return [];
}

export const redNarcorHum: CardRecipe = () => null;

export function redNarcorHumFixture(_view: PlayerView, _instanceId: string, _player: PlayerId, _defId?: string): GameEvent[] {
  return [];
}
