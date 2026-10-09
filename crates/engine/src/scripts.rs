//! Script registry: `jackioh-cards` registers one entry per catalog id (M4-T2), and engine tests
//! register fixtures (BUILD §0). Like the catalog this is static data, not game state.
//!
//! Port of `packages/engine/src/scripts.ts` (SURFACE §3, §6.6, §8):
//!
//! - The registry is a `OnceLock`, set once by `register_scripts` (`jackioh_cards::register_all()`)
//!   and read-only after. Under the `testkit` feature the testkit's thread-local override
//!   (`testkit::scenario::register_scripts`) is consulted first and, while set, stands for the whole
//!   registry, as TS's `registerScripts` replaced it wholesale.
//! - A fused or crafted id (R77, R179) that the registry does not hold is built from the definition in
//!   `state.transient_defs` (`subsystems::fuse::compose_fused_scripts`), so `scripts_for` and
//!   `script_of` take the state. TS's `syncFusedScripts` registered each fused id's scripts in the
//!   process's registry on entry to `reduce`; here the state keeps them instead
//!   (`GameState::fused_scripts`, `sync_fused_scripts`), composed once and shared by every state cloned
//!   from it, and a lookup the state holds none for composes them then and there.
//! - SURFACE §6.6 names `script_of(state, def_id) -> CardScripts`, and TS's `scriptOf(instance)` is the
//!   running face's `Script`: `script_of` answers both, by its second argument (`ScriptKey`) — a
//!   definition id gives the card's two scripts (TS `scriptsFor`), an instance its running face's
//!   script with the Vanilla guard below (TS `scriptOf`).

use std::borrow::Cow;
use std::sync::{Arc, OnceLock};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::IdHash;
use crate::config::FUSE_MIN_INGREDIENTS;
use crate::script::{CardScripts, Script, StaticFlags, empty_script};
use crate::state::{CardInstance, GameState};
use crate::wire::catalog_types::FusedIngredient;

/// The production registry (SURFACE §3: one of the two statics, set once), hashed by `IdHasher`: an
/// entry is looked up for nearly every card a rule or a view reads.
static REGISTERED: OnceLock<IndexMap<String, CardScripts, IdHash>> = OnceLock::new();

/// The testkit's thread-local registry, when a test has set one (SURFACE §8; `'static` as the catalog's
/// override is, e.g. a leaked box per registration).
#[cfg(feature = "testkit")]
fn override_map() -> Option<&'static IndexMap<String, CardScripts>> {
    crate::testkit::scenario::scripts_override()
}

#[cfg(not(feature = "testkit"))]
fn override_map() -> Option<&'static IndexMap<String, CardScripts>> {
    None
}

/// Set the registry. It is set once per process (SURFACE §3); a later call changes nothing, which is
/// what `jackioh_cards::register_all()`'s idempotence needs. Tests swap scripts through the testkit's
/// `register_scripts` instead.
pub fn register_scripts(scripts: IndexMap<String, CardScripts>) {
    let _ = REGISTERED.set(scripts.into_iter().collect());
}

/// One registered entry, borrowed: the testkit's override while one is set, else the production
/// registry. A fused id is never in it. Lookups read through this rather than `registered_scripts`,
/// which copies every entry.
pub fn registered_entry(def_id: &str) -> Option<&'static CardScripts> {
    match override_map() {
        Some(over) => over.get(def_id),
        None => REGISTERED.get().and_then(|map| map.get(def_id)),
    }
}

/// Every registered entry, by catalog id: the testkit's override while one is set, else the
/// production registry (empty before `register_scripts`). A copy, so a test may extend it and
/// register the result (TS `registerScripts({ ...registeredScripts(), … })`).
pub fn registered_scripts() -> IndexMap<String, CardScripts> {
    match override_map() {
        Some(over) => over.clone(),
        None => REGISTERED
            .get()
            .map(|map| {
                map.iter()
                    .map(|(id, entry)| (id.clone(), entry.clone()))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// R179, R468: how many ingredients a fused id names (TS `fuse.fusedIngredientSpecs`' length): a
/// readable id's top-level `+`-separated parts, or a digest id's (`t-<n>:#<hex>`) list off its
/// definition. `None` for any other id, or a malformed one.
fn fused_ingredient_count(state: &GameState, def_id: &str) -> Option<usize> {
    let head = crate::catalog::fused_head_len(def_id)?;
    if def_id[head..].starts_with('#') {
        return state
            .transient_defs
            .get(def_id)
            .and_then(|def| def.ingredients.as_ref())
            .map(Vec::len);
    }
    let bytes = def_id.as_bytes();
    let mut count = 0;
    let mut depth: i32 = 0;
    let mut start = head;
    for at in head..=def_id.len() {
        let byte = bytes.get(at).copied();
        if byte == Some(b'(') {
            depth += 1;
        } else if byte == Some(b')') {
            depth -= 1;
        } else if (byte == Some(b'+') && depth == 0) || at == def_id.len() {
            let mut part = &def_id[start..at];
            if let Some(stripped) = part.strip_suffix('*') {
                part = stripped;
            }
            let name = if part.len() >= 2 && part.starts_with('(') && part.ends_with(')') {
                &part[1..part.len() - 1]
            } else {
                part
            };
            if name.is_empty() {
                return None;
            }
            count += 1;
            start = at + 1;
        }
    }
    Some(count)
}

/// Which registry a composition read its ingredients' scripts from: the testkit's override (each one a
/// leaked box of its own) or the production registry, by address; 0 before either is set.
fn registry_identity() -> usize {
    match override_map() {
        Some(over) => std::ptr::from_ref(over) as usize,
        None => REGISTERED
            .get()
            .map_or(0, |registered| std::ptr::from_ref(registered) as usize),
    }
}

/// One fused definition's two scripts as `fuse::compose_fused_scripts` composed them, and the
/// ingredient list they were composed from.
#[derive(Clone)]
struct FusedEntry {
    ingredients: Option<Vec<FusedIngredient>>,
    base: Arc<Script>,
    radiant: Arc<Script>,
}

/// R179: the scripts of a state's fused definitions, composed once and shared, as TS's registry held
/// them once `syncFusedScripts` had registered them. A fused card's script is read for every event,
/// aura, cost and declaration that looks at it, and composing it again for each read cost a card fused
/// onto again and again (C+ #74) seconds per AI decision late in a game.
///
/// Derived data, never part of what a state is: serde skips it (no wire, view or hash sees it), every
/// state compares equal on it, and a state that came through JSON has none until `reduce` composes it
/// (`sync_fused_scripts`, run on entry to `reduce` and by a Fuse as it mints a definition). An entry
/// answers only while its definition is in the state with the ingredient list it was composed from and
/// the registry it read is still the one in force: R179, "the fused scripts depend on the ingredients'
/// ids and on nothing else". Otherwise the lookup composes, exactly as it did with no entry.
#[derive(Clone, Default)]
pub struct FusedScripts {
    registry: usize,
    entries: Arc<IndexMap<String, FusedEntry, IdHash>>,
}

impl FusedScripts {
    fn entry(&self, def_id: &str, ingredients: &Option<Vec<FusedIngredient>>) -> Option<&FusedEntry> {
        let entry = self.entries.get(def_id)?;
        (self.registry == registry_identity() && entry.ingredients == *ingredients).then_some(entry)
    }
}

impl PartialEq for FusedScripts {
    fn eq(&self, _other: &FusedScripts) -> bool {
        true
    }
}

impl std::fmt::Debug for FusedScripts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FusedScripts({} composed)", self.entries.len())
    }
}

/// R179 (TS `syncFusedScripts`, `ensureFused`): compose the scripts of every fused definition in the
/// state that `state.fused_scripts` holds none for, so a lookup reads them instead of composing them
/// again. `reduce` runs it on entry, for a state that came through JSON, and a Fuse as it mints or
/// rebuilds a definition. It changes no answer: an entry answers what composing would.
pub fn sync_fused_scripts(state: &mut GameState) {
    let registry = registry_identity();
    let fresh = state.fused_scripts.registry != registry;
    let mut composed: Vec<(String, FusedEntry)> = Vec::new();
    for (def_id, def) in &state.transient_defs {
        if !fresh && state.fused_scripts.entry(def_id, &def.ingredients).is_some() {
            continue;
        }
        if fused_ingredient_count(state, def_id).is_none_or(|count| count < FUSE_MIN_INGREDIENTS) {
            continue;
        }
        let scripts = crate::subsystems::fuse::compose_fused_scripts(state, def);
        composed.push((
            def_id.clone(),
            FusedEntry {
                ingredients: def.ingredients.clone(),
                base: Arc::new(scripts.base),
                radiant: Arc::new(scripts.radiant),
            },
        ));
    }
    if !fresh && composed.is_empty() {
        return;
    }
    let mut entries = if fresh {
        IndexMap::default()
    } else {
        (*state.fused_scripts.entries).clone()
    };
    entries.extend(composed);
    state.fused_scripts = FusedScripts {
        registry,
        entries: Arc::new(entries),
    };
}

/// R77, R179: a fused definition's two scripts — the state's (`sync_fused_scripts`), or built now from
/// its ingredients' — for an id with R77's two ingredients or more whose definition is in the state.
/// `None` otherwise (TS's registry then answered with no entry: the empty scripts).
fn fused_scripts(state: &GameState, def_id: &str) -> Option<CardScripts> {
    if fused_ingredient_count(state, def_id)? < FUSE_MIN_INGREDIENTS {
        return None;
    }
    let def = state.transient_defs.get(def_id)?;
    if let Some(entry) = state.fused_scripts.entry(def_id, &def.ingredients) {
        return Some(CardScripts {
            base: Script::clone(&entry.base),
            radiant: Script::clone(&entry.radiant),
        });
    }
    Some(crate::subsystems::fuse::compose_fused_scripts(state, def))
}

/// One face of `fused_scripts(state, def_id)`: the state's, shared, or composed alone
/// (`fuse::compose_fused_face`).
fn fused_face(state: &GameState, def_id: &str, radiant: bool) -> Option<ScriptRef> {
    if fused_ingredient_count(state, def_id)? < FUSE_MIN_INGREDIENTS {
        return None;
    }
    let def = state.transient_defs.get(def_id)?;
    if let Some(entry) = state.fused_scripts.entry(def_id, &def.ingredients) {
        let face = if radiant { &entry.radiant } else { &entry.base };
        return Some(ScriptRef::Composed(face.clone()));
    }
    Some(ScriptRef::Composed(Arc::new(
        crate::subsystems::fuse::compose_fused_face(state, def, radiant),
    )))
}

/// A definition's two scripts: the registry's entry, a fused definition's built from the state, or
/// none (`{ base: EMPTY_SCRIPT, radiant: EMPTY_SCRIPT }`). A copy; `scripts_ref` borrows.
pub fn scripts_for(state: &GameState, def_id: &str) -> CardScripts {
    scripts_ref(state, def_id).into_owned()
}

/// A script as a lookup answers it: borrowed from the registry (static data, SURFACE §3), or a fused
/// card's (R77), shared with the state's composed scripts or composed for this lookup. Reads go
/// through `Deref`; `into_owned()` is the copy `script_of` used to make of every entry.
#[derive(Clone)]
pub enum ScriptRef {
    /// The registry's entry, or the shared empty script.
    Static(&'static Script),
    /// A fused card's: `GameState::fused_scripts`'s, or composed for this lookup.
    Composed(Arc<Script>),
}

impl std::ops::Deref for ScriptRef {
    type Target = Script;

    fn deref(&self) -> &Script {
        match self {
            ScriptRef::Static(script) => script,
            ScriptRef::Composed(script) => script,
        }
    }
}

impl ScriptRef {
    /// The script, owned: a copy of a borrowed or a shared one.
    pub fn into_owned(self) -> Script {
        match self {
            ScriptRef::Static(script) => script.clone(),
            ScriptRef::Composed(script) => Arc::unwrap_or_clone(script),
        }
    }
}

/// `scripts_for` without the copy: the registry's entry borrowed, a fused definition's composed now.
pub fn scripts_ref(state: &GameState, def_id: &str) -> Cow<'static, CardScripts> {
    if let Some(entry) = registered_entry(def_id) {
        return Cow::Borrowed(entry);
    }
    if let Some(scripts) = fused_scripts(state, def_id) {
        return Cow::Owned(scripts);
    }
    Cow::Owned(crafted_scripts(state, def_id).unwrap_or_default())
}

/// One face of a definition's scripts (`radiant` or base), borrowed as `scripts_ref` borrows.
pub fn face_ref(state: &GameState, def_id: &str, radiant: bool) -> ScriptRef {
    if let Some(entry) = registered_entry(def_id) {
        return ScriptRef::Static(if radiant { &entry.radiant } else { &entry.base });
    }
    if let Some(face) = fused_face(state, def_id, radiant) {
        return face;
    }
    crafted_face(state, def_id, radiant).unwrap_or_else(|| ScriptRef::Static(shared_empty_script()))
}

/// R882 (ME-CRAFT): a crafted definition's two scripts — rebuilt from the def's recipe in any
/// process, as a fused definition's rebuild from its ingredients. `None` for any other id. No
/// cache is needed: composing is one walk over at most eight effects.
fn crafted_scripts(state: &GameState, def_id: &str) -> Option<CardScripts> {
    if !def_id.starts_with(crate::config::CRAFT_ID_PREFIX) {
        return None;
    }
    let def = state.transient_defs.get(def_id)?;
    let recipe = def.craft.as_ref()?;
    Some(crate::subsystems::craft::compile_crafted_scripts(recipe))
}

/// One face of `crafted_scripts(state, def_id)`: composed alone.
fn crafted_face(state: &GameState, def_id: &str, radiant: bool) -> Option<ScriptRef> {
    let scripts = crafted_scripts(state, def_id)?;
    Some(ScriptRef::Composed(Arc::new(if radiant {
        scripts.radiant
    } else {
        scripts.base
    })))
}

/// TS `EMPTY_SCRIPT`, one shared instance: what a Vanilla card runs (immutable, like the registry).
fn shared_empty_script() -> &'static Script {
    static EMPTY: OnceLock<Script> = OnceLock::new();
    EMPTY.get_or_init(empty_script)
}

/// The face that is running: radiant text once the instance is Radiant (§5.2).
///
/// A Vanilla instance runs no script at all: §6.3's Vanilla "clears printed keywords and scripts"
/// and R115 says so of every hook, so its triggers, its start- and end-of-turn hooks, its Death and
/// its static flags (Deft Duelist's two exertions, Spikey Pillow's "cannot be in Defense Position")
/// are gone with its text. This is the one place a card's script is read off an instance, so the
/// guard lives here rather than in each reader; a continuation parked before the Vanilla landed is
/// re-entered by its stored def id (`prompts::run_resume`), never through here, so it still finishes.
///
/// Borrowed from the registry: nothing is copied for a registered card (a fused card's script is shared
/// with the state's, or composed for the lookup).
fn instance_script(state: &GameState, instance: &CardInstance) -> ScriptRef {
    if instance.vanilla {
        return ScriptRef::Static(shared_empty_script());
    }
    face_ref(state, &instance.def_id, instance.radiant)
}

/// What `script_of` may be asked about: a definition id (the card's two scripts, SURFACE §6.6) or an
/// instance (its running face's script, TS `scriptOf`, borrowed: `ScriptRef`).
pub trait ScriptKey {
    type Scripts;
    fn scripts_in(self, state: &GameState) -> Self::Scripts;
}

impl ScriptKey for &str {
    type Scripts = CardScripts;
    fn scripts_in(self, state: &GameState) -> CardScripts {
        scripts_for(state, self)
    }
}

impl ScriptKey for &String {
    type Scripts = CardScripts;
    fn scripts_in(self, state: &GameState) -> CardScripts {
        scripts_for(state, self)
    }
}

impl ScriptKey for &CardInstance {
    type Scripts = ScriptRef;
    fn scripts_in(self, state: &GameState) -> ScriptRef {
        instance_script(state, self)
    }
}

impl ScriptKey for &mut CardInstance {
    type Scripts = ScriptRef;
    fn scripts_in(self, state: &GameState) -> ScriptRef {
        instance_script(state, self)
    }
}

/// `script_of(state, def_id)`: the registry's `CardScripts` for a catalog id, or for a fused/crafted
/// id one composed from the state (SURFACE §6.6). `script_of(state, instance)`: the script of the
/// face the instance runs, none for a Vanilla one (TS `scriptOf`).
pub fn script_of<K: ScriptKey>(state: &GameState, key: K) -> K::Scripts {
    key.scripts_in(state)
}

/// The running face's static flags (`script.staticFlags ?? {}`); none for a Vanilla card.
pub fn flags_of(state: &GameState, instance: &CardInstance) -> StaticFlags {
    if instance.vanilla {
        return StaticFlags::default();
    }
    if let Some(entry) = registered_entry(&instance.def_id) {
        let face = if instance.radiant {
            &entry.radiant
        } else {
            &entry.base
        };
        return face.flags();
    }
    instance_script(state, instance).flags()
}

/// R77, R102: where a card a Fuse kept on the field remembers the price each ingredient was played
/// for (§6.3 Embiggen, R65), in the order of the fused definition's ingredients, when they are not
/// all the kept card's own. The fused card carries every ingredient's text, and a text that reads its
/// own price (#46's "paid 4: −5/−5", #84's "paid 4: 5", #59's "paid 4: it costs 0") reads the one its
/// card was played for, not the kept instance's. An ingredient that was itself a fused card with such
/// a record keeps it as its `parts`. Absent when every ingredient was played at the kept card's price,
/// which is then every text's: R77 lets the Fuse add this one entry to the kept memory, and only when
/// the prices differ. Memory, so R78 resets it with everything else a card leaves the field without:
/// a fused card replayed from a hand paid the fused cost, and no ingredient's embiggen price.
pub const INGREDIENTS_KEY: &str = "__ingredients";

/// One ingredient of a fused card, as `INGREDIENTS_KEY` records it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IngredientRecord {
    pub def_id: String,
    pub embiggened: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<IngredientRecord>>,
}

fn records_from(raw: Option<&Value>) -> Option<Vec<IngredientRecord>> {
    let list = raw?.as_array()?;
    Some(
        list.iter()
            .filter_map(|entry| {
                let record = entry.as_object()?;
                let def_id = record.get("defId")?.as_str()?;
                let parts = records_from(record.get("parts"));
                Some(IngredientRecord {
                    def_id: def_id.to_string(),
                    embiggened: record.get("embiggened") == Some(&Value::Bool(true)),
                    parts,
                })
            })
            .collect(),
    )
}

/// The ingredient prices a kept fused card records, or `None` when it records none (it came through JSON).
pub fn ingredients_of(instance: &CardInstance) -> Option<Vec<IngredientRecord>> {
    records_from(instance.memory.get(INGREDIENTS_KEY))
}

/// R102: the price the text at `path` was played for — `path` is the text's place in the fusion, one
/// index per level (`work::PART_KEY`'s path), and a card that records no prices has one, its own.
pub fn ingredient_paid(instance: &CardInstance, path: &[usize]) -> bool {
    let mut records = ingredients_of(instance);
    let mut paid = instance.embiggened == Some(true);
    for &index in path {
        let Some(record) = records.as_ref().and_then(|list| list.get(index)).cloned() else {
            return paid;
        };
        paid = record.embiggened;
        records = record.parts;
    }
    paid
}

/// R102: the card as the text of ingredient `index` reads it — at that ingredient's price, carrying
/// that ingredient's own record — for a hook that reads "this" rather than a context (§10.4's aura).
/// The same instance when the card records no prices.
pub fn as_ingredient(instance: &CardInstance, index: usize) -> CardInstance {
    let Some(record) = ingredients_of(instance).and_then(|list| list.get(index).cloned()) else {
        return instance.clone();
    };
    let mut memory = instance.memory.clone();
    match record.parts {
        None => {
            memory.shift_remove(INGREDIENTS_KEY);
        }
        Some(parts) => {
            let parts = serde_json::to_value(parts).unwrap_or(Value::Null);
            memory.insert(INGREDIENTS_KEY.to_string(), parts);
        }
    }
    CardInstance {
        embiggened: Some(record.embiggened),
        memory,
        ..instance.clone()
    }
}

/// What a Fuse records for the card it keeps (`INGREDIENTS_KEY`), or `None` when it records nothing.
pub fn ingredient_record(kept: &CardInstance, ingredients: &[CardInstance]) -> Option<Vec<IngredientRecord>> {
    let own = kept.embiggened == Some(true);
    let records: Vec<IngredientRecord> = ingredients
        .iter()
        .map(|card| IngredientRecord {
            def_id: card.def_id.clone(),
            embiggened: card.embiggened == Some(true),
            parts: ingredients_of(card),
        })
        .collect();
    if records
        .iter()
        .all(|record| record.embiggened == own && record.parts.is_none())
    {
        None
    } else {
        Some(records)
    }
}

/// One text a card carries: its static flags and the price it was played for (`texts_of`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextFlags {
    pub flags: StaticFlags,
    pub embiggened: bool,
}

/// Every text a card carries with its own static flags and the price it was played for: one for a
/// card that records no prices (a fused card's summed flags among them, all at its one price), one
/// per ingredient for one that does (R102). A Vanilla card carries none (§6.3, R115).
pub fn texts_of(state: &GameState, instance: &CardInstance) -> Vec<TextFlags> {
    if instance.vanilla {
        return Vec::new();
    }
    let Some(records) = ingredients_of(instance) else {
        return vec![TextFlags {
            flags: flags_of(state, instance),
            embiggened: instance.embiggened == Some(true),
        }];
    };
    fn walk(state: &GameState, radiant: bool, list: &[IngredientRecord]) -> Vec<TextFlags> {
        list.iter()
            .flat_map(|record| match record.parts.as_ref() {
                Some(parts) => walk(state, radiant, parts),
                None => {
                    let entry = scripts_for(state, &record.def_id);
                    let face = if radiant { entry.radiant } else { entry.base };
                    vec![TextFlags {
                        flags: face.flags(),
                        embiggened: record.embiggened,
                    }]
                }
            })
            .collect()
    }
    walk(state, instance.radiant, &records)
}
