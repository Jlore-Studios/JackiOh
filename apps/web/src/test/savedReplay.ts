// Reading a replay file a test saved (R1141000, R1242000): its JSON text, unzipped when it is packed,
// for what must not be in it, and the object it opens as, its steps rebuilt.

import { replayText, unpackedReplay } from "../replay/pack.ts";

export async function savedReplay(file: File): Promise<{ text: string; json: Record<string, unknown> }> {
  const text = await replayText(new Uint8Array(await file.arrayBuffer()));
  return { text, json: unpackedReplay(JSON.parse(text) as Record<string, unknown>) };
}
