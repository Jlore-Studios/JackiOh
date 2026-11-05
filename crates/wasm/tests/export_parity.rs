//! #1505: the browser modules wrap one set of bodies, so a binding cannot be added to one side and
//! forgotten in the other. This test compares each module's export list to a declared allow-list: a
//! binding bound in only one module fails it, unless the allow-list says so with the reason. #3093 adds
//! a third wrapper, the C module (`crates/wasm/c`): it exports every binding the browser module binds,
//! so a binding added for the web reaches a host that is not JavaScript by itself. It uses `std` only
//! and reads files at run time.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The bindings both modules export: every body in `jackioh-wasm-core` both wrappers bind.
const SHARED: &[&str] = &[
    "init",
    "catalog",
    "catalog_version",
    "carried_versions",
    "run_content",
    "run_check_setup",
    "run_create",
    "run_reduce",
    "run_fold",
    "run_fight_setup",
    "run_fight_report",
    "run_score",
    "run_embers",
    "run_max_hp",
    "run_level_cost",
    "run_constants",
    "run_seed",
    "run_daily_seed",
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
    "lethal_hint",
    "build_ai_deck",
    "choose_action",
    "constants",
    "craft_preview",
    "engine_tables",
    "validator",
];

/// What only the normal module binds, and why: R1011000's fixture, which only the normal build is
/// held to (`apps/web/src/wasm/learned.test.ts`), #936's Duos bindings for `/dev/duos`, whose
/// game holds no custom card (`apps/web/src/wasm/index.ts` calls them on `normal`), and #3098's
/// `series`, the Conquest series rules of `jackioh-shared`, and #3106's `face`, its card face model, which
/// no custom-card practice game or card creator plays, so the Luau module that leaves with #3097 never
/// binds them. Their other wrapper is the C module's (#3093), as the host contract of
/// `crates/shared/README.md` says.
const NORMAL_ONLY: &[&str] = &[
    "learned_parity",
    "duos_create",
    "duos_begin",
    "duos_reduce",
    "duos_legal_actions",
    "duos_view_for",
    "series",
    "face",
];

/// What only the Luau module binds, and why: the creator's check (#913, #919) and the C
/// boundary's strings.
const LUAU_ONLY: &[&str] = &["check_custom_card", "jackioh_free", "jackioh_last_error"];

/// What only the C module exports, and why: its memory interface, since a host that is not JavaScript
/// reads and writes the module's memory itself.
const C_ONLY: &[(&str, &str)] = &[
    ("jackioh_alloc", C_MEMORY),
    ("jackioh_free", C_MEMORY),
    ("jackioh_result_ptr", C_MEMORY),
    ("jackioh_result_len", C_MEMORY),
];

const C_MEMORY: &str = "the C module's memory interface (`crates/wasm/c/README.md`)";

/// What the Luau module binds and the C module does not, and why: the custom-card bindings and the
/// Luau module's own error channel, which leave with part 5 of #2956 (the Luau host's removal).
const LUAU_NOT_IN_C: &[(&str, &str)] = &[
    (
        "check_custom_card",
        "the creator's check needs the Luau VM, which the C module holds none of; leaves with part 5",
    ),
    (
        "jackioh_last_error",
        "the Luau module's NUL-terminated error string; the C module answers a failure's message in its \
         result buffer instead; leaves with part 5",
    ),
];

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

fn c_exports() -> BTreeSet<String> {
    exported(&read("crates/wasm/c/src/lib.rs"), "#[unsafe(no_mangle)]")
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

/// One message for every difference between the C module and the other two: a browser binding it
/// lacks, an export of its own that `C_ONLY` does not name, a Luau binding it neither has nor
/// `LUAU_NOT_IN_C` explains, and a listed name that no longer holds.
fn c_parity_problems(
    normal: &BTreeSet<String>,
    c: &BTreeSet<String>,
    luau: &BTreeSet<String>,
) -> Vec<String> {
    let c_only: BTreeSet<&str> = C_ONLY.iter().map(|(name, _)| *name).collect();
    let luau_not_in_c: BTreeSet<&str> = LUAU_NOT_IN_C.iter().map(|(name, _)| *name).collect();
    let mut problems = Vec::new();
    for name in normal {
        if !c.contains(name) {
            problems.push(format!(
                "`{name}` is bound in crates/wasm/src/lib.rs but the C module does not export it: \
                 export it in crates/wasm/c/src/lib.rs (and crates/wasm/c/src/tests.rs's test of it), \
                 or drop the binding"
            ));
        }
    }
    for name in c {
        if !normal.contains(name) && !c_only.contains(name.as_str()) {
            problems.push(format!(
                "`{name}` is exported in crates/wasm/c/src/lib.rs only: bind it in \
                 crates/wasm/src/lib.rs too, or list it in C_ONLY with the reason"
            ));
        }
    }
    for (name, _) in C_ONLY {
        if !c.contains(*name) {
            problems.push(format!(
                "`{name}` is listed in C_ONLY but crates/wasm/c/src/lib.rs does not export it"
            ));
        }
    }
    for name in luau {
        if !c.contains(name) && !luau_not_in_c.contains(name.as_str()) {
            problems.push(format!(
                "`{name}` is bound in crates/wasm/luau/src/main.rs but the C module does not export it: \
                 export it in crates/wasm/c/src/lib.rs, or list it in LUAU_NOT_IN_C with the reason"
            ));
        }
    }
    for (name, _) in LUAU_NOT_IN_C {
        if !luau.contains(*name) {
            problems.push(format!(
                "`{name}` is listed in LUAU_NOT_IN_C but crates/wasm/luau/src/main.rs does not bind it"
            ));
        }
    }
    problems
}

#[test]
fn the_c_module_exports_every_browser_binding_and_its_memory_interface_alone() {
    let problems = c_parity_problems(&normal_exports(), &c_exports(), &luau_exports());
    assert!(problems.is_empty(), "C export parity:\n{}", problems.join("\n"));
}

/// The acceptance criterion as a test: a browser binding the C module lacks fails parity.
#[test]
fn a_browser_binding_the_c_module_lacks_fails_c_parity() {
    let text = read("crates/wasm/src/lib.rs") + "\n#[wasm_bindgen]\npub fn only_here() {}\n";
    let normal = exported(&text, "#[wasm_bindgen]");
    let problems = c_parity_problems(&normal, &c_exports(), &luau_exports());
    assert!(
        problems.iter().any(|problem| problem.contains("only_here")),
        "expected a C parity problem for only_here, got: {problems:?}"
    );
}

/// The C module is one plain `wasm32-unknown-unknown` link with an empty import section (#3093):
/// nothing it depends on or calls comes from the host. `scripts/build-wasm.sh` checks the built
/// module's imports when it builds it; this holds the sources to the same, where no build runs.
#[test]
fn the_c_module_imports_nothing_from_the_host() {
    // Comments may name what the module avoids; code and manifest lines may not.
    let code = |path: &str, comment: &str| -> String {
        read(path)
            .lines()
            .filter(|line| !line.trim_start().starts_with(comment))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let manifest = code("crates/wasm/c/Cargo.toml", "#");
    for banned in ["wasm-bindgen", "js-sys", "console_error_panic_hook", "wasi"] {
        assert!(
            !manifest.to_lowercase().contains(banned),
            "crates/wasm/c/Cargo.toml names {banned}, which the C module may not take"
        );
    }
    let source = code("crates/wasm/c/src/lib.rs", "//");
    for banned in [
        "extern \"C\" {",
        "wasm_bindgen",
        "js_sys",
        "SystemTime",
        "Instant::now",
    ] {
        assert!(
            !source.contains(banned),
            "crates/wasm/c/src/lib.rs uses {banned}, which the C module may not"
        );
    }
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
