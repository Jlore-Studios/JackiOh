//! R5039000: "Aura:" labels standing text that works while its card acts face-up on the field and
//! reaches beyond the card (other cards on either side, a player's rules or a hero), and nothing else
//! (§6.2's Aura row). The measure goes by what each face's script sets, not by its words: a face whose
//! script sets a standing hook prints a line that starts "Aura:", and a face that prints one sets one;
//! the few that differ are listed below, each with its reason.
//!
//! Not counted as standing hooks:
//! - the card's own text, which prints bare as a keyword does: `conditional_keywords`, `set_stat`,
//!   `stat_line_when`, `plague_multiplier`, `targeting_discards`, `combat_keywords_follow`;
//! - text that is labeled or timed by its own words: the trigger lists, `tribute_when`, `evolves_when`,
//!   `radiant_at_keywords`, `would_counter`, `activations`, `quests`, `stages`.
//!
//! A static flag is sorted by name into `AURA_FLAGS` or `NOT_AURA_FLAGS`, so a flag added later fails
//! here until it is sorted.

use jackioh_cards::{CATALOG, card_def, scripts_of};
use jackioh_engine::{CardType, FaceKind, FlagOrCount, ReplacementWhere, Script, StaticFlags};

use super::radiant_stats::{CATALOG_NOTE, TABLE_HEADER};

/// The static flags that are standing text reaching beyond their card.
const AURA_FLAGS: &[&str] = &[
    "antiOneshot",
    "heroArmor",
    "infiniteReserves",
    "giftedProgram",
    "quickstriker",
    "manaPerTurn",
    "radiantPlaysTagged",
    "noNaturalMana",
    "fengShui",
    "randomTargets",
    "creditLine",
    "creditInstalments",
    "doublesHealing",
    "spellDamageAura",
    "embiggenGarden",
    "unitsNeverDefense",
    "pilotsActions",
    "pilotsTurn",
    "destroyedExiled",
    "enemyDestroyedExiled",
    "hintsPlays",
    "gauntlet",
    "warheroDraws",
    "damagePool",
    "spellEchoAura",
    "faceDownWard",
    "faceDownIndestructible",
    "randomOpponentSpellTargets",
    "opponentAttackLimit",
    "revealsOpponentsHand",
];

const KEYWORD: &str = "a keyword or play-time rule, printed as one (§6)";
const OWN: &str = "the card's own text (R5039000)";
const TRAP: &str = "a Trap's firing (§5.2)";
const TRIGGER: &str = "a trigger's: its line names its moment";
const ELSEWHERE: &str = "works off the field: a Spell's, a hand's or a deck's";

/// The static flags that are not an aura, each with its reason.
const NOT_AURA_FLAGS: &[(&str, &str)] = &[
    ("castOnDraw", KEYWORD),
    ("quickdraw", KEYWORD),
    ("startsAtBottom", KEYWORD),
    ("tribute", KEYWORD),
    ("tributeWorth", KEYWORD),
    ("echo", KEYWORD),
    ("slumber", KEYWORD),
    ("starshipHull", KEYWORD),
    ("noHandLaunch", KEYWORD),
    ("starshipYard", KEYWORD),
    ("revealChance", KEYWORD),
    ("tributeEnemies", OWN),
    ("enemyTributeHandsOver", OWN),
    ("tributeCheap", OWN),
    ("attackFromHand", OWN),
    ("bounceAttacker", OWN),
    ("attackedOnlyFromLane", OWN),
    ("neverBerserk", OWN),
    ("neverDefense", OWN),
    ("immuneToTribalHate", OWN),
    ("tuneMultiplier", OWN),
    ("stackBase", OWN),
    ("stackBaseBuffs", OWN),
    ("laneMultiplier", OWN),
    ("stacksOnTarget", OWN),
    ("lendsEquip", OWN),
    ("fusesCarried", OWN),
    ("reflectsDamage", OWN),
    ("reflectsToAttacker", OWN),
    ("burnsOnDamage", OWN),
    ("damageAlsoBurns", OWN),
    ("toxicOnDamage", OWN),
    ("damageIsToxic", OWN),
    ("freezesDamaged", OWN),
    ("sweepLane", OWN),
    ("sweepAdjacent", OWN),
    ("attacksOnlyAcross", OWN),
    ("immuneToToxic", OWN),
    ("healToDamage", TRAP),
    ("ownTurnRedirect", TRAP),
    ("countersRanged", TRAP),
    ("countersRangedAll", TRAP),
    ("attackJoiners", TRIGGER),
    ("judgesPlays", TRIGGER),
    ("hearsEmotes", TRIGGER),
    ("exilesOnDamage", TRIGGER),
    ("infiniteMana", TRIGGER),
    ("creditLapses", TRIGGER),
    ("countsPlays", ELSEWHERE),
    ("returnKeepsPrice", ELSEWHERE),
    ("copiesLastSpell", ELSEWHERE),
    ("copyFallback", ELSEWHERE),
    ("paysCredits", ELSEWHERE),
    ("answersTargetedSpells", ELSEWHERE),
];

/// Cards whose script sets a standing hook while no line of the face starts "Aura:".
const SCRIPTED_BARE: &[(&str, &str)] = &[
    (
        "classic-069",
        "Plague Charger's aura raises its own Attack (R5039000)",
    ),
    (
        "classic-074",
        "Corpse Plantation's graveyard permission is its Activate's (R454)",
    ),
    (
        "classic-090",
        "In Too Deep's aura and graveyard permission are its quest rewards (R404)",
    ),
    (
        "infiniteassault-080",
        "Hive Ganger's aura raises its own Attack (R5039000)",
    ),
    (
        "stinkyscapeplus-087",
        "Frontier Flora's aura gives itself its stat line (R5039000)",
    ),
    (
        "stinkyscapeplus-163",
        "Accelerationist's aura raises its own stats (R5039000)",
    ),
];

/// Cards that print "Aura:" while their script sets no standing hook.
const PRINTED_UNSCRIPTED: &[(&str, &str)] = &[
    (
        "classic-087",
        "Plague Chalice counters plays by a field trigger (R667)",
    ),
    (
        "meditative-011",
        "Double Header's line is a field trigger on plays (R826)",
    ),
    ("stinkyscape-037", "Kwzka Allures by a field trigger (R1876004)"),
    (
        "stinkyscapeplus-074",
        "Galaxia's line is a field trigger on plays (R3568000)",
    ),
    (
        "stinkyscape-148",
        "the engine gives Alteran Reach's travel (R1847001)",
    ),
];

/// The keys of the static flags a script sets, as `catalog.json` and the wire spell them.
fn flag_keys(script: &Script) -> Vec<String> {
    match serde_json::to_value(script.flags()) {
        Ok(serde_json::Value::Object(map)) => map.keys().cloned().collect(),
        other => panic!("StaticFlags serialize to an object: {other:?}"),
    }
}

/// The standing hooks a face's script sets: what it reads while the card acts on the field and reaches
/// beyond the card. A Trap's replacement is its firing, so it is not counted on a Trap's face.
fn standing_hooks(script: &Script, trap: bool) -> Vec<String> {
    let fields = [
        ("aura", script.aura.is_some()),
        ("cost_aura", script.cost_aura.is_some()),
        ("hero_guard", script.hero_guard.is_some()),
        ("draw_limit", script.draw_limit.is_some()),
        ("discard_guard", script.discard_guard.is_some()),
        ("attack_mods", script.attack_mods.is_some()),
        ("graveyard_play", script.graveyard_play.is_some()),
        ("reveal_echo", script.reveal_echo.is_some()),
        ("turn_hook_extra", script.turn_hook_extra.is_some()),
        ("cry_death_extra", script.cry_death_extra.is_some()),
        ("death_extra", script.death_extra.is_some()),
        ("reborn_extra", script.reborn_extra.is_some()),
        ("evolve_extra", script.evolve_extra.is_some()),
        ("trades_per_turn", script.trades_per_turn.is_some()),
        ("exile_auras", !script.exile_auras.is_empty()),
        ("resolves_set_spells", script.resolves_set_spells),
    ];
    let mut hooks: Vec<String> = fields
        .into_iter()
        .filter(|(_, set)| *set)
        .map(|(name, _)| name.to_string())
        .collect();
    let field_replacement = script
        .replacements
        .iter()
        .any(|replacement| matches!(replacement.where_, None | Some(ReplacementWhere::Field)));
    if !trap && field_replacement {
        hooks.push("replacements".to_string());
    }
    hooks.extend(
        flag_keys(script)
            .into_iter()
            .filter(|key| AURA_FLAGS.contains(&key.as_str())),
    );
    hooks
}

/// Whether a line of the face text starts "Aura:".
fn prints_aura(text: &str) -> bool {
    text.split('\n').any(|line| line.starts_with("Aura:"))
}

/// One face of one catalog entry, as the measure reads it.
struct Face {
    id: String,
    face: FaceKind,
    hooks: Vec<String>,
    printed: bool,
    flags: Vec<String>,
}

/// Both faces of every catalog entry, the held set's included.
fn measured() -> Vec<Face> {
    let scripts = scripts_of();
    let mut faces = Vec::new();
    for (id, def) in CATALOG.iter() {
        let Some(card) = scripts.get(id) else {
            panic!("{id} has no script (CLAUDE.md rule 6)");
        };
        for (face, script) in [(FaceKind::Base, &card.base), (FaceKind::Radiant, &card.radiant)] {
            let printed_face = def.face(face);
            let trap = matches!(
                printed_face.type_.unwrap_or(def.type_),
                CardType::Trap | CardType::FieldTrap
            );
            faces.push(Face {
                id: id.clone(),
                face,
                hooks: standing_hooks(script, trap),
                printed: prints_aura(&printed_face.text),
                flags: flag_keys(script),
            });
        }
    }
    faces
}

fn listed(list: &[(&str, &str)], id: &str) -> bool {
    list.iter().any(|(listed_id, _)| *listed_id == id)
}

#[test]
fn r5039000_every_face_whose_script_sets_a_standing_hook_prints_aura_and_every_aura_face_sets_one() {
    let mut problems = Vec::new();
    for face in measured() {
        let (id, side) = (&face.id, face.face.as_str());
        if !face.hooks.is_empty() && !face.printed && !listed(SCRIPTED_BARE, id) {
            problems.push(format!(
                "{id} {side}: its script sets {:?} but no line starts \"Aura:\"",
                face.hooks
            ));
        }
        if face.printed && face.hooks.is_empty() && !listed(PRINTED_UNSCRIPTED, id) {
            problems.push(format!(
                "{id} {side}: prints \"Aura:\" but its script sets no standing hook"
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "R5039000: every face whose script sets a standing hook prints a line that starts \"Aura:\", and \
         every face that prints one sets one (§6.2, Aura); print the label, take it off, or list the card \
         in SCRIPTED_BARE or PRINTED_UNSCRIPTED with its reason:\n{}",
        problems.join("\n")
    );
}

#[test]
fn r5039000_every_static_flag_a_card_sets_is_sorted_once_with_its_reason() {
    let faces = measured();
    let mut problems = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for face in &faces {
        for key in &face.flags {
            if seen.contains(&key.as_str()) {
                continue;
            }
            seen.push(key);
            let aura = AURA_FLAGS.contains(&key.as_str());
            let not_aura = NOT_AURA_FLAGS.iter().any(|(listed, _)| listed == key);
            if aura == not_aura {
                problems.push(format!(
                    "{key} (set by {} {}) is in {} of AURA_FLAGS and NOT_AURA_FLAGS",
                    face.id,
                    face.face.as_str(),
                    if aura { "both" } else { "neither" }
                ));
            }
        }
    }
    let listed_keys = AURA_FLAGS
        .iter()
        .copied()
        .chain(NOT_AURA_FLAGS.iter().map(|(key, _)| *key));
    for key in listed_keys {
        if !seen.contains(&key) {
            problems.push(format!("{key} is listed but no card's script sets it"));
        }
    }
    assert!(
        problems.is_empty(),
        "R5039000: every static flag a card sets is an aura or not, said once:\n{}",
        problems.join("\n")
    );
}

#[test]
fn r5039000_every_listed_face_still_needs_its_entry() {
    let faces = measured();
    let mut problems = Vec::new();
    for (id, reason) in SCRIPTED_BARE {
        if !faces
            .iter()
            .any(|face| face.id == *id && !face.hooks.is_empty() && !face.printed)
        {
            problems.push(format!(
                "{id} is in SCRIPTED_BARE ({reason}) but no face of it sets a standing hook and prints no \"Aura:\""
            ));
        }
    }
    for (id, reason) in PRINTED_UNSCRIPTED {
        if !faces
            .iter()
            .any(|face| face.id == *id && face.hooks.is_empty() && face.printed)
        {
            problems.push(format!(
                "{id} is in PRINTED_UNSCRIPTED ({reason}) but no face of it prints \"Aura:\" and sets no standing hook"
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "R5039000: every allowlisted card still differs from the measure:\n{}",
        problems.join("\n")
    );
}

#[test]
fn r5039000_the_measure_flags_a_bare_aura_and_passes_own_text() {
    let armor = Script {
        static_flags: Some(StaticFlags {
            hero_armor: Some(FlagOrCount::Flag(true)),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    assert_eq!(standing_hooks(&armor, false), vec!["heroArmor".to_string()]);
    let burns = Script {
        static_flags: Some(StaticFlags {
            burns_on_damage: Some(2),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    assert!(
        standing_hooks(&burns, false).is_empty(),
        "its own damage burning is its own text"
    );
    assert!(standing_hooks(&Script::default(), false).is_empty());

    assert!(prints_aura("Cry: Draw 1.\nAura: X."));
    assert!(!prints_aura("Has 2. Aura: X."), "the label starts its line");

    let big_d = &scripts_of()["core-001"].base;
    assert!(
        !standing_hooks(big_d, false).is_empty(),
        "Big D-fender's aura is a standing hook"
    );
    let unlabeled = card_def("core-001").base.text.replacen("Aura: ", "", 1);
    assert!(
        !prints_aura(&unlabeled),
        "Big D-fender without its label prints no \"Aura:\""
    );
}

/// Every row of `note`'s §8 card tables whose Base effect cell says "Aura:" where its catalog entry's
/// base face prints no "Aura:" line, or the reverse, as `<id>: …`; and every row that names no catalog
/// entry by its index and name. A row's cell restates the face in prose, keywords and lines run
/// together, so the label may stand anywhere in it (`Armor 4. Aura: Lethal damage …`).
fn section_eight_mismatches(note: &str) -> Vec<String> {
    let header: Vec<&str> = TABLE_HEADER
        .trim_matches('|')
        .split(" | ")
        .map(str::trim)
        .collect();
    let index_at = header.iter().position(|cell| *cell == "#").unwrap();
    let name_at = header.iter().position(|cell| *cell == "Name").unwrap();
    let base_at = header.iter().position(|cell| *cell == "Base effect").unwrap();
    let same_name = |name: &str| name.replace('’', "'");
    let mut problems = Vec::new();
    let mut in_table = false;
    for line in note.lines() {
        if line == TABLE_HEADER {
            in_table = true;
            continue;
        }
        if !line.starts_with('|') {
            in_table = false;
            continue;
        }
        if !in_table || line.starts_with("| ---") {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split(" | ").map(str::trim).collect();
        let (Some(index), Some(name), Some(base)) =
            (cells.get(index_at), cells.get(name_at), cells.get(base_at))
        else {
            problems.push(format!("a row too short to read: {line}"));
            continue;
        };
        let Some((id, def)) = CATALOG
            .iter()
            .find(|(_, def)| def.index == *index && same_name(&def.name) == same_name(name))
        else {
            problems.push(format!(
                "#{index} {name}: no catalog entry has this index and name"
            ));
            continue;
        };
        let printed = prints_aura(&def.base.text);
        if printed != base.contains("Aura:") {
            problems.push(format!(
                "{id}: the catalog's base face {} \"Aura:\" but its §8 row's Base effect cell {} (\"{base}\")",
                if printed { "prints" } else { "prints no" },
                if printed { "does not" } else { "does" },
            ));
        }
    }
    problems
}

#[test]
fn r5039000_every_section_eight_row_says_aura_where_its_card_prints_it() {
    let problems = section_eight_mismatches(CATALOG_NOTE);
    assert!(
        problems.is_empty(),
        "§8 rows that disagree with catalog.json on \"Aura:\":\n{}",
        problems.join("\n")
    );
}

#[test]
fn r5039000_a_drone_row_left_with_its_old_aura_fails_the_section_eight_check() {
    let row = CATALOG_NOTE
        .lines()
        .find(|line| line.starts_with("| T-drone-interceptor |"))
        .expect("§8.10 has the Drone Interceptor's row");
    let old = row.replacen("Ranged, Flying. Has 1", "Ranged, Flying. Aura: has 1", 1);
    assert_ne!(old, row, "the row prints its keywords line bare");
    let problems = section_eight_mismatches(&format!("{TABLE_HEADER}\n| --- |\n{old}"));
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with("stinkyscape-t-drone-interceptor: "),
        "{problems:?}"
    );
}
