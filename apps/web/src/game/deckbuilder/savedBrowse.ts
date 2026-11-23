// The deck builder's filter and sort, remembered per device (#263). One localStorage key; every
// storage access sits in try/catch, because storage can be missing, full or throw on access.
//
// What is read back is checked chip by chip against the lists `filters.ts` offers, so a saved copy
// from an older build loses only what no longer exists. When filter and sort are both back at their
// defaults the key is removed, so nothing a reload restores can hide a card the player did not ask
// to hide. The almanac (R630) browses without one and starts clean every visit.

import type { CardType, KeywordKind, Rarity, SetName, Tag } from "@jackioh/shared";

import {
  COST_BUCKETS,
  DEFAULT_FILTER,
  DEFAULT_SORT,
  FILTER_KEYWORDS,
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
import { isRecord } from "../../wire/json.ts";

export const SAVED_BROWSE_KEY = "jackioh.deckbuilder.browse.v1";

export type SavedBrowse = { filter: PoolFilter; sort: PoolSort };

type Stored = {
  sets: string[];
  costs: string[];
  types: string[];
  tags: string[];
  keywords: string[];
  rarities: string[];
  search: string;
  ownedOnly: boolean;
  sort: PoolSort;
};

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
  if (!isRecord(data)) {
    return { filter: DEFAULT_FILTER, sort: DEFAULT_SORT };
  }
  return {
    filter: {
      sets: pick<SetName>(data.sets, FILTER_SETS),
      costs: pick<CostBucket>(data.costs, COST_BUCKETS),
      types: pick<CardType>(data.types, FILTER_TYPES),
      tags: pick<Tag>(data.tags, FILTER_TAGS),
      keywords: pick<KeywordKind>(data.keywords, FILTER_KEYWORDS),
      rarities: pick<Rarity>(data.rarities, FILTER_RARITIES),
      search: typeof data.search === "string" ? data.search : DEFAULT_FILTER.search,
      ownedOnly: typeof data.ownedOnly === "boolean" ? data.ownedOnly : DEFAULT_FILTER.ownedOnly,
    },
    sort: sortOf(data.sort),
  };
}

/** True when `filter` constrains nothing more than `DEFAULT_FILTER` does. */
export function isDefaultFilter(filter: PoolFilter): boolean {
  return (
    filter.sets.size === 0 &&
    filter.costs.size === 0 &&
    filter.types.size === 0 &&
    filter.tags.size === 0 &&
    filter.keywords.size === 0 &&
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
      keywords: [...filter.keywords],
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
