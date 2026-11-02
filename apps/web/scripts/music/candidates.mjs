// Audition candidates for the stations' in-game music (SPEC §10.11 "Music", R631). Not shipped:
//
//   node apps/web/scripts/gen-music.mjs --candidates --root <scratchDir>
//
// renders CANDIDATES into a scratch tree (never this app, and with no size budget) for the owner to
// listen to. Each candidate is { id, station, title, blurb, loop, build }, `build` a score like those
// in tracks.mjs. The ones the owner keeps move into tracks.mjs under station ids, keeping the
// `createSong` id they were auditioned under so they render as heard; the rest are deleted. Empty
// between auditions: the last one (two rounds) shipped every station's tracks after the first two.

export const CANDIDATES = [];
