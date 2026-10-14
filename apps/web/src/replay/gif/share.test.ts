// Handing a made GIF over (#513, R513000): the share sheet where it takes the file, a download
// elsewhere, and nothing sent anywhere. The share API is stubbed on `navigator` as the room link's
// tests stub it (`net/roomLink.test.ts`).

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { REPLAY_GIF_REVOKE_MS } from "../config.ts";
import { canShareFile, gifFile, shareGif } from "./share.ts";

const BYTES = Uint8Array.from([0x47, 0x49, 0x46, 0x38, 0x39, 0x61]);

function stub(name: "share" | "canShare", value: unknown): void {
  Object.defineProperty(navigator, name, { configurable: true, value });
}

let createObjectURL: ReturnType<typeof vi.fn>;
let revokeObjectURL: ReturnType<typeof vi.fn>;
/** The downloads started: the name and the address of each clicked link. */
let downloads: { name: string; href: string }[];

beforeEach(() => {
  vi.useFakeTimers();
  createObjectURL = vi.fn(() => "blob:jackioh-gif");
  revokeObjectURL = vi.fn();
  Object.defineProperty(URL, "createObjectURL", { configurable: true, value: createObjectURL });
  Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: revokeObjectURL });
  downloads = [];
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function click(this: HTMLAnchorElement) {
    downloads.push({ name: this.download, href: this.href });
  });
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  Reflect.deleteProperty(navigator, "share");
  Reflect.deleteProperty(navigator, "canShare");
  Reflect.deleteProperty(URL, "createObjectURL");
  Reflect.deleteProperty(URL, "revokeObjectURL");
});

describe("sharing a made GIF", () => {
  it("gifFile is a GIF file of the bytes", () => {
    const file = gifFile(BYTES, "jackioh-replay-3-9.gif");
    expect(file.name).toBe("jackioh-replay-3-9.gif");
    expect(file.type).toBe("image/gif");
    expect(file.size).toBe(BYTES.length);
  });

  it("shares the GIF as a file where canShare accepts files", async () => {
    const share = vi.fn(() => Promise.resolve());
    const canShare = vi.fn(() => true);
    stub("share", share);
    stub("canShare", canShare);
    const file = gifFile(BYTES, "a.gif");
    expect(canShareFile(file)).toBe(true);
    expect(await shareGif(file)).toBe("shared");
    expect(canShare).toHaveBeenCalledWith({ files: [file] });
    expect(share).toHaveBeenCalledTimes(1);
    expect(share).toHaveBeenCalledWith({ files: [file] });
    expect(downloads, "nothing is downloaded as well").toEqual([]);
    expect(createObjectURL).not.toHaveBeenCalled();
  });

  it("downloads it where canShare refuses files or there is no share sheet", async () => {
    const file = gifFile(BYTES, "b.gif");

    // A share sheet that takes text but not files.
    const share = vi.fn(() => Promise.resolve());
    stub("share", share);
    stub("canShare", () => false);
    expect(canShareFile(file)).toBe(false);
    expect(await shareGif(file)).toBe("downloaded");
    expect(share).not.toHaveBeenCalled();
    expect(downloads).toEqual([{ name: "b.gif", href: "blob:jackioh-gif" }]);
    expect(createObjectURL).toHaveBeenCalledWith(file);

    // A browser with no `canShare` (so no file sharing) or no `share` at all.
    Reflect.deleteProperty(navigator, "canShare");
    expect(await shareGif(file)).toBe("downloaded");
    Reflect.deleteProperty(navigator, "share");
    stub("canShare", () => true);
    expect(canShareFile(file)).toBe(false);
    expect(await shareGif(file)).toBe("downloaded");
    expect(downloads).toHaveLength(3);
    expect(share).not.toHaveBeenCalled();
  });

  it("lets the download's address go a while after the click", async () => {
    await shareGif(gifFile(BYTES, "c.gif"));
    expect(revokeObjectURL).not.toHaveBeenCalled();
    vi.advanceTimersByTime(REPLAY_GIF_REVOKE_MS);
    expect(revokeObjectURL).toHaveBeenCalledWith("blob:jackioh-gif");
  });

  it("a dismissed share sheet rejects and downloads nothing", async () => {
    stub("share", vi.fn(() => Promise.reject(new DOMException("dismissed", "AbortError"))));
    stub("canShare", () => true);
    await expect(shareGif(gifFile(BYTES, "d.gif"))).rejects.toMatchObject({ name: "AbortError" });
    expect(downloads).toEqual([]);
    expect(createObjectURL).not.toHaveBeenCalled();
  });
});
