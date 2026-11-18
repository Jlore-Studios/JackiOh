// Shared lazy-glob helpers for the patch and card-table sidecars (R388, #4180).
// Kept here, without any `import.meta.glob` of its own, so `cards/chinese.ts` can use them
// without pulling in `patches/source.ts`'s top-level patches glob: the GitHub-releases script
// (`scripts/github-releases.ts`, run under `tsx` where `import.meta.glob` does not exist) reads
// `ChangeList.tsx`, which reads `cards/chinese.ts`, and must not evaluate a patches glob.

export type Loader = () => Promise<unknown>;

export function fileStem(path: string): string {
  const name = path.slice(path.lastIndexOf("/") + 1);
  return name.endsWith(".json") ? name.slice(0, -".json".length) : name;
}

export function loadersByStem(glob: Readonly<Record<string, Loader>>): Record<string, Loader> {
  const out: Record<string, Loader> = {};
  for (const [path, loader] of Object.entries(glob)) out[fileStem(path)] = loader;
  return out;
}
