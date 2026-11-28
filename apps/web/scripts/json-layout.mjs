// R5289000: JSON laid out a record per line, crates/tools/src/json_layout.rs line for line, for the one
// file of the card data a script of the web writes: src/audio/voice-manifest.json (gen-voice.mjs).
//
// `layout(json, depth)` writes the JSON text `json` so that each container nested less than `depth` deep
// opens and closes on lines of its own, with each of its members on a line of its own (`"key": value`,
// two spaces a level), when it is the root and not empty, or when it holds an object or an array; any
// other container, every record at `depth` among them, is written compact on its member's line. The text
// ends with a newline. Only white space outside strings changes, so the value and key order are kept.
//
// `cargo jackioh format-json --check` holds the committed manifest to the Rust's layout at
// VOICE_MANIFEST_DEPTH, so the two must give the same bytes: src/net/json-layout.test.ts runs this on the
// Rust's own cases.

/** The manifest's records: each line's entry under `"files"` (format_json.rs VOICE_MANIFEST_DEPTH). */
export const VOICE_MANIFEST_DEPTH = 2;

const INDENT = "  ";

/** @param {string} c */
function isSpace(c) {
  return c === " " || c === "\n" || c === "\r" || c === "\t";
}

/** @param {string} c */
function isMark(c) {
  return c === "{" || c === "}" || c === "[" || c === "]" || c === "," || c === ":";
}

/** @param {string} token */
function opens(token) {
  return token === "{" || token === "[";
}

/** @param {string} token */
function closes(token) {
  return token === "}" || token === "]";
}

/**
 * Each mark, each string whole (its escapes kept), each other literal; white space dropped.
 * @param {string} json
 * @returns {string[]}
 */
function tokens(json) {
  const out = [];
  let at = 0;
  while (at < json.length) {
    const c = json[at];
    const start = at;
    at += 1;
    if (isSpace(c)) continue;
    if (c === '"') {
      let escaped = false;
      while (at < json.length) {
        const d = json[at];
        at += 1;
        if (escaped) escaped = false;
        else if (d === "\\") escaped = true;
        else if (d === '"') break;
      }
    } else if (!isMark(c)) {
      while (at < json.length) {
        const d = json[at];
        if (isSpace(d) || isMark(d) || d === '"') break;
        at += 1;
      }
    }
    out.push(json.slice(start, at));
  }
  return out;
}

/**
 * @param {string[]} out
 * @param {number} level
 */
function newLine(out, level) {
  out.push("\n", INDENT.repeat(level));
}

/**
 * `json` laid out a record per line, its records `depth` levels down.
 * @param {string} json
 * @param {number} depth
 * @returns {string}
 */
export function layout(json, depth) {
  const list = tokens(json);
  const out = [];
  // For each container open at this token, whether it is laid out a member per line.
  const open = [];
  for (let at = 0; at < list.length; at += 1) {
    const token = list[at];
    if (opens(token)) {
      let wraps = false;
      if (open.length < depth) {
        if (open.length === 0) {
          wraps = at + 1 < list.length && !closes(list[at + 1]);
        } else {
          let next = at + 1;
          while (next < list.length && !opens(list[next]) && !closes(list[next])) next += 1;
          wraps = next < list.length && opens(list[next]);
        }
      }
      out.push(token);
      open.push(wraps);
      if (wraps) newLine(out, open.length);
    } else if (closes(token)) {
      if (open.pop() === true) newLine(out, open.length);
      out.push(token);
    } else {
      out.push(token);
      if (open.length > 0 && open[open.length - 1]) {
        if (token === ",") newLine(out, open.length);
        else if (token === ":") out.push(" ");
      }
    }
  }
  out.push("\n");
  return out.join("");
}
