// The Daily screen's own numbers (#3727, CLAUDE.md rule 9). The rules' numbers (the times per move, the
// urgent hours, the list's sizes) are `crates/shared`'s, read from the WebAssembly module
// (`wasm/index.ts` `dailyConstants`); only the client's pacing is stated here.

/** How often a list's and a board's time left is drawn again: the text counts hours, so a minute is plenty. */
export const DAILY_TICK_MS = 60_000;

/** Where this device keeps its push subscription's address, so sign-out can drop it without waiting on the worker. */
export const DAILY_PUSH_ENDPOINT_KEY = "jackioh.daily.pushEndpoint";

/** The service worker that shows a daily game's push (`public/sw.js`), at the site's root so its scope is the site. */
export const DAILY_SERVICE_WORKER_PATH = "/sw.js";
