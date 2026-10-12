// Scans apps/web/src for t("…") keys and for user-visible English not yet wrapped.
// usage: node scan.mjs <repo> [--missing | --keys | --unwrapped] [path-filter]
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";

const repo = process.argv[2];
const mode = process.argv[3] ?? "--missing";
const filter = process.argv[4] ?? "";
const require = createRequire(path.join(repo, "package.json"));
const ts = require("typescript");

const src = path.join(repo, "apps/web/src");
const dict = JSON.parse(fs.readFileSync(path.join(src, "i18n/zh-CN.json"), "utf8"));
// The area agents' fragments count as entries too (the lead folds them into zh-CN.json).
const partsDir = path.join(repo, "i18n-parts");
if (fs.existsSync(partsDir)) {
  for (const name of fs.readdirSync(partsDir)) {
    if (!name.endsWith(".json")) continue;
    try {
      Object.assign(dict, { ...JSON.parse(fs.readFileSync(path.join(partsDir, name), "utf8")), ...dict });
    } catch (e) {
      console.error(`bad fragment ${name}: ${e.message}`);
    }
  }
}

function walk(dir, out) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "wasm" || entry.name === "__snapshots__" || entry.name === "generated") continue;
      walk(full, out);
    } else if (/\.(ts|tsx)$/.test(entry.name) && !/\.test\.(ts|tsx)$/.test(entry.name) && !entry.name.endsWith(".d.ts")) {
      out.push(full);
    }
  }
  return out;
}

const files = walk(src, []).filter((f) => f.includes(filter));
const keys = new Map(); // key -> [loc]
const unwrapped = [];
const ATTRS = new Set(["aria-label", "title", "placeholder", "alt", "label", "hint", "description"]);

function insideT(node) {
  for (let p = node.parent; p; p = p.parent) {
    if (ts.isCallExpression(p) && ts.isIdentifier(p.expression) && p.expression.text === "t") return true;
  }
  return false;
}

for (const file of files) {
  const text = fs.readFileSync(file, "utf8");
  const sf = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, file.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  const rel = path.relative(src, file);
  const visit = (node) => {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression) && node.expression.text === "t") {
      const arg = node.arguments[0];
      if (arg && (ts.isStringLiteral(arg) || ts.isNoSubstitutionTemplateLiteral(arg))) {
        const line = sf.getLineAndCharacterOfPosition(node.getStart()).line + 1;
        const list = keys.get(arg.text) ?? [];
        list.push(`${rel}:${line}`);
        keys.set(arg.text, list);
      }
    }
    if (ts.isJsxText(node)) {
      const s = node.text.replace(/\s+/g, " ").trim();
      if (/[A-Za-z]{2,}/.test(s)) {
        const line = sf.getLineAndCharacterOfPosition(node.getStart()).line + 1;
        unwrapped.push(`${rel}:${line}\tjsx\t${s}`);
      }
    }
    if (ts.isJsxAttribute(node) && node.initializer && ts.isStringLiteral(node.initializer)) {
      const name = node.name.getText();
      if (ATTRS.has(name) && /[A-Za-z]{2,}/.test(node.initializer.text)) {
        const line = sf.getLineAndCharacterOfPosition(node.getStart()).line + 1;
        unwrapped.push(`${rel}:${line}\t${name}\t${node.initializer.text}`);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sf);
}

if (mode === "--keys") {
  for (const [k, locs] of keys) console.log(`${JSON.stringify(k)}\t${locs.join(" ")}`);
} else if (mode === "--missing") {
  let n = 0;
  for (const [k, locs] of keys) {
    if (!Object.hasOwn(dict, k)) {
      n++;
      console.log(`${JSON.stringify(k)}\t${locs.join(" ")}`);
    }
  }
  console.error(`${n} missing of ${keys.size} keys`);
} else if (mode === "--unwrapped") {
  for (const u of unwrapped) console.log(u);
  console.error(`${unwrapped.length} unwrapped`);
} else if (mode === "--unused") {
  for (const k of Object.keys(dict)) if (!keys.has(k)) console.log(JSON.stringify(k));
}
