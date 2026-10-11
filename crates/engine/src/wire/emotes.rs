//! Emotes and hero portraits (patch v0.2.X, SPEC §9.4 D5, §9.5, §10.10, §10.11, R641–R645; the
//! dealt hand of patch v0.3.X's MN03, #545, R1340–R1345).
//!
//! Everything here is cosmetic, except while a card hears emotes: an emote is never an `ActionBody`,
//! never reaches `reduce`, the action log, the replay hash or a game record, and a portrait or an
//! emote hand is never part of `PlayerView` (R643, R1342; revised by R1127: an `Emote` action exists
//! exactly while a card with `StaticFlags.hears_emotes` acts). This module holds only what BOTH sides
//! of the wire must agree on — the id lists, the portrait roster the deck save checks (D5, R641), the
//! hand each seat is dealt (R1341) and the rate limit the client and the server enforce identically
//! (R643) — because `apps/web` and the server may not import each other
//! (§9.2: a client and a server are separate deployables). Constants therefore live here and not
//! in the server's `config.rs` (CLAUDE.md rule 9): the rule books numbers to one named place,
//! and this module is the one place both ends read.
//!
//! The AI's persona config is NOT here: it is presentation logic of the practice opponent and
//! lives in the web client (R645), which the engine and the AI's move search never import.
//!
//! Port of `packages/shared/src/emotes.ts` (SURFACE §4.1, §10.4); the web keeps its TS copy as
//! `apps/web/src/wire/emotes.ts`. Its tests are `packages/shared/test/emotes.test.ts`, below.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::rng::Rng;
use crate::wire::catalog_types::PlayerId;
use crate::wire::string_union;

string_union! {
    /// The pool of twenty-four emotes (R1340): the five voice lines first (each portrait has its
    /// own text), then the nineteen shared animated emoji — the five of patch v0.2.X and the
    /// fourteen of MN03 (#545). The wire spells them exactly like this (R643).
    pub enum EmoteId {
        Greetings = "greetings",
        WellPlayed = "wellPlayed",
        Oops = "oops",
        Thanks = "thanks",
        Threaten = "threaten",
        Sob = "sob",
        Yawn = "yawn",
        Laugh = "laugh",
        Angry = "angry",
        WahWah = "wahWah",
        Wave = "wave",
        Clap = "clap",
        ThumbsUp = "thumbsUp",
        Facepalm = "facepalm",
        Shrug = "shrug",
        Thinking = "thinking",
        Heart = "heart",
        Fire = "fire",
        Skull = "skull",
        Sweat = "sweat",
        Cool = "cool",
        Gasp = "gasp",
        Salute = "salute",
        Party = "party",
    }
}

/// The five voice lines (R643).
pub const VOICE_EMOTE_IDS: &[EmoteId] = &[
    EmoteId::Greetings,
    EmoteId::WellPlayed,
    EmoteId::Oops,
    EmoteId::Thanks,
    EmoteId::Threaten,
];
/// The nineteen shared animated emoji (R643, R1340): patch v0.2.X's five, then MN03's fourteen.
pub const EMOJI_EMOTE_IDS: &[EmoteId] = &[
    EmoteId::Sob,
    EmoteId::Yawn,
    EmoteId::Laugh,
    EmoteId::Angry,
    EmoteId::WahWah,
    EmoteId::Wave,
    EmoteId::Clap,
    EmoteId::ThumbsUp,
    EmoteId::Facepalm,
    EmoteId::Shrug,
    EmoteId::Thinking,
    EmoteId::Heart,
    EmoteId::Fire,
    EmoteId::Skull,
    EmoteId::Sweat,
    EmoteId::Cool,
    EmoteId::Gasp,
    EmoteId::Salute,
    EmoteId::Party,
];
/// `[...VOICE_EMOTE_IDS, ...EMOJI_EMOTE_IDS]`, the whole pool (R1340): `EmoteId`'s declaration
/// order is exactly that.
pub const EMOTE_IDS: &[EmoteId] = EmoteId::ALL;

/// TS's `(typeof VOICE_EMOTE_IDS)[number]`: one of the voice lines (`is_voice_emote` tells).
pub type VoiceEmoteId = EmoteId;
/// TS's `(typeof EMOJI_EMOTE_IDS)[number]`: one of the emoji (`!is_voice_emote`).
pub type EmojiEmoteId = EmoteId;

/// TS `isEmoteId(value: unknown)`: a string that is one of the pool's ids.
pub fn is_emote_id(value: &Value) -> bool {
    match value.as_str() {
        Some(text) => EMOTE_IDS.iter().any(|id| id.as_str() == text),
        None => false,
    }
}

pub fn is_voice_emote(emote: EmoteId) -> bool {
    VOICE_EMOTE_IDS.contains(&emote)
}

// ---------------------------------------------------------------------------------------------
// The emote hand (R1341): eight of the pool, dealt to each seat each game from the match seed.
// ---------------------------------------------------------------------------------------------

/// How many emotes a seat is dealt each game (R1341): the menu shows these and nothing else.
pub const EMOTE_HAND_SIZE: usize = 8;
/// How many of a hand are voice lines (R1341); the other `EMOTE_HAND_SIZE - EMOTE_HAND_VOICE` are
/// emoji. A fixed split, so every hand speaks in its portrait's voice and the menu keeps one shape.
pub const EMOTE_HAND_VOICE: usize = 3;

/// The seed of a seat's own emote stream (R1341): the match seed and the seat, so the deal never
/// draws from the match rng (`state.rng`) and no state, log, replay hash or golden trace moves.
pub fn emote_hand_stream(seed: &str, seat: PlayerId) -> String {
    format!("{seed}:emotes:{seat}")
}

/// R1341: the hand `seat` is dealt in the match seeded `seed` — `EMOTE_HAND_VOICE` of the voice
/// lines and the rest of `EMOTE_HAND_SIZE` from the emoji, each a uniform draw without repeats
/// (Fisher–Yates on the seat's own stream, `emote_hand_stream`), listed in the pool's order so a
/// menu reads the same whatever order the draw took. A pure function of `(seed, seat)`: the server
/// deals it at match start and again after a restart, hotseat and practice deal it from the game
/// seed through WASM, and a reconnect or a replay sees the same eight.
pub fn deal_emote_hand(seed: &str, seat: PlayerId) -> Vec<EmoteId> {
    let mut rng = Rng::new(&emote_hand_stream(seed, seat), 0);
    let voices = rng.shuffle(VOICE_EMOTE_IDS);
    let emoji = rng.shuffle(EMOJI_EMOTE_IDS);
    let dealt: Vec<EmoteId> = voices
        .into_iter()
        .take(EMOTE_HAND_VOICE)
        .chain(emoji.into_iter().take(EMOTE_HAND_SIZE - EMOTE_HAND_VOICE))
        .collect();
    EMOTE_IDS
        .iter()
        .copied()
        .filter(|id| dealt.contains(id))
        .collect()
}

/// R1342: whether `hand` holds `emote` — the relay's check before it admits a send.
pub fn hand_holds(hand: &[EmoteId], emote: EmoteId) -> bool {
    hand.contains(&emote)
}

// ---------------------------------------------------------------------------------------------
// Hero portraits (R641)
// ---------------------------------------------------------------------------------------------

string_union! {
    /// A portrait's id. `vanilla` is the default everywhere a deck does not name one: a saved deck's
    /// `portrait` stays `null` and reads back as `vanilla` (R641).
    pub enum PortraitId {
        Vanilla = "vanilla",
        Gary = "gary",
        Timmy = "timmy",
        Dfender = "dfender",
        Felinors = "felinors",
        Shredder = "shredder",
    }
}

pub const PORTRAIT_IDS: &[PortraitId] = PortraitId::ALL;

pub const DEFAULT_PORTRAIT: PortraitId = PortraitId::Vanilla;

/// TS `isPortraitId(value: unknown)`: a string that is one of the six ids.
pub fn is_portrait_id(value: &Value) -> bool {
    match value.as_str() {
        Some(text) => PORTRAIT_IDS.iter().any(|id| id.as_str() == text),
        None => false,
    }
}

/// `null` means `vanilla` (R641): the column, the deck view and imports all read through this.
pub fn portrait_or_default(portrait: Option<&str>) -> PortraitId {
    match portrait {
        Some(text) => PORTRAIT_IDS
            .iter()
            .copied()
            .find(|id| id.as_str() == text)
            .unwrap_or(DEFAULT_PORTRAIT),
        None => DEFAULT_PORTRAIT,
    }
}

/// One entry of the launch roster (TS's anonymous `{ cardName: string; flavour: string }`). A
/// constant table, so it serialises but is never read back.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortraitEntry {
    pub card_name: &'static str,
    pub flavour: &'static str,
}

/// The launch roster. `card_name` is the Core card the portrait's art and voice come from, looked
/// up BY NAME in the catalog, never by number (the issue's roster table). `flavour` is the voice
/// direction the emote persona in `card-audio.json5` (`emote-<id>`) follows.
///
/// TS's `Record<PortraitId, …>`, in its key order (`PORTRAIT_IDS`'s).
pub const PORTRAITS: &[(PortraitId, PortraitEntry)] = &[
    (
        PortraitId::Vanilla,
        PortraitEntry {
            card_name: "Mr. Vanilla",
            flavour: "Flat, polite, unbothered",
        },
    ),
    (
        PortraitId::Gary,
        PortraitEntry {
            card_name: "Gary the Gambler",
            flavour: "Fast-talking card sharp",
        },
    ),
    (
        PortraitId::Timmy,
        PortraitEntry {
            card_name: "Tempo Timmy",
            flavour: "Hyper, rushed",
        },
    ),
    (
        PortraitId::Dfender,
        PortraitEntry {
            card_name: "Big D-fender",
            flavour: "Low, steady bodyguard",
        },
    ),
    (
        PortraitId::Felinors,
        PortraitEntry {
            card_name: "Duplicating Felinors",
            flavour: "Two cats talking at once",
        },
    ),
    (
        PortraitId::Shredder,
        PortraitEntry {
            card_name: "Jlockeed Shredder-10",
            flavour: "Robot, all caps",
        },
    ),
];

/// TS `PORTRAITS[id]`: the roster entry of `id` (every id has one).
pub fn portrait_entry(id: PortraitId) -> &'static PortraitEntry {
    match PORTRAITS.iter().find(|(key, _)| *key == id) {
        Some((_, entry)) => entry,
        None => panic!("portrait {id} has no roster entry"),
    }
}

/// A uniform pick over the roster (R642: All Random deals one per seat, independently).
pub fn pick_portrait(mut rng: impl FnMut() -> f64) -> PortraitId {
    let len = PORTRAIT_IDS.len();
    let drawn = (rng() * len as f64).floor();
    // `Math.min(length - 1, Math.floor(...))`; a negative draw (no rng gives one) lands on index 0.
    let index = if drawn < 0.0 {
        0
    } else {
        (drawn as usize).min(len - 1)
    };
    PORTRAIT_IDS.get(index).copied().unwrap_or(DEFAULT_PORTRAIT)
}

/// A tiny deterministic hash → [0,1), for server-side portrait deals keyed on the match seed.
fn hash_unit(seed: &str) -> f64 {
    let mut h: u32 = 0x811c_9dc5;
    for unit in seed.encode_utf16() {
        h ^= u32::from(unit);
        h = h.wrapping_mul(0x0100_0193);
    }
    // Avalanche (MurmurHash3's fmix32): FNV-1a diffuses a differing last character into a small
    // multiple of the prime, so `${seed}:portrait:p1` and `${seed}:portrait:p2` fell in the same
    // portrait bucket ~95% of the time instead of R642's independent ~1/6. The finalizer spreads
    // one-character differences over all 32 bits, so the seats draw independently.
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    f64::from(h) / 4_294_967_296.0
}

/// All Random's deal (R642): uniform, independent per seat, decided at match creation. Seeded like
/// `dealRandomDeck`'s per-seat draws (`${seed}:portrait:<seat>`) so it needs no extra entropy and a
/// test can reproduce it exactly.
pub fn pick_portrait_from_seed(seed: &str) -> PortraitId {
    pick_portrait(|| hash_unit(seed))
}

// ---------------------------------------------------------------------------------------------
// The rate limit (R643): one rule, one config, enforced at both ends.
// ---------------------------------------------------------------------------------------------

/// The pause between two emotes from one player.
pub const EMOTE_COOLDOWN_MS: i64 = 1500;
/// The rolling window the cap is counted over.
pub const EMOTE_WINDOW_MS: i64 = 20_000;
/// At most this many emotes in a window.
pub const EMOTE_WINDOW_MAX: usize = 5;

/// TS `EmoteGate = { ok: true; sentAt } | { ok: false; retryAfterMs; sentAt }`: `retry_after_ms`
/// is `Some` exactly when `ok` is false, so the JSON is TS's either way.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct EmoteGate {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "number"))]
    pub retry_after_ms: Option<i64>,
    /// Milliseconds, a JS `number` (ts-rs would write an `i64` as `bigint`).
    #[cfg_attr(feature = "ts", ts(type = "Array<number>"))]
    pub sent_at: Vec<i64>,
}

/// Whether `sent_at` (timestamps of this player's recent emotes, oldest first) admits one more at
/// `now`. Pure and total: the same function answers the server's drop and the client's grey-out,
/// so the two can never disagree (R643). `sent_at` is returned pruned to the window, so a caller
/// can keep one rolling array per player.
pub fn emote_gate(sent_at: &[i64], now: i64) -> EmoteGate {
    let kept: Vec<i64> = sent_at
        .iter()
        .copied()
        .filter(|at| now - at < EMOTE_WINDOW_MS)
        .collect();
    if let Some(&last) = kept.last()
        && now - last < EMOTE_COOLDOWN_MS
    {
        return EmoteGate {
            ok: false,
            retry_after_ms: Some(EMOTE_COOLDOWN_MS - (now - last)),
            sent_at: kept,
        };
    }
    if kept.len() >= EMOTE_WINDOW_MAX {
        let oldest = kept.first().copied().unwrap_or(now);
        return EmoteGate {
            ok: false,
            retry_after_ms: Some(EMOTE_WINDOW_MS - (now - oldest)),
            sent_at: kept,
        };
    }
    EmoteGate {
        ok: true,
        retry_after_ms: None,
        sent_at: kept,
    }
}

/// The emote a seat sends, as the server relays it to the opponent (R643).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct EmoteRelay {
    pub from: PlayerId,
    pub emote: EmoteId,
}

// Emotes and hero portraits (SPEC §9.4 D5, §9.5, §10.10, R641–R643, R1340, R1341).
//
// `wire/emotes.rs` is the one place both ends of the wire read the emote ids, the portrait roster,
// the hand's deal and the rate limit from, so what is proved here holds for the client's greyed menu
// items and the server's drop alike (R643), for the hand the actor sends and the one hotseat and
// practice deal over WASM (R1341), and for the deck save's D5 check, which reads the same roster
// through the caller's `is_portrait` — proved end to end in the validator's own drafts tests
// ("R641 …").
#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::{IndexMap, IndexSet};
    use serde_json::json;

    fn ids(list: &[EmoteId]) -> Vec<&'static str> {
        list.iter().map(|id| id.as_str()).collect()
    }

    fn portrait_ids(list: &[PortraitId]) -> Vec<&'static str> {
        list.iter().map(|id| id.as_str()).collect()
    }

    mod r643_r1340_the_pool_of_emote_ids_the_wire_spells {
        use super::*;

        #[test]
        fn r1340_admits_exactly_the_twenty_four_ids_the_five_voice_lines_then_the_nineteen_emoji() {
            assert_eq!(
                ids(VOICE_EMOTE_IDS),
                vec!["greetings", "wellPlayed", "oops", "thanks", "threaten"]
            );
            assert_eq!(
                ids(EMOJI_EMOTE_IDS),
                vec![
                    "sob", "yawn", "laugh", "angry", "wahWah", "wave", "clap", "thumbsUp", "facepalm",
                    "shrug", "thinking", "heart", "fire", "skull", "sweat", "cool", "gasp", "salute",
                    "party",
                ]
            );
            let both: Vec<EmoteId> = VOICE_EMOTE_IDS
                .iter()
                .chain(EMOJI_EMOTE_IDS.iter())
                .copied()
                .collect();
            assert_eq!(EMOTE_IDS.to_vec(), both);
            // MN03's floor: at least 24 in all, at least 14 of them new beside R643's ten.
            assert!(EMOTE_IDS.len() >= 24);
            assert_eq!(EMOTE_IDS.len(), 24);
            assert!(EMOJI_EMOTE_IDS.len() - 5 >= 14);
            let distinct: IndexSet<EmoteId> = EMOTE_IDS.iter().copied().collect();
            assert_eq!(distinct.len(), EMOTE_IDS.len());
            for id in EMOTE_IDS {
                assert!(is_emote_id(&json!(id.as_str())), "{id}");
            }
        }

        #[test]
        fn r643_r1340_keeps_the_ten_ids_of_patch_v0_2_x_first_and_spelt_as_they_were() {
            assert_eq!(
                ids(&EMOTE_IDS[..10]),
                vec![
                    "greetings",
                    "wellPlayed",
                    "oops",
                    "thanks",
                    "threaten",
                    "sob",
                    "yawn",
                    "laugh",
                    "angry",
                    "wahWah",
                ]
            );
        }

        #[test]
        fn r643_refuses_an_id_the_wire_does_not_spell_and_anything_that_is_not_a_string() {
            for bad in [
                "",
                "GREETINGS",
                "greetings ",
                " emote",
                "sobbing",
                "wahWah!",
                "Wave",
                "thumbs_up",
            ] {
                assert!(!is_emote_id(&json!(bad)), "{bad:?}");
            }
            // TS's `undefined` and `null` are both `Value::Null` here.
            for not_string in [
                json!(null),
                json!(0),
                json!(5),
                json!(true),
                json!({}),
                json!([]),
                json!(["greetings"]),
            ] {
                assert!(!is_emote_id(&not_string), "{not_string}");
            }
        }

        #[test]
        fn r643_sorts_every_emote_into_a_voice_line_or_an_emoji_never_both_never_neither() {
            for id in VOICE_EMOTE_IDS {
                assert!(is_voice_emote(*id), "{id}");
            }
            for id in EMOJI_EMOTE_IDS {
                assert!(!is_voice_emote(*id), "{id}");
            }
            let voices: Vec<EmoteId> = EMOTE_IDS
                .iter()
                .copied()
                .filter(|id| is_voice_emote(*id))
                .collect();
            assert_eq!(voices, VOICE_EMOTE_IDS.to_vec());
        }
    }

    mod r641_the_launch_portrait_roster_a_deck_may_carry {
        use super::*;

        #[test]
        fn r641_holds_exactly_the_six_issue_portraits_each_with_a_card_name_and_a_flavour() {
            assert_eq!(
                portrait_ids(PORTRAIT_IDS),
                vec!["vanilla", "gary", "timmy", "dfender", "felinors", "shredder"]
            );
            let keys: Vec<PortraitId> = PORTRAITS.iter().map(|(id, _)| *id).collect();
            assert_eq!(keys, PORTRAIT_IDS.to_vec());
            for id in PORTRAIT_IDS {
                let entry = portrait_entry(*id);
                assert!(!entry.card_name.is_empty(), "{id}");
                assert!(!entry.flavour.is_empty(), "{id}");
            }
        }

        #[test]
        fn r641_takes_vanilla_as_the_default_and_reads_null_undefined_and_unknown_ids_as_it() {
            assert_eq!(DEFAULT_PORTRAIT.as_str(), "vanilla");
            assert_eq!(PORTRAIT_IDS[0], DEFAULT_PORTRAIT);
            for id in PORTRAIT_IDS {
                assert!(is_portrait_id(&json!(id.as_str())), "{id}");
                assert_eq!(portrait_or_default(Some(id.as_str())), *id, "{id}");
            }
            assert_eq!(portrait_or_default(None), DEFAULT_PORTRAIT);
            assert_eq!(portrait_or_default(Some("bogus")), DEFAULT_PORTRAIT);
            assert_eq!(portrait_or_default(Some("")), DEFAULT_PORTRAIT);
        }

        #[test]
        fn r641_knows_the_six_and_nothing_else_no_other_string_nothing_that_is_not_a_string() {
            for bad in ["", "Vanilla", "vanilla ", "gary1", "portrait", "null"] {
                assert!(!is_portrait_id(&json!(bad)), "{bad:?}");
            }
            for not_string in [json!(null), json!(0), json!(true), json!({}), json!([])] {
                assert!(!is_portrait_id(&not_string));
            }
        }
    }

    mod r642_a_match_s_portrait_picks_land_on_the_roster {
        use super::*;

        #[test]
        fn r642_pick_portrait_maps_the_bounds_of_rng_onto_the_roster_s_ends() {
            let last = *PORTRAIT_IDS.last().expect("a roster");
            assert_eq!(pick_portrait(|| 0.0).as_str(), "vanilla");
            assert_eq!(pick_portrait(|| 0.999_999), last);
            // A draw of exactly 1, which Math.random never returns but a foreign rng could, clamps on-roster.
            assert_eq!(pick_portrait(|| 1.0), last);
        }

        #[test]
        fn r642_pick_portrait_deals_every_roster_member_over_a_sweep_of_draws_and_nothing_else() {
            let mut dealt: IndexSet<PortraitId> = IndexSet::new();
            for i in 0..1000 {
                let draw = f64::from(i) / 1000.0;
                let pick = pick_portrait(|| draw);
                assert!(is_portrait_id(&json!(pick.as_str())), "draw {draw}");
                dealt.insert(pick);
            }
            assert_eq!(dealt.len(), PORTRAIT_IDS.len());
        }

        #[test]
        fn r642_pick_portrait_from_seed_is_deterministic_and_deals_each_seat_a_roster_id() {
            assert_eq!(
                pick_portrait_from_seed("match-42:portrait:p1"),
                pick_portrait_from_seed("match-42:portrait:p1")
            );
            let mut differ = 0;
            for i in 0..60 {
                let p1 = pick_portrait_from_seed(&format!("match-{i}:portrait:p1"));
                let p2 = pick_portrait_from_seed(&format!("match-{i}:portrait:p2"));
                assert!(is_portrait_id(&json!(p1.as_str())), "match-{i} p1");
                assert!(is_portrait_id(&json!(p2.as_str())), "match-{i} p2");
                if p1 != p2 {
                    differ += 1;
                }
            }
            // Dealt independently per seat (R642): the two draws disagree on at least some matches.
            assert!(differ > 0);
        }

        #[test]
        fn r642_deals_the_two_all_random_seats_independently_p1_p2_agree_about_one_match_in_six() {
            // Fixed strings, no RNG: a hash with no avalanche dealt the seats the same portrait ~95% of
            // the time (the `:p1`/`:p2` suffixes differ in one character). Six hundred matches put the
            // ~1/6 agreement at 100 ± 27 (3σ), so [50, 200] proves independence and stays far from both
            // the old ~570 and a degenerate never-agreeing hash.
            let matches = 600;
            let mut same = 0;
            for i in 0..matches {
                if pick_portrait_from_seed(&format!("seed-{i}:portrait:p1"))
                    == pick_portrait_from_seed(&format!("seed-{i}:portrait:p2"))
                {
                    same += 1;
                }
            }
            assert!(same >= 50, "p1/p2 agree on {same} of {matches}");
            assert!(same <= 200, "p1/p2 agree on {same} of {matches}");
        }
    }

    mod r1341_the_hand_each_seat_is_dealt {
        use super::*;
        use crate::wire::catalog_types::PlayerId::{P1, P2};

        fn seeds(count: usize) -> impl Iterator<Item = String> {
            (0..count).map(|i| format!("{i:032x}"))
        }

        #[test]
        fn r1341_deals_eight_distinct_pool_ids_three_voice_lines_and_five_emoji() {
            assert_eq!(EMOTE_HAND_SIZE, 8);
            assert_eq!(EMOTE_HAND_VOICE, 3);
            for seed in seeds(300) {
                for seat in [P1, P2] {
                    let hand = deal_emote_hand(&seed, seat);
                    assert_eq!(hand.len(), EMOTE_HAND_SIZE, "{seed} {seat}");
                    let distinct: IndexSet<EmoteId> = hand.iter().copied().collect();
                    assert_eq!(distinct.len(), EMOTE_HAND_SIZE, "{seed} {seat}: {hand:?}");
                    let voices = hand.iter().filter(|id| is_voice_emote(**id)).count();
                    assert_eq!(voices, EMOTE_HAND_VOICE, "{seed} {seat}: {hand:?}");
                    assert!(hand.iter().all(|id| EMOTE_IDS.contains(id)));
                }
            }
        }

        #[test]
        fn r1341_lists_a_hand_in_the_pools_order_so_the_menu_reads_the_same_whatever_the_draw() {
            for seed in seeds(100) {
                let hand = deal_emote_hand(&seed, P1);
                let positions: Vec<usize> = hand
                    .iter()
                    .map(|id| EMOTE_IDS.iter().position(|pool| pool == id).unwrap_or(usize::MAX))
                    .collect();
                assert!(
                    positions.windows(2).all(|pair| pair[0] < pair[1]),
                    "{seed}: {hand:?}"
                );
            }
        }

        #[test]
        fn r1341_is_a_pure_function_of_the_seed_and_the_seat_the_same_on_every_call() {
            for seed in seeds(50) {
                for seat in [P1, P2] {
                    assert_eq!(deal_emote_hand(&seed, seat), deal_emote_hand(&seed, seat));
                }
            }
            // Pinned, so a reconnect, a restart and a replay see these eight for this match, and a
            // change to the deal is a change someone meant.
            assert_eq!(
                ids(&deal_emote_hand("seed-actor", P1)),
                PINNED_SEED_ACTOR_P1.to_vec()
            );
        }

        /// `deal_emote_hand("seed-actor", p1)`, as dealt when the deal was written (R1341).
        const PINNED_SEED_ACTOR_P1: [&str; EMOTE_HAND_SIZE] = [
            "greetings",
            "thanks",
            "threaten",
            "laugh",
            "wahWah",
            "wave",
            "thumbsUp",
            "party",
        ];

        #[test]
        fn r1341_draws_from_the_seats_own_stream_never_the_match_rng() {
            assert_eq!(emote_hand_stream("match-42", P1), "match-42:emotes:p1");
            assert_eq!(emote_hand_stream("match-42", P2), "match-42:emotes:p2");
            // The match rng is `Rng::new(seed, cursor)` on the bare seed; the hand's stream is its own
            // string, so dealing a hand reads no state and moves no cursor (there is none to move).
            assert_ne!(emote_hand_stream("match-42", P1), "match-42");
        }

        #[test]
        fn r1341_deals_the_two_seats_independently_they_almost_never_hold_the_same_eight() {
            // C(5,3) x C(19,5) = 116,280 hands, so two seats agreeing is ~1 in 10^5.
            let same = seeds(400)
                .filter(|seed| deal_emote_hand(seed, P1) == deal_emote_hand(seed, P2))
                .count();
            assert!(same <= 1, "p1 and p2 held the same hand in {same} of 400 matches");
        }

        #[test]
        fn r1341_deals_every_id_and_each_at_its_share_three_in_five_voice_lines_five_in_nineteen_emoji() {
            // 4000 hands. A voice line is in 3/5 of them (2400 ± 31 at 1σ), an emoji in 5/19
            // (1053 ± 28): the bounds are past 12σ, so they hold a uniform deal and nothing skewed.
            let mut counts: IndexMap<EmoteId, usize> = EMOTE_IDS.iter().map(|id| (*id, 0)).collect();
            for seed in seeds(2000) {
                for seat in [P1, P2] {
                    for id in deal_emote_hand(&seed, seat) {
                        *counts.entry(id).or_insert(0) += 1;
                    }
                }
            }
            for (id, count) in &counts {
                if is_voice_emote(*id) {
                    assert!((2000..=2800).contains(count), "{id}: {count}");
                } else {
                    assert!((850..=1250).contains(count), "{id}: {count}");
                }
            }
        }

        #[test]
        fn r1341_hand_holds_answers_for_exactly_the_dealt_eight() {
            let hand = deal_emote_hand("seed-holds", P2);
            for id in EMOTE_IDS {
                assert_eq!(hand_holds(&hand, *id), hand.contains(id), "{id}");
            }
            assert_eq!(
                EMOTE_IDS.iter().filter(|id| hand_holds(&hand, **id)).count(),
                EMOTE_HAND_SIZE
            );
        }
    }

    mod r643_the_one_rate_limit_both_ends_enforce {
        use super::*;

        #[test]
        fn r643_fixes_1_5_s_between_a_player_s_emotes_and_at_most_five_in_a_rolling_20_s() {
            assert_eq!(EMOTE_COOLDOWN_MS, 1500);
            assert_eq!(EMOTE_WINDOW_MS, 20_000);
            assert_eq!(EMOTE_WINDOW_MAX, 5);
        }

        #[test]
        fn r643_admits_the_first_emote_a_player_sends() {
            assert_eq!(
                emote_gate(&[], 1_000),
                EmoteGate {
                    ok: true,
                    retry_after_ms: None,
                    sent_at: vec![]
                }
            );
            assert_eq!(
                serde_json::to_value(emote_gate(&[], 1_000)).expect("json"),
                json!({ "ok": true, "sentAt": [] })
            );
        }

        #[test]
        fn r643_rejects_an_emote_inside_the_cooldown_with_the_time_left_to_wait() {
            assert_eq!(
                emote_gate(&[10_000], 10_500),
                EmoteGate {
                    ok: false,
                    retry_after_ms: Some(EMOTE_COOLDOWN_MS - 500),
                    sent_at: vec![10_000]
                },
            );
        }

        #[test]
        fn r643_admits_an_emote_exactly_at_the_cooldown_boundary() {
            assert_eq!(
                emote_gate(&[10_000], 10_000 + EMOTE_COOLDOWN_MS),
                EmoteGate {
                    ok: true,
                    retry_after_ms: None,
                    sent_at: vec![10_000]
                },
            );
        }

        #[test]
        fn r643_admits_five_spaced_emotes_inside_a_window_and_refuses_the_sixth() {
            let mut sent_at: Vec<i64> = Vec::new();
            // Two seconds apart: past the cooldown each time, all five inside the rolling window.
            for now in [0, 2_000, 4_000, 6_000, 8_000] {
                let gate = emote_gate(&sent_at, now);
                assert!(gate.ok, "emote at {now}");
                sent_at = gate.sent_at;
                sent_at.push(now);
            }
            assert_eq!(sent_at.len(), EMOTE_WINDOW_MAX);

            let sixth = emote_gate(&sent_at, 10_000);
            assert!(!sixth.ok);
            // The wait is the time until the window's oldest emote ages out.
            let retry = sixth.retry_after_ms.expect("a refusal carries the wait");
            assert_eq!(
                retry,
                EMOTE_WINDOW_MS - (10_000 - sent_at.first().copied().unwrap_or(0))
            );
            assert!(retry > 0);
            // A refused emote does not join the rolling list.
            assert_eq!(sixth.sent_at, sent_at);
        }

        #[test]
        fn r643_prunes_emotes_older_than_the_window_from_the_returned_rolling_list() {
            // At t = EMOTE_WINDOW_MS the t = 0 emote is exactly a window old: the boundary drops it.
            assert_eq!(
                emote_gate(&[0, 18_000], 20_000),
                EmoteGate {
                    ok: true,
                    retry_after_ms: None,
                    sent_at: vec![18_000]
                },
            );
        }

        #[test]
        fn r643_does_not_count_emotes_outside_the_window_toward_the_cap() {
            // Five recorded emotes, but the oldest has aged out: four in the window, the send is admitted.
            let gate = emote_gate(&[1_000, 16_000, 18_000, 20_000, 22_000], 24_000);
            assert_eq!(
                gate,
                EmoteGate {
                    ok: true,
                    retry_after_ms: None,
                    sent_at: vec![16_000, 18_000, 20_000, 22_000]
                }
            );
        }
    }
}
