// The deck builder's filter and sort, remembered per device (#263). One localStorage key; every
// storage access sits in try/catch, as in the settings stores, because storage can be missing, full
// or throw on access (private mode, sandboxed frames). When it refuses, the workshop's own state
// still holds the choice for this page.
//
// What is read back is checked chip by chip against the lists `filters.ts` offers, so a saved copy
// from an older build (a tag since renamed, a sort key since dropped) loses only what no longer
// exists. "Clear filters" resets the filter, and the saved copy with it: when filter and sort are
// both back at their defaults the key is removed, so nothing a reload restores can hide a card the
// player did not ask to hide. The almanac (R630) browses without one and starts clean every visit.

import type { CardType, Rarity, SetName, Tag } from "@jackioh/shared";

import {
  COST_BUCKETS,
  DEFAULT_FILTER,
  DEFAULT_SORT,
  FILTER_RARITIES,
  FILTER_SETS,
  FILTER_TAGS,
  FILTER_TYPES,
  SORT_KEYS,
  type CostBucket,
  type PoolFilter,
  type PoolSort,
} from "./filters.ts";
import type { StorageLike } from "./sync.ts";

export const SAVED_BROWSE_KEY = "jackioh.deckbuilder.browse.v1";

export type SavedBrowse = { filter: PoolFilter; sort: PoolSort };

type Stored = {
  sets: string[];
  costs: string[];
  types: string[];
  tags: string[];
  rarities: string[];
  search: string;
  ownedOnly: boolean;
  sort: PoolSort;
};

/** The members of `value` that `allowed` offers, in `allowed`'s order. */
function pick<T extends string>(value: unknown, allowed: readonly T[]): ReadonlySet<T> {
  if (!Array.isArray(value)) return new Set<T>();
  return new Set(allowed.filter((item) => value.includes(item)));
}

function sortOf(value: unknown): PoolSort {
  if (typeof value !== "object" || value === null) return DEFAULT_SORT;
  const record = value as Record<string, unknown>;
  const key = SORT_KEYS.find((item) => item === record.key);
  const dir = record.dir === "asc" || record.dir === "desc" ? record.dir : undefined;
  if (key === undefined || dir === undefined) return DEFAULT_SORT;
  return { key, dir };
}

function parse(raw: string): SavedBrowse {
  let data: unknown;
  try {
    data = JSON.parse(raw);
  } catch {
    return { filter: DEFAULT_FILTER, sort: DEFAULT_SORT };
  }
  if (typeof data !== "object" || data === null || Array.isArray(data)) {
    return { filter: DEFAULT_FILTER, sort: DEFAULT_SORT };
  }
  const record = data as Record<string, unknown>;
  return {
    filter: {
      sets: pick<SetName>(record.sets, FILTER_SETS),
      costs: pick<CostBucket>(record.costs, COST_BUCKETS),
      types: pick<CardType>(record.types, FILTER_TYPES),
      tags: pick<Tag>(record.tags, FILTER_TAGS),
      rarities: pick<Rarity>(record.rarities, FILTER_RARITIES),
      search: typeof record.search === "string" ? record.search : DEFAULT_FILTER.search,
      ownedOnly: typeof record.ownedOnly === "boolean" ? record.ownedOnly : DEFAULT_FILTER.ownedOnly,
    },
    sort: sortOf(record.sort),
  };
}

/** True when `filter` constrains nothing more than `DEFAULT_FILTER` does. */
export function isDefaultFilter(filter: PoolFilter): boolean {
  return (
    filter.sets.size === 0 &&
    filter.costs.size === 0 &&
    filter.types.size === 0 &&
    filter.tags.size === 0 &&
    filter.rarities.size === 0 &&
    filter.search === DEFAULT_FILTER.search &&
    filter.ownedOnly === DEFAULT_FILTER.ownedOnly
  );
}

function isDefaultSort(sort: PoolSort): boolean {
  return sort.key === DEFAULT_SORT.key && sort.dir === DEFAULT_SORT.dir;
}

/** The saved filter and sort, or the defaults when nothing usable is saved. */
export function loadBrowse(storage: StorageLike | null): SavedBrowse {
  let raw: string | null;
  try {
    raw = storage?.getItem(SAVED_BROWSE_KEY) ?? null;
  } catch {
    raw = null;
  }
  return raw === null ? { filter: DEFAULT_FILTER, sort: DEFAULT_SORT } : parse(raw);
}

/** Saves the pair, or removes the key when both are at their defaults. A refusal is swallowed. */
export function saveBrowse(storage: StorageLike | null, browse: SavedBrowse): void {
  if (storage === null) return;
  try {
    if (isDefaultFilter(browse.filter) && isDefaultSort(browse.sort)) {
      storage.removeItem(SAVED_BROWSE_KEY);
      return;
    }
    const { filter, sort } = browse;
    const stored: Stored = {
      sets: [...filter.sets],
      costs: [...filter.costs],
      types: [...filter.types],
      tags: [...filter.tags],
      rarities: [...filter.rarities],
      search: filter.search,
      ownedOnly: filter.ownedOnly,
      sort,
    };
    storage.setItem(SAVED_BROWSE_KEY, JSON.stringify(stored));
  } catch {
    // Storage refused: the workshop's state still holds the choice for this page.
  }
}
