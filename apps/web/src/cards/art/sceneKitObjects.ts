import type { KitPart } from "./scene.ts";

export const OBJECTS_KIT = {} as const satisfies Readonly<Record<string, KitPart>>;
