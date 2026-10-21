//! #1505: the two browser modules wrap one set of bodies, so a binding cannot be added to one
//! side and forgotten in the other. This test compares each module's export list to a declared
//! allow-list: a binding bound in only one module fails it, unless the allow-list says so with the
//! reason. It uses `std` only and reads files at run time.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The bindings both modules export: every body in `jackioh-wasm-core` both wrappers bind.
const SHARED: &[&str] = &[
    "init",
    "catalog",
    "catalog_version",
    "carried_versions",
    "create_game",
    "begin_game",
    "reduce",
    "legal_actions",
    "view_for",
    "seat_to_act",
    "hash_state",
    "fold",
    "replay_open",
    "replay_page",
    "last_board_for",
    "seat_played_by",
    "find_instance",
    "deal_emote_hand",
    "preview_sets",
    "ai_to_act",
    "ai_decide",
    "build_ai_deck",
    "choose_action",
    "constants",
    "craft_preview",
    "engine_tables",
    "validator",
];

/// What only the normal module binds, and why: R1011000's fixture, which only the normal build is
/// held to (`apps/web/src/wasm/learned.test.ts`), and #936's Duos bindings for `/dev/duos`, whose
/// game holds no custom card (`apps/web/src/wasm/index.ts` calls them on `normal`).
const NORMAL_ONLY: &[&str] = &[
    "learned_parity",
    "duos_create",
    "duos_begin",
    "duos_reduce",
    "duos_legal_actions",
    "duos_view_for",
];

/// What only the Luau module binds, and why: the creator's check (#913, #919) and the C
/// boundary's strings.
const LUAU_ONLY: &[&str] = &["check_custom_card", "jackioh_free", "jackioh_last_error"];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    std::fs::read_to_string(repo().join(path)).unwrap_or_else(|_| panic!("cannot read {path}"))
}

/// The functions a module binds: for each line that trims to `attribute`, the `fn` name on the
/// next line.
fn exported(source: &str, attribute: &str) -> BTreeSet<String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut names = BTreeSet::new();
    for (index, line) in lines.iter().enumerate() {
        if line.trim() == attribute {
            let next = lines
                .get(index + 1)
                .unwrap_or_else(|| panic!("{attribute} at line {} has no next line", index + 1));
            let at = next
                .find("fn ")
                .unwrap_or_else(|| panic!("no `fn ` after {attribute} at line {}", index + 2));
            let rest = &next[at + "fn ".len()..];
            let end = rest
                .find('(')
                .unwrap_or_else(|| panic!("no `(` after {attribute} at line {}", index + 2));
            names.insert(rest[..end].trim().to_owned());
        }
    }
    names
}

fn normal_exports() -> BTreeSet<String> {
    exported(&read("crates/wasm/src/lib.rs"), "#[wasm_bindgen]")
}

fn luau_exports() -> BTreeSet<String> {
    exported(&read("crates/wasm/luau/src/main.rs"), "#[unsafe(no_mangle)]")
}

/// One message for every export the allow-list does not account for: a name a module binds that
/// `SHARED` plus its own list lacks, and a listed name the module does not bind.
fn parity_problems(normal: &BTreeSet<String>, luau: &BTreeSet<String>) -> Vec<String> {
    let allowed_normal: BTreeSet<&str> = SHARED.iter().chain(NORMAL_ONLY.iter()).copied().collect();
    let allowed_luau: BTreeSet<&str> = SHARED.iter().chain(LUAU_ONLY.iter()).copied().collect();
    let mut problems = Vec::new();
    for name in normal {
        if !allowed_normal.contains(name.as_str()) {
            if luau.contains(name) {
                problems.push(format!(
                    "`{name}` is bound in both modules but the allow-list names it in neither: \
                     list it in SHARED, or in NORMAL_ONLY and LUAU_ONLY with the reason"
                ));
            } else {
                problems.push(format!(
                    "`{name}` is bound in crates/wasm/src/lib.rs only: bind it in \
                     crates/wasm/luau/src/main.rs too (and .cargo/config.toml's \
                     EXPORTED_FUNCTIONS, scripts/build-wasm.sh's types, \
                     apps/web/src/wasm/luau.ts), or list it in NORMAL_ONLY with the reason"
                ));
            }
        }
    }
    for name in luau {
        if !allowed_luau.contains(name.as_str()) {
            if normal.contains(name) {
                // Reported from the normal side above; one message per name.
            } else {
                problems.push(format!(
                    "`{name}` is bound in crates/wasm/luau/src/main.rs only: bind it in \
                     crates/wasm/src/lib.rs too, or list it in LUAU_ONLY with the reason"
                ));
            }
        }
    }
    for name in SHARED.iter().chain(NORMAL_ONLY.iter()) {
        if !normal.contains(*name) {
            problems.push(format!(
                "`{name}` is listed for the normal module but crates/wasm/src/lib.rs does not bind it"
            ));
        }
    }
    for name in SHARED.iter().chain(LUAU_ONLY.iter()) {
        if !luau.contains(*name) {
            problems.push(format!(
                "`{name}` is listed for the Luau module but crates/wasm/luau/src/main.rs does not bind it"
            ));
        }
    }
    problems
}

#[test]
fn the_two_modules_export_the_shared_bindings_and_their_own_alone() {
    let problems = parity_problems(&normal_exports(), &luau_exports());
    assert!(problems.is_empty(), "export parity:\n{}", problems.join("\n"));
}

/// The emscripten link and the factory types name every C binding: `.cargo/config.toml`'s
/// `EXPORTED_FUNCTIONS` and the `JackiohWasmLuauModule` interface `scripts/build-wasm.sh` writes.
#[test]
fn the_luau_link_and_its_types_name_every_c_binding() {
    let luau = luau_exports();
    let config = read(".cargo/config.toml");
    let marker = "-sEXPORTED_FUNCTIONS=";
    let at = config
        .find(marker)
        .unwrap_or_else(|| panic!("no {marker} in .cargo/config.toml"));
    let rest = &config[at + marker.len()..];
    let end = rest
        .find('"')
        .unwrap_or_else(|| panic!("no closing quote after {marker}"));
    let linked: BTreeSet<String> = rest[..end]
        .split(',')
        .map(|name| name.strip_prefix('_').unwrap_or(name).to_owned())
        .collect();
    let mut expected: BTreeSet<String> = luau.clone();
    expected.insert("main".to_owned());
    assert_eq!(linked, expected, "EXPORTED_FUNCTIONS names every C binding");

    let script = read("scripts/build-wasm.sh");
    let interface = "export interface JackiohWasmLuauModule {";
    let start = script
        .lines()
        .position(|line| line.trim() == interface)
        .unwrap_or_else(|| panic!("no {interface} in scripts/build-wasm.sh"));
    let typed: BTreeSet<String> = script
        .lines()
        .skip(start + 1)
        .take_while(|line| line.trim() != "}")
        .filter_map(|line| {
            let trimmed = line.trim().strip_prefix('_')?;
            Some(trimmed[..trimmed.find('(').unwrap_or(trimmed.len())].to_owned())
        })
        .collect();
    assert_eq!(typed, luau, "the factory types name every C binding");
}

/// The acceptance criterion as a test: a binding bound in one module only fails parity.
#[test]
fn a_binding_bound_in_one_module_only_fails_parity() {
    let text = read("crates/wasm/src/lib.rs") + "\n#[wasm_bindgen]\npub fn only_here() {}\n";
    let normal = exported(&text, "#[wasm_bindgen]");
    let problems = parity_problems(&normal, &luau_exports());
    assert!(
        problems.iter().any(|problem| problem.contains("only_here")),
        "expected a parity problem for only_here, got: {problems:?}"
    );
}
