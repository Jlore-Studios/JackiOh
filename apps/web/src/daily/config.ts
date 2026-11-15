// The Daily screen's own numbers (#3727, CLAUDE.md rule 9). The rules' numbers (the times per move, the
// urgent hours, the list's sizes) are `crates/shared`'s, read from the WebAssembly module
// (`wasm/index.ts` `dailyConstants`); only the client's pacing is stated here.

/** How often a list's and a board's time left is drawn again: the text counts hours, so a minute is plenty. */
export const DAILY_TICK_MS = 60_000;

/** How often an open daily board and the Daily list read the server again while the tab shows, and (R3205000) once more when it shows again. The social read's `dailyWaiting` counts the games waiting on the player, so a game that ends on the opponent's move, or while the move is the player's, leaves it unchanged (R3840000). A minute is slow on purpose: a move that does change the count is read at once. */
export const DAILY_REREAD_MS = 60_000;

/** Where this device keeps its push subscription's address, so sign-out can drop it without waiting on the worker. */
export const DAILY_PUSH_ENDPOINT_KEY = "jackioh.daily.pushEndpoint";

/** The service worker that shows a daily game's push (`public/sw.js`), at the site's root so its scope is the site. */
export const DAILY_SERVICE_WORKER_PATH = "/sw.js";
