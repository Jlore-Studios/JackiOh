// Handing a made GIF over (#513, R513000): the device's share sheet where it takes a file, a
// download everywhere else. Nothing is uploaded: the GIF is bytes in memory.

import { downloadFile } from "../download.ts";

export function gifFile(bytes: Uint8Array<ArrayBuffer>, name: string): File {
  return new File([bytes], name, { type: "image/gif" });
}

/** Whether the device's share sheet accepts `file`. */
export function canShareFile(file: File): boolean {
  return typeof navigator.share === "function" && typeof navigator.canShare === "function" && navigator.canShare({ files: [file] });
}

/**
 * Shares `file` where the share sheet takes it, and downloads it otherwise. A dismissed sheet
 * rejects (an `AbortError`, as `navigator.share` does) and downloads nothing.
 */
export async function shareGif(file: File): Promise<"shared" | "downloaded"> {
  if (canShareFile(file)) {
    await navigator.share({ files: [file] });
    return "shared";
  }
  downloadFile(file);
  return "downloaded";
}
