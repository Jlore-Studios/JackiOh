//! The card catalog is static data shipped with the client (§9.4), so the engine reads it from a
//! registry, a `OnceLock` set once by `register_catalog` (SURFACE §3, §6.6, §8): a mutable static
//! would let one game see another's cards when the tools run games on many threads. GameState stays
//! serializable; its fused and crafted definitions (`transient_defs`) win over the registry, and a
//! digest id's ingredients (R468) are read off its fused definition, so those readers take the state.
//!
//! R767000: a replay folds under the catalog version its game was played on (`with_shipped`, for one
//! call on the calling thread); every reader here and `scripts::registered_entry` answer from it first.
//! R1329000: so does every compiled table the engine reads while it folds, or a ruling says why not;
//! the code blocks (no final hash holds them) and code lines (no card reads a count) stay the build's.
//! R2173000 (R1846003's obligation): the BUSTED and complicated lists (R1846000) answer from the shipped
//! catalog when its version kept lists of its own, else from the build's, which no game on it could read.

use std::cell::Cell;
use std::cmp::Ordering;
use std::sync::OnceLock;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::config::{
    DORMANT_STARSHIP_DEF_ID, GLITCH_DEF_ID, GLITCH_ODDS_DENOMINATOR, GLITCH_ODDS_PER_SYSTEM_PLAY, GRAPE_ODDS,
    POOL_TOKEN_TAGS, RollOdds,
};
use crate::rng::Rng;
use crate::script::CardScripts;
use crate::state::GameState;
use crate::wire::{
    AttackHealth, CATALOG_SETS, CardCost, CardDef, CardDefs, CardType, CatalogQuery, CostRange,
    FusedIngredient, KeywordKind, OneOrMany, Rarity, SetName, Tag, has_keyword, set_ships,
};

/// A hasher for the registries' card ids: short strings looked up thousands of times a turn, where
/// SipHash's defense against chosen keys buys nothing (the keys are the catalog's own). A map's
/// iteration order is its insertion order whatever its hasher, so this changes speed and nothing else.
#[derive(Default, Clone, Copy)]
pub struct IdHasher(u64);

impl IdHasher {
    fn mix(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

impl std::hash::Hasher for IdHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut words = bytes.chunks_exact(8);
        for word in &mut words {
            let mut eight = [0u8; 8];
            eight.copy_from_slice(word);
            self.mix(u64::from_le_bytes(eight));
        }
        let rest = words.remainder();
        if !rest.is_empty() {
            let mut eight = [0u8; 8];
            eight[..rest.len()].copy_from_slice(rest);
            self.mix(u64::from_le_bytes(eight) ^ ((rest.len() as u64) << 56));
        }
    }
    fn write_u8(&mut self, byte: u8) {
        self.mix(u64::from(byte));
    }
    fn write_usize(&mut self, word: usize) {
        self.mix(word as u64);
    }
    fn write_u64(&mut self, word: u64) {
        self.mix(word);
    }
    /// splitmix64's finaliser, so the low bits a table indexes by depend on every byte.
    fn finish(&self) -> u64 {
        let mut hash = self.0;
        hash = (hash ^ (hash >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        hash = (hash ^ (hash >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        hash ^ (hash >> 31)
    }
}

pub type IdHash = std::hash::BuildHasherDefault<IdHasher>;

/// The registered catalog, its version and where each id sits in it (`find_def` reads through it).
struct Registered {
    defs: CardDefs,
    version: String,
    index: indexmap::IndexMap<String, usize, IdHash>,
    win_rates: crate::win_rates::WinRateTable,
    code_blocks: CodeBlocks,
}

/// R1183004 (Meditative+ #25 KY's Intuition): each card's code block, by catalog id, sorted by id —
/// the excerpt of its own script `crates/cards/build.rs` compiles in (`code_block_of`).
pub type CodeBlocks = &'static [(&'static str, &'static str)];

static REGISTERED: OnceLock<Registered> = OnceLock::new();

/// What `registered_catalog` answers before anything is registered.
static EMPTY: OnceLock<CardDefs> = OnceLock::new();

/// SURFACE §8: the testkit's thread-local catalog, when the calling thread's test set one.
#[cfg(feature = "testkit")]
fn test_override() -> Option<&'static CardDefs> {
    crate::testkit::scenario::catalog_override()
}

#[cfg(not(feature = "testkit"))]
fn test_override() -> Option<&'static CardDefs> {
    None
}

/// The catalog every reader searches: the shipped one a replay is folding under (R767000), else a
/// test's override, else the registered one, else none.
fn registered() -> &'static CardDefs {
    if let Some(shipped) = folding() {
        return &shipped.defs;
    }
    if let Some(defs) = test_override() {
        return defs;
    }
    match REGISTERED.get() {
        Some(registered) => &registered.defs,
        None => EMPTY.get_or_init(CardDefs::new),
    }
}

/// Registers the catalog with its compiled win-rate table (ME-STATS, MD-D1) and code blocks (R1183004).
/// Set once per process: a second call is ignored (SURFACE §3); a test that needs another catalog sets
/// the testkit's override instead (SURFACE §8).
pub fn register_catalog(
    defs: CardDefs,
    catalog_version: &str,
    win_rates: crate::win_rates::WinRateTable,
    code_blocks: CodeBlocks,
) {
    let index = defs.keys().enumerate().map(|(at, id)| (id.clone(), at)).collect();
    let _ = REGISTERED.set(Registered {
        defs,
        version: catalog_version.to_string(),
        index,
        win_rates,
        code_blocks,
    });
}

/// R1183004: a card's code block — the testkit's override first, then the compiled table, never a
/// shipped catalog's (R1329000) — or `None` for a card with none (a token, a fused or crafted
/// definition, a fixture).
pub fn code_block_of(def_id: &str) -> Option<&'static str> {
    #[cfg(feature = "testkit")]
    if let Some(blocks) = crate::testkit::scenario::code_blocks_override() {
        return find_code_block(blocks, def_id);
    }
    find_code_block(REGISTERED.get()?.code_blocks, def_id)
}

fn find_code_block(blocks: CodeBlocks, def_id: &str) -> Option<&'static str> {
    blocks
        .binary_search_by(|(id, _)| (*id).cmp(def_id))
        .ok()
        .map(|at| blocks[at].1)
}

/// ME-STATS: the win-rate table CN Tech reads: a shipped catalog's while a replay folds under it
/// (R1266000), else the testkit's override, else the registered table, else an empty one.
pub fn registered_win_rates() -> &'static crate::win_rates::WinRateTable {
    if let Some(shipped) = folding() {
        return &shipped.win_rates;
    }
    #[cfg(feature = "testkit")]
    if let Some(table) = crate::testkit::scenario::win_rates_override() {
        return table;
    }
    match REGISTERED.get() {
        Some(registered) => &registered.win_rates,
        None => EMPTY_WIN_RATES.get_or_init(crate::win_rates::WinRateTable::default),
    }
}

static EMPTY_WIN_RATES: OnceLock<crate::win_rates::WinRateTable> = OnceLock::new();

pub fn registered_catalog() -> &'static CardDefs {
    registered()
}

/// Tokens and cards are also reachable by their §5 index ("43", "51.1", "T-rush") — within their set,
/// since an index is unique only there (B2.2: Core's "43" and Classic's "43" are two cards).
pub fn def_by_index(set: SetName, index: &str) -> Option<&'static CardDef> {
    registered()
        .values()
        .find(|def| def.set == set && def.index == index)
}

/// The registered catalog's version: "0" before one is registered, "test" while a test's override is
/// in force, and while a replay folds under a shipped catalog, that one's (R767000).
pub fn catalog_version() -> &'static str {
    if let Some(shipped) = folding() {
        return &shipped.version;
    }
    if test_override().is_some() {
        return "test";
    }
    match REGISTERED.get() {
        Some(registered) => &registered.version,
        None => "0",
    }
}

/// What holds the transient definitions: the state.
pub type TransientHolder = GameState;

pub fn find_def<'a>(state: Option<&'a GameState>, def_id: &str) -> Option<&'a CardDef> {
    if let Some(def) = state.and_then(|state| state.transient_defs.get(def_id)) {
        return Some(def);
    }
    // R915000: a custom card's definition is the game's own, never the catalog's.
    if crate::custom::is_custom_id(def_id) {
        return state.and_then(|state| state.custom_defs.get(def_id));
    }
    if let Some(shipped) = folding() {
        return shipped.find(def_id);
    }
    if let Some(defs) = test_override() {
        return defs.get(def_id);
    }
    let registered = REGISTERED.get()?;
    let at = *registered.index.get(def_id)?;
    registered.defs.get_index(at).map(|(_, def)| def)
}

// R767000: the shipped catalogs a replay folds under.

/// R767000: a shipped catalog version this build carries: its definitions, in order, where each id
/// sits in them, and the scripts kept for it (`<id>@<version>`, R388). A card with no script kept
/// runs the build's own.
pub struct ShippedCatalog {
    version: String,
    defs: CardDefs,
    index: IndexMap<String, usize, IdHash>,
    scripts: IndexMap<String, CardScripts, IdHash>,
    /// R1266000: the win-rate table this version shipped with, which its folds read.
    win_rates: crate::win_rates::WinRateTable,
    /// R2173000: the BUSTED and complicated lists this version shipped with, which its folds read;
    /// none for a version shipped before any card read them.
    card_lists: Option<CardLists>,
}

/// R2173000: a shipped version's own BUSTED and complicated lists (R1846000), each sorted by id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CardLists {
    pub busted: Vec<&'static str>,
    pub complicated: Vec<&'static str>,
}

impl ShippedCatalog {
    /// R767000: `version`'s catalog, indexed as `register_catalog` indexes the build's, with the empty
    /// win-rate table (R1266000; `with_win_rates` gives it another).
    pub fn new(version: &str, defs: CardDefs, scripts: IndexMap<String, CardScripts>) -> ShippedCatalog {
        let index = defs.keys().enumerate().map(|(at, id)| (id.clone(), at)).collect();
        ShippedCatalog {
            version: version.to_string(),
            defs,
            index,
            scripts: scripts.into_iter().collect(),
            win_rates: crate::win_rates::WinRateTable::default(),
            card_lists: None,
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    /// Its definitions, in catalog order.
    pub fn defs(&self) -> &CardDefs {
        &self.defs
    }

    /// R1266000: this catalog with `table` as the win-rate table its version shipped with.
    pub fn with_win_rates(self, table: crate::win_rates::WinRateTable) -> ShippedCatalog {
        ShippedCatalog {
            win_rates: table,
            ..self
        }
    }

    pub fn win_rates(&self) -> &crate::win_rates::WinRateTable {
        &self.win_rates
    }

    /// R2173000: this catalog with `lists` as the BUSTED and complicated lists its version shipped
    /// with, each sorted here so the engine finds an id by binary search.
    pub fn with_card_lists(self, mut lists: CardLists) -> ShippedCatalog {
        lists.busted.sort_unstable();
        lists.complicated.sort_unstable();
        ShippedCatalog {
            card_lists: Some(lists),
            ..self
        }
    }

    /// R2173000: the BUSTED and complicated lists its version shipped with, if it kept any.
    pub fn card_lists(&self) -> Option<&CardLists> {
        self.card_lists.as_ref()
    }

    fn find(&self, def_id: &str) -> Option<&CardDef> {
        let at = *self.index.get(def_id)?;
        self.defs.get_index(at).map(|(_, def)| def)
    }
}

impl std::fmt::Debug for ShippedCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ShippedCatalog({}: {} defs, {} kept scripts)",
            self.version,
            self.defs.len(),
            self.scripts.len()
        )
    }
}

/// R767000: where the build finds a shipped catalog by version, set once by
/// `jackioh_cards::register_all()`.
static SHIPPED: OnceLock<fn(&str) -> Option<&'static ShippedCatalog>> = OnceLock::new();

/// R767000: set where the shipped catalogs are found, once per process like `register_catalog`.
pub fn register_shipped(find: fn(&str) -> Option<&'static ShippedCatalog>) {
    let _ = SHIPPED.set(find);
}

/// R767000: the testkit's thread-local shipped catalogs, when the calling thread's test set some.
#[cfg(feature = "testkit")]
fn shipped_override() -> Option<&'static [ShippedCatalog]> {
    crate::testkit::scenario::shipped_override()
}

#[cfg(not(feature = "testkit"))]
fn shipped_override() -> Option<&'static [ShippedCatalog]> {
    None
}

/// R767000: the shipped catalog of `version`, when this build carries it: a test's, while one is set
/// (it then stands for every version), else the build's.
pub fn shipped_catalog(version: &str) -> Option<&'static ShippedCatalog> {
    if let Some(catalogs) = shipped_override() {
        return catalogs.iter().find(|catalog| catalog.version == version);
    }
    SHIPPED.get().and_then(|find| find(version))
}

// CLAUDE.md rule 4's scoped exception (R767000): the shipped catalog this thread's fold runs under,
// set by `with_shipped` for that one call and put back after it, so nothing outlives the call.
thread_local! {
    static FOLDING: Cell<Option<&'static ShippedCatalog>> = const { Cell::new(None) };
}

/// R767000: the shipped catalog this thread is folding under, if any.
pub(crate) fn folding() -> Option<&'static ShippedCatalog> {
    FOLDING.with(Cell::get)
}

/// R767000: the script kept for `def_id` in the shipped catalog this thread is folding under.
pub(crate) fn folding_script(def_id: &str) -> Option<&'static CardScripts> {
    folding()?.scripts.get(def_id)
}

/// R767000: `run` under `shipped` (`None`: the build's own catalog), on this thread, for that call
/// alone. The catalog it ran under before is put back after it, a panic included.
pub fn with_shipped<T>(shipped: Option<&'static ShippedCatalog>, run: impl FnOnce() -> T) -> T {
    struct Restore(Option<&'static ShippedCatalog>);
    impl Drop for Restore {
        fn drop(&mut self) {
            FOLDING.with(|cell| cell.set(self.0));
        }
    }
    let _restore = Restore(FOLDING.with(|cell| cell.replace(shipped)));
    run()
}

// CLAUDE.md rule 4's scoped exception (R2201001): whether this thread's step in flight holds its
// game's pools, set by `with_pool_hold` for that one call, so every pool `query` builds reads it.
thread_local! {
    static POOL_HOLD: Cell<bool> = const { Cell::new(false) };
}

/// R2201001: whether the step in flight on this thread holds Legendary and Mythic cards out of its
/// random pools.
pub fn pool_held() -> bool {
    POOL_HOLD.with(Cell::get)
}

/// R2201001: the rarities a held pool leaves out.
pub const HELD_RARITIES: [Rarity; 2] = [Rarity::Legendary, Rarity::Mythic];

/// R2201001: whether a held pool leaves `def` out.
pub fn held_out(def: &CardDef) -> bool {
    pool_held() && HELD_RARITIES.contains(&def.rarity)
}

/// R2201001: `run` with this thread's pools held as `state`'s handicaps say (`state::holds_pools`),
/// the previous hold put back after it (a panic included), so a held step nested in an unheld one (the
/// AI's search, the autopilot) reads its own game's hold. Every entry point that steps a game runs
/// under it (`reduce`, `reduce_unpiloted`, `begin_game`); `create_game` draws from no pool.
pub fn with_pool_hold<T>(state: &GameState, run: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            POOL_HOLD.with(|cell| cell.set(self.0));
        }
    }
    let _restore = Restore(POOL_HOLD.with(|cell| cell.replace(crate::state::holds_pools(state))));
    run()
}

/// Panics when a def is missing: a card instance always has a definition.
pub fn def_of<'a>(state: Option<&'a GameState>, def_id: &str) -> &'a CardDef {
    match find_def(state, def_id) {
        Some(def) => def,
        None => panic!("unknown defId \"{def_id}\": register the catalog first"),
    }
}

// §5.1's one query function, and the cost every filter reads (R65).

/// R65 outside play: "an embiggen card's printed cost is its base price and an X-cost card's is 0".
/// Every pool, filter and comparison that looks at a *definition* rather than at a played instance
/// goes through this, so #7's cost brackets, #30's highest/lowest, #94's odd costs and Recruit all
/// read one number. The in-play calculation is `mana::effective_cost`, which starts from the instance.
pub fn query_cost(def: &CardDef) -> i32 {
    match def.cost {
        CardCost::X => 0,
        CardCost::Fixed(cost) => cost,
        CardCost::Embiggen { base, .. } => base,
    }
}

/// §5.1's `catalog.query({type, cost, costRange, tags, notTags, rarity, set, excludeDefId})`, plus the
/// identity fields a card needs to name a pool by hand. Every field is a narrowing filter, and `{}` is
/// every non-token card of every set (R380). A card writes one as `json_as(json!({ … }))`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CatalogQueryArgs {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<OneOrMany<CardType>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_range: Option<CostRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_tags: Option<Vec<Tag>>,
    /// R1422: has at least one of these tags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub any_tags: Option<Vec<Tag>>,
    /// R1444: only Luck-based cards (`is_luck_based`), Meditative #101 Gachaholic's pool.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub luck_based: Option<bool>,
    /// R1178001: `true` asks for embiggen cards only, the cards with two prices (a printed
    /// `CardCost::Embiggen`, `is_embiggen_card`), `false` forbids them (Meditative+ #22, #23).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embiggen: Option<bool>,
    /// R1080: the printed base attack and health a pool asks for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<AttackHealth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rarity: Option<OneOrMany<Rarity>>,
    /// A set, or several ("Classic or Classic+"). Absent is every set that ships (R380, R1420).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set: Option<OneOrMany<SetName>>,
    /// R387: never these definitions, by catalog id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude_def_id: Option<OneOrMany<String>>,
    /// R382: tokens may come out of this pool beside the cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_tokens: Option<bool>,
    /// A catalog id, or several, for a pool a script names card by card or builds from ids it holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub def_id: Option<OneOrMany<String>>,
    /// §5.1: `true` asks for tokens only, `false` forbids them (the default already does).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<bool>,
    /// R1221: `true` asks for cards with a Tribute play cost only, `false` forbids them. A script
    /// flag, not catalog data, so the pool keeps up with every new Tribute card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tribute: Option<bool>,
    /// R1846000: `true` asks for BUSTED cards only, those whose script has more code lines than
    /// `BUSTED_CODE_LINES` (`is_busted`), `false` forbids them. Read off the compiled card metrics,
    /// so the pool grows with the catalog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub busted: Option<bool>,
    /// R1846000: `true` asks for complicated cards only, more code lines than
    /// `COMPLICATED_CODE_LINES` (`is_complicated`), `false` forbids them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complicated: Option<bool>,
    /// R1846001: `true` widens a pool that names no set from the sets that ship (R380, R1420) to every
    /// set, single-player ones too (`SINGLE_PLAYER_SETS`). A pool that names its set or its cards
    /// keeps to them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub every_set: Option<bool>,
    /// R1859005: `true` asks for the cards the catalog marks `dealsDamage` only (R1748001), `false`
    /// forbids them (Stinkyscape #136 Magic Module's "a random Spell that deals damage").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deals_damage: Option<bool>,
    /// R1878000: prints every one of these keywords on its base face, as `stats` reads the base face
    /// (R1080): Stinkyscape #124's "a random Starship Piece". A keyword a card gains in play (granted,
    /// lent by its Embiggened version or by an aura) is not printed, so no pool reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<KeywordKind>>,
}

/// R1221: whether this definition carries a Tribute play cost — the `tribute` static flag of its
/// base face, read off the registered script without state.
fn has_tribute_cost(def: &CardDef) -> bool {
    crate::scripts::registered_entry(&def.id)
        .map(|entry| {
            entry
                .base
                .static_flags
                .as_ref()
                .is_some_and(|flags| flags.tribute.unwrap_or(0) > 0)
        })
        .unwrap_or(false)
}

impl From<CatalogQuery> for CatalogQueryArgs {
    fn from(query: CatalogQuery) -> CatalogQueryArgs {
        CatalogQueryArgs {
            type_: query.type_,
            cost: query.cost,
            cost_range: query.cost_range,
            tags: query.tags,
            not_tags: query.not_tags,
            any_tags: query.any_tags,
            luck_based: None,
            embiggen: None,
            stats: query.stats,
            rarity: query.rarity,
            set: query.set,
            exclude_def_id: query.exclude_def_id,
            with_tokens: query.with_tokens,
            def_id: None,
            token: None,
            tribute: None,
            busted: None,
            complicated: None,
            every_set: None,
            deals_damage: None,
            keywords: None,
        }
    }
}

fn as_list<T: PartialEq>(value: &Option<OneOrMany<T>>) -> &[T] {
    match value {
        Some(value) => value.as_slice(),
        None => &[],
    }
}

/// §5.1: "Random pools never include Token-tagged cards … unless the card names the pool itself".
/// A query names the token pool by asking for the Token tag or rarity, by setting `token: true`, or
/// by naming its members outright through `defId`. `withTokens` (R382) lets tokens in beside cards.
fn asks_for_tokens(args: &CatalogQueryArgs) -> bool {
    if let Some(token) = args.token {
        return token;
    }
    if args.with_tokens == Some(true) {
        return true;
    }
    if args.tags.as_ref().is_some_and(|tags| tags.contains(&Tag::Token)) {
        return true;
    }
    if as_list(&args.rarity).contains(&Rarity::Token) {
        return true;
    }
    !as_list(&args.def_id).is_empty()
}

/// R382: a pool named by one of `POOL_TOKEN_TAGS` (a Fruit pool) holds that tag's tokens too — the
/// five Grapes are Tokens "that can be generated by any Fruit card" — and so do a Prime pool and an
/// AI pool, whose cards are all tokens (R1421). No other pool reaches them.
fn tag_pool_takes_token(args: &CatalogQueryArgs, def: &CardDef) -> bool {
    let asked = args
        .tags
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .chain(args.any_tags.as_deref().unwrap_or(&[]));
    asked
        .into_iter()
        .any(|tag| POOL_TOKEN_TAGS.contains(tag) && def.tags.contains(tag))
}

/// R1444: the words of a card's text that make it flip a coin, read lower-cased.
pub const COIN_FLIP_WORDS: &str = "flip a coin";

/// MD-G1, R1444: a Luck-based card has a roll Lucky improves — it prints Lucky on either face (§6.1),
/// or its text flips a coin, whose better side is heads (R1440). Read off the definition, so Lucky
/// given to a card (R1438) never puts it in the pool.
pub fn is_luck_based(def: &CardDef) -> bool {
    [&def.base, &def.radiant].into_iter().any(|face| {
        has_keyword(&face.keywords, KeywordKind::Lucky) || face.text.to_lowercase().contains(COIN_FLIP_WORDS)
    })
}

/// R1178001: whether a definition is an embiggen card, one that prints two prices (`CardCost::Embiggen`:
/// its normal price and its Embiggened price, #1175's words). Read off the printed cost, never an
/// instance's, so a cost override, a discount or a Tune never makes a card one or unmakes it.
pub fn is_embiggen_card(def: &CardDef) -> bool {
    matches!(def.cost, CardCost::Embiggen { .. })
}

/// R674, R1857001: the cards only a rule makes, which no pool reaches unless it names them by id
/// (`defId`) and no deck holds: Glitch, which only R673's roll makes, and the Dormant Starship, which
/// only a Starship Piece's death summons (R1852000).
pub const RULE_MADE_DEF_IDS: [&str; 2] = [GLITCH_DEF_ID, DORMANT_STARSHIP_DEF_ID];

/// R674, R1857001: whether a definition is one of `RULE_MADE_DEF_IDS`.
pub fn made_by_a_rule(def: &CardDef) -> bool {
    RULE_MADE_DEF_IDS.contains(&def.id.as_str())
}

fn matches_query(def: &CardDef, args: &CatalogQueryArgs, tokens_allowed: bool) -> bool {
    // R674: Glitch is in no pool, not even one that takes every token; only R673's roll makes one.
    // R1857001: nor is the Dormant Starship, a card and no token, which only a piece's death summons.
    if made_by_a_rule(def) && !as_list(&args.def_id).contains(&def.id) {
        return false;
    }
    // R2201001: a held game's pools leave out Legendary and Mythic cards. A query naming one card
    // (`defId`) is a lookup of that card, not a pool, and is not held.
    if held_out(def) && as_list(&args.def_id).len() != 1 {
        return false;
    }
    if !tokens_allowed && def.is_token() && !tag_pool_takes_token(args, def) {
        return false;
    }
    if let Some(token) = args.token
        && def.is_token() != token
    {
        return false;
    }

    let types = as_list(&args.type_);
    if !types.is_empty() && !types.contains(&def.type_) {
        return false;
    }
    let rarities = as_list(&args.rarity);
    if !rarities.is_empty() && !rarities.contains(&def.rarity) {
        return false;
    }
    let sets = as_list(&args.set);
    if sets.is_empty() {
        // R1420: a pool that names no set draws from the sets that ship, unless it names its cards
        // outright (`defId`); one that names a set draws from that set, shipped or not. R1846001:
        // `everySet` adds the single-player sets.
        if as_list(&args.def_id).is_empty() && !set_is_open(def.set) && !every_set_takes(args, def.set) {
            return false;
        }
    } else if !sets.contains(&def.set) {
        return false;
    }

    let def_ids = as_list(&args.def_id);
    if !def_ids.is_empty() && !def_ids.contains(&def.id) {
        return false;
    }
    // R387, §5.1: a random pool never offers the card that generated it, named by its id.
    if as_list(&args.exclude_def_id).contains(&def.id) {
        return false;
    }

    if let Some(tags) = &args.tags
        && !tags.iter().all(|tag| def.tags.contains(tag))
    {
        return false;
    }
    if let Some(not_tags) = &args.not_tags
        && not_tags.iter().any(|tag| def.tags.contains(tag))
    {
        return false;
    }
    if let Some(any_tags) = &args.any_tags
        && !any_tags.iter().any(|tag| def.tags.contains(tag))
    {
        return false;
    }
    if args.luck_based == Some(true) && !is_luck_based(def) {
        return false;
    }
    if let Some(wanted) = args.embiggen
        && is_embiggen_card(def) != wanted
    {
        return false;
    }
    if let Some(wanted) = args.tribute
        && has_tribute_cost(def) != wanted
    {
        return false;
    }
    if let Some(wanted) = args.busted
        && is_busted(&def.id) != wanted
    {
        return false;
    }
    if let Some(wanted) = args.complicated
        && is_complicated(&def.id) != wanted
    {
        return false;
    }
    if let Some(wanted) = args.deals_damage
        && (def.deals_damage == Some(true)) != wanted
    {
        return false;
    }
    // R1878000: `keywords` reads the printed base face too.
    if let Some(kinds) = &args.keywords
        && !kinds.iter().all(|kind| has_keyword(&def.base.keywords, *kind))
    {
        return false;
    }
    // R1080: `stats` reads the printed base face; a face that prints none never matches.
    if let Some(wanted) = args.stats
        && (def.base.attack != Some(wanted.attack) || def.base.health != Some(wanted.health))
    {
        return false;
    }

    // R65: costs are read out of play, so X counts as 0 and an embiggen card as its base price.
    let cost = query_cost(def);
    if let Some(wanted) = args.cost
        && cost != wanted
    {
        return false;
    }
    if let Some(min) = args.cost_range.and_then(|range| range.min)
        && cost < min
    {
        return false;
    }
    if let Some(max) = args.cost_range.and_then(|range| range.max)
        && cost > max
    {
        return false;
    }
    true
}

/// R77, R102: the catalog ids a definition stands for when it generates — its own id, or, for a fused
/// definition (R179's `t-<n>:<a>+<b>`), every ingredient's, its fused ingredients' included, so a
/// fused card never generates any card it was made from (B4.1 rule 2). A crafted definition with no
/// ingredients stands for itself, which no pool can reach anyway.
pub fn self_def_ids(state: Option<&GameState>, def_id: &str) -> Vec<String> {
    match fused_id_parts(state, def_id) {
        None => vec![def_id.to_string()],
        Some(ingredients) => ingredients
            .iter()
            .flat_map(|id| self_def_ids(state, id))
            .collect(),
    }
}

/// R179: the ingredient ids a fused id (`t-<n>:<a>+<b>`, a fused ingredient in parentheses) names,
/// split at the top-level `+` signs; `None` for any other id. `fuse::fused_ingredients` is its reader
/// with R77's two-ingredient minimum. R469's Radiant mark (`<a>*`) is dropped here, since what a
/// card stands for is the definition, whichever face went in.
pub fn fused_id_parts(state: Option<&GameState>, def_id: &str) -> Option<Vec<String>> {
    fused_id_specs(state, def_id).map(|specs| specs.into_iter().map(|spec| spec.def_id).collect())
}

/// R468: what follows `t-<n>:` in a digest id, which names its ingredients by a hash of them alone.
pub const FUSED_DIGEST_MARK: &str = "#";

/// R469: the mark an ingredient that went in on its Radiant face carries in a readable fused id.
pub const RADIANT_INGREDIENT_MARK: &str = "*";

/// `/^t-\d+:/`: the length of a fused id's head, or `None` for an id no Fuse minted.
pub fn fused_head_len(def_id: &str) -> Option<usize> {
    let rest = def_id.strip_prefix("t-")?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 || rest.as_bytes().get(digits) != Some(&b':') {
        return None;
    }
    Some("t-".len() + digits + 1)
}

/// R468: whether a fused id is a digest of its ingredients rather than their names.
pub fn is_digest_id(def_id: &str) -> bool {
    match fused_head_len(def_id) {
        Some(head) => def_id[head..].starts_with(FUSED_DIGEST_MARK),
        None => false,
    }
}

pub(crate) fn copy_spec(entry: &FusedIngredient) -> FusedIngredient {
    FusedIngredient {
        def_id: entry.def_id.clone(),
        radiant: if entry.radiant == Some(true) {
            Some(true)
        } else {
            None
        },
    }
}

/// R179, R468, R469: the ingredients a fused id names, in order, each with its Radiant mark; `None`
/// for an id no Fuse minted, or a digest the state holds no definition of. A readable id is split at
/// its top-level `+` signs (a fused ingredient in parentheses, a Radiant one followed by `*`); a digest
/// id's list is its fused definition's `ingredients`.
pub fn fused_id_specs(state: Option<&GameState>, def_id: &str) -> Option<Vec<FusedIngredient>> {
    let head = fused_head_len(def_id)?;
    if is_digest_id(def_id) {
        let known = find_def(state, def_id)?.ingredients.as_ref()?;
        return Some(known.iter().map(copy_spec).collect());
    }
    let bytes = def_id.as_bytes();
    let mut specs: Vec<FusedIngredient> = Vec::new();
    let mut depth: i32 = 0;
    let mut start = head;
    for at in head..=bytes.len() {
        let ch = bytes.get(at).copied();
        if ch == Some(b'(') {
            depth += 1;
        } else if ch == Some(b')') {
            depth -= 1;
        } else if (ch == Some(b'+') && depth == 0) || at == bytes.len() {
            let mut part = &def_id[start..at];
            let radiant = part.ends_with(RADIANT_INGREDIENT_MARK);
            if radiant {
                part = &part[..part.len() - RADIANT_INGREDIENT_MARK.len()];
            }
            let name = if part.len() >= 2 && part.starts_with('(') && part.ends_with(')') {
                &part[1..part.len() - 1]
            } else {
                part
            };
            specs.push(FusedIngredient {
                def_id: name.to_string(),
                radiant: if radiant { Some(true) } else { None },
            });
            start = at + 1;
        }
    }
    if specs.iter().all(|spec| !spec.def_id.is_empty()) {
        Some(specs)
    } else {
        None
    }
}

/// §5.1, R387: "a random pool never includes the generating card's own definition". The running
/// card's ids (`self_def_ids`) are ADDED to whatever the caller already excludes, never put in its
/// place: a card whose text names its own exclusion keeps it when the card running that text is a
/// fused one (R77, R102).
pub fn excluding_def_id(
    state: Option<&GameState>,
    args: &CatalogQueryArgs,
    def_id: Option<&str>,
) -> CatalogQueryArgs {
    let Some(def_id) = def_id else {
        return args.clone();
    };
    let already: Vec<String> = as_list(&args.exclude_def_id).to_vec();
    let added: Vec<String> = self_def_ids(state, def_id)
        .into_iter()
        .filter(|id| !already.contains(id))
        .collect();
    if added.is_empty() {
        return args.clone();
    }
    let mut excluded = already;
    excluded.extend(added);
    CatalogQueryArgs {
        exclude_def_id: Some(OneOrMany::Many(excluded)),
        ..args.clone()
    }
}

/// §5's index as a number (SURFACE §4.4.4), so "2" sorts before "10" and a token index ("T-rush")
/// sorts last.
pub(crate) fn index_rank(index: &str) -> f64 {
    match index.parse::<f64>() {
        Ok(parsed) if parsed.is_finite() => parsed,
        _ => f64::INFINITY,
    }
}

/// B2.2: a set's place in the catalog order (`CATALOG_SETS`, which keeps a set's place when it
/// ships, R1420); a set the order does not name sorts last.
fn set_rank(set: SetName) -> usize {
    CATALOG_SETS
        .iter()
        .position(|listed| *listed == set)
        .unwrap_or(CATALOG_SETS.len())
}

#[cfg(feature = "testkit")]
fn previewed(set: SetName) -> bool {
    crate::testkit::preview::previewed(set)
}

#[cfg(not(feature = "testkit"))]
fn previewed(_set: SetName) -> bool {
    false
}

/// R1420: whether a pool that names no set may draw a card of `set`, and whether a deck may hold
/// one: a set that ships, or, under the testkit, a set this thread previews.
pub fn set_is_open(set: SetName) -> bool {
    set_ships(set) || previewed(set)
}

/// R1420, §2.6, §9.4 L3: whether a definition may be in a deck: a card, not a Token, of a set that is
/// open (`set_is_open`), and not one only a rule makes (`made_by_a_rule`: R1857001's Dormant
/// Starship). The validator, the deck builder and every random deck read this one test.
pub fn deckable(def: &CardDef) -> bool {
    !def.is_token() && !made_by_a_rule(def) && set_is_open(def.set)
}

/// R2204001, R2511001: the key a Discover from the catalog offers once. A definition that names the
/// Core card it copied (`CardDef.snapshot_of`) shares that card's key; every other card is its own.
/// The build's catalog names none, so an offer may show a Core card and its Legacy card both; the
/// catalogs shipped from v0.5.5 to v0.5.7 name one for each of Training Grounds #41–#60, so a replay
/// folded under one (R767000) offers what its game offered. Only the two Discovers read it
/// (`effects::choose`'s offer and `subsystems::scorer::top_three`); a deck holds both cards (R2511001).
pub fn pair_key(def: &CardDef) -> &str {
    def.snapshot_of.as_deref().unwrap_or(&def.id)
}

/// §5.1's single source of random pools and Discover: every definition the filters allow, in a strict
/// total order (set by `CATALOG_SETS`, §5 index as a number, the index, then the catalog id), so a
/// seeded `rng.pick`/`rng.shuffle` over it replays identically (R58, R60, §9.3). Only the registered
/// catalog is searched, so no pool can generate a fused or crafted definition.
pub fn query(args: &CatalogQueryArgs) -> Vec<&'static CardDef> {
    let tokens_allowed = asks_for_tokens(args);
    let mut pool: Vec<&'static CardDef> = registered()
        .values()
        .filter(|def| matches_query(def, args, tokens_allowed))
        .collect();
    pool.sort_by(|a, b| {
        let set_a = set_rank(a.set);
        let set_b = set_rank(b.set);
        if set_a != set_b {
            return set_a.cmp(&set_b);
        }
        // Compared, not subtracted: two token indexes are both +Infinity, and `Infinity - Infinity`
        // is NaN, which would skip both tie-breaks and break the total order (R58, R60, §9.3).
        let rank_a = index_rank(&a.index);
        let rank_b = index_rank(&b.index);
        if rank_a != rank_b {
            return if rank_a < rank_b {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        if a.index != b.index {
            return a.index.cmp(&b.index);
        }
        a.id.cmp(&b.id)
    });
    pool
}

// Grapes (C+ #65, #66; R382)

/// R960: one weighted roll over `table` — one draw of the match rng over the percents' sum,
/// walked in the table's order. `lucky` extra rolls (§6.1 Lucky X) keep the best, and the best is
/// the later entry, since the table runs from worst to best. Returns the rolled entry's def id.
pub fn roll_weighted(rng: &mut Rng, table: &[RollOdds], lucky: i32) -> String {
    // R2201001: a held game's table leaves out the entries a held pool would, and rolls the rest by
    // their own percents. A table left empty rolls nothing. An unheld game rolls the table as given.
    let kept: Vec<RollOdds>;
    let table = if pool_held() {
        kept = table
            .iter()
            .filter(|entry| !find_def(None, entry.def_id).is_some_and(held_out))
            .copied()
            .collect();
        if kept.is_empty() {
            return String::new();
        }
        &kept[..]
    } else {
        table
    };
    let roll = |rng: &mut Rng| -> usize {
        let total: i32 = table.iter().map(|entry| entry.percent).sum();
        let mut at = rng.int(total);
        for (i, entry) in table.iter().enumerate() {
            if at < entry.percent {
                return i;
            }
            at -= entry.percent;
        }
        table.len().saturating_sub(1)
    };
    let extra = lucky.max(0);
    let index = if extra == 0 {
        roll(rng)
    } else {
        rng.lucky(extra, roll, |a, b| a.max(b))
    };
    let entry = table.get(index).or_else(|| table.last());
    entry.map(|entry| entry.def_id.to_string()).unwrap_or_default()
}

/// R382, BUILD §2: one Grape, `roll_weighted` on `GRAPE_ODDS`, which runs from worst to best (Rotten <
/// Normal < Large < Golden < Mythic). Returns the Grape's def id.
pub fn roll_grape(rng: &mut Rng, lucky: i32) -> String {
    roll_weighted(rng, GRAPE_ODDS, lucky)
}

/// The def ids `GRAPE_ODDS` rolls: a pool pick naming one of these is re-rolled (R382).
fn is_grape_def_id(def_id: &str) -> bool {
    GRAPE_ODDS.iter().any(|grape| grape.def_id == def_id)
}

/// R673: what the Glitch roll reads: the match's count of "… in the System" plays (`system_plays`).
pub type GlitchOdds = GameState;

/// R382: draw the generated card from a pool with `rng.pick`, except that a Grape pick is rolled again
/// on `GRAPE_ODDS` (one extra draw, only then), so the Grape rarity pool persists across every kind of
/// Grape generation. A caller generating into a hand or a deck passes the state as `glitch`, so the pick
/// may become Glitch (R673).
pub fn pick_generated<'a>(
    rng: &mut Rng,
    pool: &[&'a CardDef],
    glitch: Option<&GlitchOdds>,
) -> Option<&'a CardDef> {
    let picked: Option<&'a CardDef> = rng.pick(pool).copied();
    let def = match picked {
        Some(grape) if is_grape_def_id(&grape.id) => {
            let rolled = roll_grape(rng, 0);
            Some(find_def(None, &rolled).unwrap_or(grape))
        }
        other => other,
    };
    match (def, glitch) {
        (Some(def), Some(glitch)) => Some(glitch_or_not(rng, def, glitch)),
        (def, _) => def,
    }
}

/// R673: after a card is picked for a hand or a deck, n/10000 that it is Glitch instead, n the match's
/// System plays. One rng draw, and only when n is above 0, so a match with no System card draws as
/// before; a catalog without Glitch keeps the pick.
pub fn glitch_or_not<'a>(rng: &mut Rng, def: &'a CardDef, glitch: &GlitchOdds) -> &'a CardDef {
    let plays = glitch.system_plays.unwrap_or(0);
    if plays <= 0 {
        return def;
    }
    let roll = rng.int(GLITCH_ODDS_DENOMINATOR);
    if roll >= plays * GLITCH_ODDS_PER_SYSTEM_PLAY {
        return def;
    }
    find_def(None, GLITCH_DEF_ID).unwrap_or(def)
}

// R1846000: the card metrics, BUSTED and complicated cards (Stinkyscape #109, #111).

/// R1846000: the card metrics `crates/cards/build.rs` compiles in and `jackioh_cards::register_all`
/// hands over: each script's code lines (those above its tests that hold code once comments are out)
/// and the two lists they make, every catalog entry's, a token's included, each sorted by id.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CardMetrics {
    /// Each catalog entry's script's code lines, by catalog id.
    pub code_lines: &'static [(&'static str, usize)],
    /// The BUSTED cards: more code lines than `BUSTED_CODE_LINES`.
    pub busted: &'static [&'static str],
    /// The complicated cards: more code lines than `COMPLICATED_CODE_LINES`.
    pub complicated: &'static [&'static str],
}

/// R1846000: the compiled card metrics, set once by `register_card_metrics`, beside the registry.
static METRICS: OnceLock<CardMetrics> = OnceLock::new();

/// R1846000: hands the compiled card metrics to the engine (`jackioh_cards::register_all`). Set once
/// per process, as `register_catalog` is: a second call is ignored, and a test that needs other
/// metrics sets the testkit's override (`testkit::register_code_lines`) instead.
pub fn register_card_metrics(metrics: CardMetrics) {
    let _ = METRICS.set(metrics);
}

/// R1846000: the card metrics: the testkit's override first, then the compiled ones; none before
/// anything is registered. R2173000: while a replay folds under a shipped catalog that kept its own
/// lists, its BUSTED and complicated lists are that version's, beside the code lines above.
pub fn card_metrics() -> CardMetrics {
    let build = build_card_metrics();
    match folding().and_then(|shipped| shipped.card_lists.as_ref()) {
        Some(lists) => CardMetrics {
            code_lines: build.code_lines,
            busted: &lists.busted,
            complicated: &lists.complicated,
        },
        None => build,
    }
}

/// R1846000: the testkit's override of the compiled card metrics, else the compiled ones.
fn build_card_metrics() -> CardMetrics {
    #[cfg(feature = "testkit")]
    if let Some(metrics) = crate::testkit::scenario::card_metrics_override() {
        return metrics;
    }
    METRICS.get().copied().unwrap_or_default()
}

/// R1846000: a card's code lines, or `None` for a definition with no script of its own in the
/// catalog: a fused or crafted card, a fixture the metrics do not list, or a custom card, whatever a
/// table lists (R1846002: its Luau script is not the game's code).
pub fn code_lines_of(def_id: &str) -> Option<usize> {
    if crate::custom::is_custom_id(def_id) {
        return None;
    }
    let lines = card_metrics().code_lines;
    lines
        .binary_search_by(|(id, _)| (*id).cmp(def_id))
        .ok()
        .map(|at| lines[at].1)
}

/// R1846000: whether a definition is BUSTED, its script over `BUSTED_CODE_LINES` code lines; a
/// custom card never is (R1846002).
pub fn is_busted(def_id: &str) -> bool {
    !crate::custom::is_custom_id(def_id) && card_metrics().busted.binary_search(&def_id).is_ok()
}

/// R1846000: whether a definition is complicated, its script over `COMPLICATED_CODE_LINES` code
/// lines; a custom card never is (R1846002).
pub fn is_complicated(def_id: &str) -> bool {
    !crate::custom::is_custom_id(def_id) && card_metrics().complicated.binary_search(&def_id).is_ok()
}

/// R1846001: whether a query's `everySet` takes a card of `set` into a pool that names no set: a
/// single-player set (`SINGLE_PLAYER_SETS`). The sets that ship are in every such pool already.
fn every_set_takes(args: &CatalogQueryArgs, set: SetName) -> bool {
    args.every_set == Some(true) && crate::wire::SINGLE_PLAYER_SETS.contains(&set)
}
