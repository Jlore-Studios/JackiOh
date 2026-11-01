//! R767000: the shipped catalog versions this build carries, so a replay folds under the version its
//! game was played on. Every shipped version but the newest, which is the build's own: its
//! definitions rebuilt from `build.rs`'s `history.json` (the snapshot's entries as they differ from
//! `catalog.json`'s), in catalog order, and its scripts the build's own but where one is kept in
//! `KEPT_SCRIPTS`. Each carries the win-rate table it shipped with (R1266000): its
//! `patches/win_rates/<version>.json`, or the empty table when it has none (R1329000, `table_of`),
//! and the BUSTED and complicated lists of its `patches/card_metrics/<version>.json` when it has one
//! (R2173000, `lists_of`).
//! The browser's module carries only the newest of them (`build.rs`'s `BROWSER_CARRIED_FROM`).
//!
//! A version's catalog is built the first time a replay asks for it and kept for the process: one of
//! the catalog statics a pure crate may hold (CLAUDE.md rule 4), never game state.

use std::sync::{LazyLock, OnceLock};

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::{CardDef, CardDefs, CardLists, CardScripts, ShippedCatalog, WinRateTable};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::scripts::classic::{
    c008_pickle, c009_income_tax, c013_boots_on_the_ground, c014_shadowstep, c020_the_power_to_punish,
    c023_devils_pact, c024_book_of_knowledge, c027_pestilent_slime, c032_felinor_feelings, c035_prep,
    c039_outbreak, c047_recurring_felinor, c052_final_gambit, c064_malzahars_recycler, c088_siphon_squad,
    c090_in_too_deep,
};
use crate::scripts::classic_plus::{
    c007_the_house, c011_anime_armor, c019_1_top_loser, c023_dropshipping, c030_felinor_fuser,
    c033_ivory_tower, c040_appropriations, c042_kys_test, c043_ai_slop, c048_jlockeeds_lobbyist,
    c051_jlockeeds_j15_fighter, c053_book_of_tokens, c062_kys_papaya, c071_book_of_buff,
    c073_1_classic_golem, c078_claudes_datacenter,
};
use crate::scripts::core::{
    c021_hinder, c023_reoccurring_dream, c029_giga_glowy_jelly_bean, c032_prem_panther, c042_eugenics,
    c049_snom_bunny_mind_control, c050_k_pop_fanatic, c051_1_kys_empty_notebook, c054_straaza,
    c065_1_spikey_pillow, c069_call_to_arms, c073_anti_oneshot_armor, c075_infinite_reserves, c080_zao_gao,
    c081_radiant_saintess, c083_transmogulate, c084_going_long,
};
use crate::scripts::meditative::{
    c017_true_craft_a_card, c018_expedition12, c019_expedition123, c021_api_key_fishing,
    c030_fickle_e_kitten, c032_spiritually_zhongguo, c037_cn_in_a_bottle, c038_h1b_printer,
    c045_knowledge_breaker, c048_tranquility, c049_1_yilegpt_unleashed, c049_3_ai_girlfriend,
    c049_yilegpt_tamed, c053_prestige, c054_money_machine, c074_montana_giant, c080_aluneth, c085_playtester,
    c090_spell_basket, c093_1_growing_felinor_sr, c093_2_growing_felinor_sr_sr,
    c093_3_growing_felinor_super_senior, c093_growing_felinor, c094_shrinking_felinor, c095_1_cn_golem,
    c097_2_wishing_well, c097_8_prison, c102_catboy_maid_ssr,
};
use crate::scripts::meditative_plus::{
    c001_bobs_plan, c003_wondrous_wand, c009_magic_amplifier, c019_grubby_gloves,
};
use crate::scripts::stinkyscape::{
    c026_zolarketh, c034_drone_core_viii, c035_drone_core_ix, c036_drone_core_x, c050_beam_v3,
    c051_repeater_v3, c052_cannon_v3, c053_flak_v3, c054_beam_v4, c055_repeater_v4, c056_cannon_v4,
    c057_flak_v4, c058_beam_v5, c059_repeater_v5, c060_cannon_v5, c061_flak_v5, c074_paragon_x,
    c086_prototype_x2, c094_tmack_trav, c116_megabase, c143_book_launcher, c149_wildar,
    c163_swear_loyalities, c169_tiny_asteroid, c170_medium_asteroid, c171_large_asteroid, c172_huge_asteroid,
    c173_abandoned_spice_tower, t_narcor, t_red_narcor,
};
use crate::scripts::training_grounds::c036_training_dummy;
use crate::{CATALOG, REGISTRY, catalog_json};

/// R767000, R388: the scripts kept for shipped versions, `(ID, version, script)`: a patch that changes
/// what a card's script does, not only a number its definition holds, keeps the script it replaces
/// here, under the newest shipped version that ran it, as any `fn` that builds that card's scripts
/// (`docs/ADDING_CARDS.md` §3). A replay of a game on that version, or on one before it back to the
/// previous kept script, folds under the kept one; every other card runs the build's own. The first
/// kept are #1034's: Wishing Well and Prison rolled without their controller's Luck (R987) through
/// v0.3.7b, and #1039's: Montaña Giant read a full hand as `HAND_CAP` cards, not its player's hand cap
/// (R1143), through v0.3.7b. K-Pop Fanatic's plain steal is kept beside them: it marked its target
/// purple and left one it had no room for with the opponent through v0.3.7b, before it Allured (R1045000).
/// #1149's five are kept beside them, each as it ran through v0.3.7b: Snom Bunny Mind Control stole any
/// enemy permanent, not only a Unit; Siphon Squad's aura worked from the moment it was set, not once it
/// was revealed (R1149000); True Craft a Card went to the graveyard, not to exile; CN in a Bottle was
/// replaced on any arrival in a hand, not cast on draw (R1149001); Appropriations' Culture and the
/// Radiant's Military and Healthcare had their higher multiples. #1194's four ran their v0.3.7b scripts
/// through v0.3.7b too: Malzahar's Recycler discarded at the end of its controller's turn, Ivory Tower had
/// no Aura, Money Machine rolled its controller's card and either player could Tribute it, and Aluneth's
/// draws took no discount. #1279's three ran theirs through v0.3.16: Pickle's questions opened during your
/// turn, Income Tax's keep opened at once on either turn, and In Too Deep's reward G asked at once with
/// its quest opening beside the prompt (R1279000). #1563's ran theirs through v0.4.0f: YileGPT Tamed put
/// its Prime on the bottom of the deck, not shuffled in, GIGA Glowy Jelly Bean had no Float discount, and
/// Expedition12 and Expedition123 lost all mana for their turns (R844), Expedition12's Radiant face with
/// Echo 1, and Bob's Plan added Spa Day after Aestheticize the Game, with no Discover. #1704's nine ran
/// theirs through v0.4.0f too: KY's Papaya offered 3 cells on its base face, could end on "done" and
/// exiled (R831003); The House summoned at its Cry and each start of turn (R406); Felinor Feelings stole
/// any enemy permanent; Final Gambit redirected the hit to the enemy hero; Straaza's pool was (3) or (4)
/// Cost Units; Fickle E-Kitten asked at the start of the turn; CN in a Bottle's base card took no raise;
/// Appropriations had +2X Attack, 2X Books, 8X% and +2X Health [+4X, 5X, 20X%, +6X]; and In Too Deep's
/// quests 3, 5 and 6 asked 3 permanents, 12 damage and totals of 10. #1748's eleven ran theirs through
/// v0.4.4b with the numbers printed that they now declare (R1748003): Hinder's discard, Reoccurring
/// Dream's and Eugenics' chances, Call to Arms' recruits, Zao Gao's tokens, Outbreak's draws per
/// counter, the cost Claude's Datacenter, H1B Printer, Wondrous Wand and Grubby Gloves set, and CN in a
/// Bottle's raise and its Radiant face's cost. #1828's four ran theirs through v0.4.9: API Key Fishing's
/// Radiant face added a base copy (R1828000), AI Slop fused from every AI generated card (R1828001), KY's
/// Empty Notebook drew 1, or 2 on its Radiant face (R1828002), and Shrinking Felinor's Radiant face
/// summoned a Radiant body (R1828003). #1851's ran its through v0.4.11: Prestige could be played on
/// either face with nothing to De-Radiant (R1103, R1851003). #1803's Shadowstep ran through v0.4.12b
/// with its definition-remade hand cards carrying none of the flickered unit's stats, Armor or tuning.
/// #2020's Infinite Reserves ran its through v0.4.12c. #2179's seventeen ran theirs through v0.5.2:
/// Tranquility went to the graveyard; Knowledge Breaker Nerfed the decks too; Felinor Fuser's Discovers
/// had no cost limit; Transmogulate had one price and replaced the board, graveyard and exile too; Devil's
/// Pact's Radiant Book of Flame was Radiant; Playtester's controller chose the Book (R1161); Catboy Maid
/// added 1-3 (1-6); Magic Amplifier had no Cry; Prep drew nothing; Anime Armor guarded the hero alone;
/// Dropshipping gave Brittle {brittle}; The Power to Punish's damage did not pierce; Spiritually 中国 did one
/// of its three at random (R920); Radiant Saintess's Radiant Death radiated the hand; Book of Buff's base
/// face could take a hand card; Claude's Datacenter added at its controller's end of turn; and KY's Test
/// asked two-digit sums (R580) from v0.5.2's bank (R2179003). #2396's three ran theirs through v0.5.11:
/// Prem Panther drew and Boots on the Ground drew or Recruited only after an attack they survived, and CN
/// Golem shuffled nothing when it died in the combat that killed (R2396000). #2521's twenty-two ran
/// theirs through v0.5.11 too: Pestilent Slime had no Death; Going Long and Aluneth were Quickdraw;
/// Bob's Plan added Alteran Reach with no Star Chart; ZolarKeth summoned a Radiant University and ran
/// Universities without spending their uses; the Beams, Repeaters and Cannons could hit the enemy hero
/// for half, the Repeaters at Activate 4, 6 and 6; the Flaks dealt their declared damage, not a random
/// range; Paragon-X carried Immune to Spell Damage as a flag; Tmack Trav's kill lifted its summoning
/// sickness; Swear Loyalities exiled the other four Faction cards alone, from the graveyard too; Narcor
/// added a Rare card; and Red Narcor added a Legendary or Mythic card at a discount. #2657's
/// thirty-five ran theirs through v0.5.9e: the sixteen that printed "Can't be in Defense Position" had it
/// as the static flag, before the keyword Can't Defend (R2657000, `never_defense_kept`); GIGA Glowy Jelly
/// Bean, YileGPT Prime, Drone Core VIII to X and Prototype-X2 Floated in the deck too (R2657002), and
/// YileGPT Prime made permanents Radiant; Recurring Felinor answered a Field Trap's every firing
/// (R2657001); Classic Golem's second attack lifted its summoning sickness (R2657004); the Flaks hit the
/// enemy hero; Book Launcher had two Activates; Spiritually 中国's hand cost (0); Spell Basket's Spells were
/// not Temporary; Book of Knowledge drew 3 or 6; Anti-oneshot Armor's Radiant face gave no Armor; Wildar's
/// Radiant face gave the opponent nothing; Prep's base face drew the highest cost Spell; Book of Tokens
/// rolled 1 or 2 evenly; and ZolarKeth ran its Universities.
/// A card patch before them that changed a script's behavior kept none, and a game it alters is refused
/// by its final hash.
pub static KEPT_SCRIPTS: &[KeptScript] = &[
    (
        c097_2_wishing_well::ID,
        "v0.3.1",
        c097_2_wishing_well::script_v0_3_7b,
    ),
    (c097_8_prison::ID, "v0.3.1", c097_8_prison::script_v0_3_7b),
    (
        c074_montana_giant::ID,
        "v0.3.1",
        c074_montana_giant::script_v0_3_7b,
    ),
    (
        c050_k_pop_fanatic::ID,
        "v0.3.1",
        c050_k_pop_fanatic::script_v0_3_7b,
    ),
    (
        c049_snom_bunny_mind_control::ID,
        "v0.3.1",
        c049_snom_bunny_mind_control::script_v0_3_7b,
    ),
    (c088_siphon_squad::ID, "v0.3.1", c088_siphon_squad::script_v0_3_7b),
    (
        c017_true_craft_a_card::ID,
        "v0.3.1",
        c017_true_craft_a_card::script_v0_3_7b,
    ),
    (
        c037_cn_in_a_bottle::ID,
        "v0.3.1",
        c037_cn_in_a_bottle::script_v0_3_7b,
    ),
    (
        c040_appropriations::ID,
        "v0.3.1",
        c040_appropriations::script_v0_3_7b,
    ),
    (
        c064_malzahars_recycler::ID,
        "v0.3.1",
        c064_malzahars_recycler::script_v0_3_7b,
    ),
    (c033_ivory_tower::ID, "v0.3.1", c033_ivory_tower::script_v0_3_7b),
    (
        c054_money_machine::ID,
        "v0.3.1",
        c054_money_machine::script_v0_3_7b,
    ),
    (c080_aluneth::ID, "v0.3.1", c080_aluneth::script_v0_3_7b),
    (c008_pickle::ID, "v0.4.0b", c008_pickle::script_v0_3_16),
    (c009_income_tax::ID, "v0.4.0b", c009_income_tax::script_v0_3_16),
    (c090_in_too_deep::ID, "v0.4.0b", c090_in_too_deep::script_v0_3_16),
    (
        c049_yilegpt_tamed::ID,
        "v0.4.0f",
        c049_yilegpt_tamed::script_v0_4_0f,
    ),
    (
        c029_giga_glowy_jelly_bean::ID,
        "v0.4.0f",
        c029_giga_glowy_jelly_bean::script_v0_4_0f,
    ),
    (
        c018_expedition12::ID,
        "v0.4.0f",
        c018_expedition12::script_v0_4_0f,
    ),
    (
        c019_expedition123::ID,
        "v0.4.0f",
        c019_expedition123::script_v0_4_0f,
    ),
    (c001_bobs_plan::ID, "v0.4.0f", c001_bobs_plan::script_v0_4_0f),
    (c062_kys_papaya::ID, "v0.4.0f", c062_kys_papaya::script_v0_4_0f),
    (c007_the_house::ID, "v0.4.0f", c007_the_house::script_v0_4_0f),
    (
        c032_felinor_feelings::ID,
        "v0.4.0f",
        c032_felinor_feelings::script_v0_4_0f,
    ),
    (
        c052_final_gambit::ID,
        "v0.4.0f",
        c052_final_gambit::script_v0_4_0f,
    ),
    (c054_straaza::ID, "v0.4.0f", c054_straaza::script_v0_4_0f),
    (
        c030_fickle_e_kitten::ID,
        "v0.4.0f",
        c030_fickle_e_kitten::script_v0_4_0f,
    ),
    (
        c037_cn_in_a_bottle::ID,
        "v0.4.0f",
        c037_cn_in_a_bottle::script_v0_4_0f,
    ),
    (
        c040_appropriations::ID,
        "v0.4.0f",
        c040_appropriations::script_v0_4_0f,
    ),
    (c090_in_too_deep::ID, "v0.4.0f", c090_in_too_deep::script_v0_4_0f),
    (c021_hinder::ID, "v0.4.4b", c021_hinder::script_v0_4_4b),
    (
        c023_reoccurring_dream::ID,
        "v0.4.4b",
        c023_reoccurring_dream::script_v0_4_4b,
    ),
    (c042_eugenics::ID, "v0.4.4b", c042_eugenics::script_v0_4_4b),
    (
        c069_call_to_arms::ID,
        "v0.4.4b",
        c069_call_to_arms::script_v0_4_4b,
    ),
    (c080_zao_gao::ID, "v0.4.4b", c080_zao_gao::script_v0_4_4b),
    (c039_outbreak::ID, "v0.4.4b", c039_outbreak::script_v0_4_4b),
    (
        c078_claudes_datacenter::ID,
        "v0.4.4b",
        c078_claudes_datacenter::script_v0_4_4b,
    ),
    (
        c037_cn_in_a_bottle::ID,
        "v0.4.4b",
        c037_cn_in_a_bottle::script_v0_4_4b,
    ),
    (c038_h1b_printer::ID, "v0.4.4b", c038_h1b_printer::script_v0_4_4b),
    (
        c003_wondrous_wand::ID,
        "v0.4.4b",
        c003_wondrous_wand::script_v0_4_4b,
    ),
    (
        c019_grubby_gloves::ID,
        "v0.4.4b",
        c019_grubby_gloves::script_v0_4_4b,
    ),
    (
        c021_api_key_fishing::ID,
        "v0.4.9",
        c021_api_key_fishing::script_v0_4_9,
    ),
    (c043_ai_slop::ID, "v0.4.9", c043_ai_slop::script_v0_4_9),
    (
        c051_1_kys_empty_notebook::ID,
        "v0.4.9",
        c051_1_kys_empty_notebook::script_v0_4_9,
    ),
    (
        c094_shrinking_felinor::ID,
        "v0.4.9",
        c094_shrinking_felinor::script_v0_4_9,
    ),
    (c053_prestige::ID, "v0.4.11", c053_prestige::script_v0_4_11),
    (c014_shadowstep::ID, "v0.4.12b", c014_shadowstep::script_v0_4_12b),
    (
        c075_infinite_reserves::ID,
        "v0.4.12c",
        c075_infinite_reserves::script_v0_4_12c,
    ),
    (c048_tranquility::ID, "v0.5.1", c048_tranquility::script_v0_5_2),
    (
        c045_knowledge_breaker::ID,
        "v0.5.1",
        c045_knowledge_breaker::script_v0_5_2,
    ),
    (
        c030_felinor_fuser::ID,
        "v0.5.1",
        c030_felinor_fuser::script_v0_5_2,
    ),
    (
        c083_transmogulate::ID,
        "v0.5.1",
        c083_transmogulate::script_v0_5_2,
    ),
    (c023_devils_pact::ID, "v0.5.1", c023_devils_pact::script_v0_5_2),
    (c085_playtester::ID, "v0.5.1", c085_playtester::script_v0_5_2),
    (
        c102_catboy_maid_ssr::ID,
        "v0.5.1",
        c102_catboy_maid_ssr::script_v0_5_2,
    ),
    (
        c009_magic_amplifier::ID,
        "v0.5.1",
        c009_magic_amplifier::script_v0_5_2,
    ),
    (c035_prep::ID, "v0.5.1", c035_prep::script_v0_5_2),
    (c011_anime_armor::ID, "v0.5.1", c011_anime_armor::script_v0_5_2),
    (c023_dropshipping::ID, "v0.5.1", c023_dropshipping::script_v0_5_2),
    (
        c020_the_power_to_punish::ID,
        "v0.5.1",
        c020_the_power_to_punish::script_v0_5_2,
    ),
    (
        c032_spiritually_zhongguo::ID,
        "v0.5.1",
        c032_spiritually_zhongguo::script_v0_5_2,
    ),
    (
        c081_radiant_saintess::ID,
        "v0.5.1",
        c081_radiant_saintess::script_v0_5_2,
    ),
    (c071_book_of_buff::ID, "v0.5.1", c071_book_of_buff::script_v0_5_2),
    (
        c078_claudes_datacenter::ID,
        "v0.5.1",
        c078_claudes_datacenter::script_v0_5_2,
    ),
    (c042_kys_test::ID, "v0.5.1", c042_kys_test::script_v0_5_2),
    (
        c049_3_ai_girlfriend::ID,
        "v0.5.7",
        c049_3_ai_girlfriend::script_v0_5_7,
    ),
    (c032_prem_panther::ID, "v0.5.7", c032_prem_panther::script_v0_5_11),
    (
        c013_boots_on_the_ground::ID,
        "v0.5.7",
        c013_boots_on_the_ground::script_v0_5_11,
    ),
    (c095_1_cn_golem::ID, "v0.5.7", c095_1_cn_golem::script_v0_5_11),
    (
        c027_pestilent_slime::ID,
        "v0.5.7",
        c027_pestilent_slime::script_v0_5_11,
    ),
    (c084_going_long::ID, "v0.5.7", c084_going_long::script_v0_5_11),
    (c080_aluneth::ID, "v0.5.7", c080_aluneth::script_v0_5_11),
    (c001_bobs_plan::ID, "v0.5.7", c001_bobs_plan::script_v0_5_11),
    (c026_zolarketh::ID, "v0.5.7", c026_zolarketh::script_v0_5_11),
    (c050_beam_v3::ID, "v0.5.7", c050_beam_v3::script_v0_5_11),
    (c051_repeater_v3::ID, "v0.5.7", c051_repeater_v3::script_v0_5_11),
    (c052_cannon_v3::ID, "v0.5.7", c052_cannon_v3::script_v0_5_11),
    (c053_flak_v3::ID, "v0.5.7", c053_flak_v3::script_v0_5_11),
    (c054_beam_v4::ID, "v0.5.7", c054_beam_v4::script_v0_5_11),
    (c055_repeater_v4::ID, "v0.5.7", c055_repeater_v4::script_v0_5_11),
    (c056_cannon_v4::ID, "v0.5.7", c056_cannon_v4::script_v0_5_11),
    (c057_flak_v4::ID, "v0.5.7", c057_flak_v4::script_v0_5_11),
    (c058_beam_v5::ID, "v0.5.7", c058_beam_v5::script_v0_5_11),
    (c059_repeater_v5::ID, "v0.5.7", c059_repeater_v5::script_v0_5_11),
    (c060_cannon_v5::ID, "v0.5.7", c060_cannon_v5::script_v0_5_11),
    (c061_flak_v5::ID, "v0.5.7", c061_flak_v5::script_v0_5_11),
    (c074_paragon_x::ID, "v0.5.7", c074_paragon_x::script_v0_5_11),
    (c094_tmack_trav::ID, "v0.5.7", c094_tmack_trav::script_v0_5_11),
    (
        c163_swear_loyalities::ID,
        "v0.5.7",
        c163_swear_loyalities::script_v0_5_11,
    ),
    (t_narcor::ID, "v0.5.7", t_narcor::script_v0_5_11),
    (t_red_narcor::ID, "v0.5.7", t_red_narcor::script_v0_5_11),
    (
        c024_book_of_knowledge::ID,
        "v0.5.9e",
        c024_book_of_knowledge::script_v0_5_9e,
    ),
    (c035_prep::ID, "v0.5.9e", c035_prep::script_v0_5_9e),
    (
        c047_recurring_felinor::ID,
        "v0.5.9e",
        c047_recurring_felinor::script_v0_5_9e,
    ),
    (c019_1_top_loser::ID, "v0.5.9e", c019_1_top_loser::script_v0_5_9e),
    (
        c048_jlockeeds_lobbyist::ID,
        "v0.5.9e",
        c048_jlockeeds_lobbyist::script_v0_5_9e,
    ),
    (
        c051_jlockeeds_j15_fighter::ID,
        "v0.5.9e",
        c051_jlockeeds_j15_fighter::script_v0_5_9e,
    ),
    (
        c053_book_of_tokens::ID,
        "v0.5.9e",
        c053_book_of_tokens::script_v0_5_9e,
    ),
    (
        c073_1_classic_golem::ID,
        "v0.5.9e",
        c073_1_classic_golem::script_v0_5_9e,
    ),
    (
        c029_giga_glowy_jelly_bean::ID,
        "v0.5.9e",
        c029_giga_glowy_jelly_bean::script_v0_5_9e,
    ),
    (
        c065_1_spikey_pillow::ID,
        "v0.5.9e",
        c065_1_spikey_pillow::script_v0_5_9e,
    ),
    (
        c073_anti_oneshot_armor::ID,
        "v0.5.9e",
        c073_anti_oneshot_armor::script_v0_5_9e,
    ),
    (
        c032_spiritually_zhongguo::ID,
        "v0.5.9e",
        c032_spiritually_zhongguo::script_v0_5_9e,
    ),
    (
        c049_1_yilegpt_unleashed::ID,
        "v0.5.9e",
        c049_1_yilegpt_unleashed::script_v0_5_9e,
    ),
    (
        c090_spell_basket::ID,
        "v0.5.9e",
        c090_spell_basket::script_v0_5_9e,
    ),
    (
        c093_1_growing_felinor_sr::ID,
        "v0.5.9e",
        c093_1_growing_felinor_sr::script_v0_5_9e,
    ),
    (
        c093_2_growing_felinor_sr_sr::ID,
        "v0.5.9e",
        c093_2_growing_felinor_sr_sr::script_v0_5_9e,
    ),
    (
        c093_3_growing_felinor_super_senior::ID,
        "v0.5.9e",
        c093_3_growing_felinor_super_senior::script_v0_5_9e,
    ),
    (
        c093_growing_felinor::ID,
        "v0.5.9e",
        c093_growing_felinor::script_v0_5_9e,
    ),
    (c026_zolarketh::ID, "v0.5.9e", c026_zolarketh::script_v0_5_9e),
    (
        c034_drone_core_viii::ID,
        "v0.5.9e",
        c034_drone_core_viii::script_v0_5_9e,
    ),
    (
        c035_drone_core_ix::ID,
        "v0.5.9e",
        c035_drone_core_ix::script_v0_5_9e,
    ),
    (
        c036_drone_core_x::ID,
        "v0.5.9e",
        c036_drone_core_x::script_v0_5_9e,
    ),
    (c053_flak_v3::ID, "v0.5.9e", c053_flak_v3::script_v0_5_9e),
    (c057_flak_v4::ID, "v0.5.9e", c057_flak_v4::script_v0_5_9e),
    (c061_flak_v5::ID, "v0.5.9e", c061_flak_v5::script_v0_5_9e),
    (
        c086_prototype_x2::ID,
        "v0.5.9e",
        c086_prototype_x2::script_v0_5_9e,
    ),
    (c116_megabase::ID, "v0.5.9e", c116_megabase::script_v0_5_9e),
    (
        c143_book_launcher::ID,
        "v0.5.9e",
        c143_book_launcher::script_v0_5_9e,
    ),
    (c149_wildar::ID, "v0.5.9e", c149_wildar::script_v0_5_9e),
    (
        c169_tiny_asteroid::ID,
        "v0.5.9e",
        c169_tiny_asteroid::script_v0_5_9e,
    ),
    (
        c170_medium_asteroid::ID,
        "v0.5.9e",
        c170_medium_asteroid::script_v0_5_9e,
    ),
    (
        c171_large_asteroid::ID,
        "v0.5.9e",
        c171_large_asteroid::script_v0_5_9e,
    ),
    (
        c172_huge_asteroid::ID,
        "v0.5.9e",
        c172_huge_asteroid::script_v0_5_9e,
    ),
    (
        c173_abandoned_spice_tower::ID,
        "v0.5.9e",
        c173_abandoned_spice_tower::script_v0_5_9e,
    ),
    (
        c036_training_dummy::ID,
        "v0.5.9e",
        c036_training_dummy::script_v0_5_9e,
    ),
];

/// One entry of `KEPT_SCRIPTS`: the card's `ID`, the shipped version it is kept under, and the `fn`
/// that builds the scripts that version ran.
pub type KeptScript = (&'static str, &'static str, fn() -> CardScripts);

/// `history.json`: every shipped version in order, the definition patches, the win-rate tables, and
/// each carried version.
#[derive(Deserialize, Default)]
struct History {
    #[serde(default)]
    order: Vec<String>,
    #[serde(default)]
    patches: Vec<DefPatch>,
    /// R1329000: every table a version's `win_rates/<version>.json` holds, each once.
    #[serde(default, rename = "winRates")]
    win_rates: Vec<WinRateTable>,
    /// R2173000: every pair of lists a version's `card_metrics/<version>.json` holds, each once.
    #[serde(default, rename = "cardLists")]
    card_lists: Vec<ListsFile>,
    #[serde(default)]
    versions: Vec<VersionDelta>,
    /// R1371000: `patches/aliases.json`'s map, each old name to its new one; empty before the retcon.
    #[serde(default)]
    aliases: IndexMap<String, String>,
    /// R2532000: `patches/aliases.json`'s `marked`: for each older scheme a marked file can name, each
    /// of its names a later retcon renamed, to its name now.
    #[serde(default)]
    marked: IndexMap<String, IndexMap<String, String>>,
}

/// R2173000: a `card_metrics/<version>.json`: the BUSTED and complicated lists a version shipped with.
#[derive(Deserialize, Default)]
struct ListsFile {
    busted: Vec<String>,
    complicated: Vec<String>,
}

/// What turns a catalog entry back into a snapshot's: top-level keys set to the snapshot's values,
/// and keys the snapshot's entry did not have.
#[derive(Deserialize, Default)]
struct DefPatch {
    #[serde(default)]
    set: Map<String, Value>,
    #[serde(default)]
    unset: Vec<String>,
}

/// One carried version: the catalog ids its snapshot lacks, each id that differs, by patch, and its
/// win-rate table, by index, when it has a file.
#[derive(Deserialize, Default)]
struct VersionDelta {
    #[serde(default)]
    version: String,
    #[serde(default)]
    absent: IndexSet<String>,
    #[serde(default)]
    changed: IndexMap<String, usize>,
    #[serde(default, rename = "winRates")]
    win_rates: Option<usize>,
    /// R2173000: its lists, by index into `History::card_lists`, when it has a file.
    #[serde(default, rename = "cardLists")]
    card_lists: Option<usize>,
}

static HISTORY: LazyLock<History> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/history.json")))
        .unwrap_or_else(|error| panic!("crates/cards/build.rs's history.json: {error}"))
});

/// Each carried version's catalog, in `HISTORY.versions`' order, built on first use.
static BUILT: LazyLock<Vec<OnceLock<ShippedCatalog>>> =
    LazyLock::new(|| HISTORY.versions.iter().map(|_| OnceLock::new()).collect());

/// R767000: every shipped version this build carries, oldest first: all but the newest.
pub fn carried_versions() -> Vec<&'static str> {
    HISTORY
        .versions
        .iter()
        .filter(|delta| delta.version != crate::CATALOG_VERSION)
        .map(|delta| delta.version.as_str())
        .collect()
}

/// R767000: the catalog of a carried version, `None` for the build's own version or any it does not
/// carry. What `register_all` registers with the engine.
pub fn shipped_catalog(version: &str) -> Option<&'static ShippedCatalog> {
    if version == crate::CATALOG_VERSION {
        return None;
    }
    let at = HISTORY
        .versions
        .iter()
        .position(|delta| delta.version == version)?;
    Some(BUILT[at].get_or_init(|| build(&HISTORY.versions[at])))
}

/// R1371000: the name a version read from an unmarked file goes by now: its new name when the
/// version retcon renamed it (`patches/aliases.json`), else the name itself. Only a reader of an
/// unmarked file asks: a marked file, and everything the build writes, names versions the new way,
/// and an old name the retcon reused names another catalog there.
pub fn current_version_name(unmarked: &str) -> &str {
    version_alias(unmarked).unwrap_or(unmarked)
}

/// R1371000: the new name of a version the retcon renamed, `None` for any other name.
pub fn version_alias(old: &str) -> Option<&'static str> {
    HISTORY.aliases.get(old).map(String::as_str)
}

/// R2532000: the name a version from a file named by `scheme` goes by now (`VERSIONS_SCHEME` is
/// today's): an unmarked file's (scheme 1) through `version_alias`, an older marked file's through
/// `patches/aliases.json`'s `marked`, and today's as it is.
pub fn version_name_from(scheme: u32, name: &str) -> &str {
    version_alias_from(scheme, name).unwrap_or(name)
}

/// R2532000: the name now of a version a retcon renamed since `scheme`, `None` for any other name.
pub fn version_alias_from(scheme: u32, name: &str) -> Option<&'static str> {
    if scheme <= 1 {
        return version_alias(name);
    }
    HISTORY
        .marked
        .get(&scheme.to_string())?
        .get(name)
        .map(String::as_str)
}

/// R388: every shipped catalog version from `patches.json`, in release order.
pub fn shipped_versions() -> Vec<&'static str> {
    HISTORY.order.iter().map(String::as_str).collect()
}

/// R388: the catalog of any shipped version in `patches.json`, exact snapshot.
pub fn catalog_at(version: &str) -> Option<&'static ShippedCatalog> {
    let at = HISTORY
        .versions
        .iter()
        .position(|delta| delta.version == version)?;
    Some(BUILT[at].get_or_init(|| build(&HISTORY.versions[at])))
}

/// R388: the card definitions of a shipped catalog version: the carried snapshot for any version
/// in `patches.json`. `None` for any version `patches.json` does not hold.
pub fn catalog_defs_at(version: &str) -> Option<&'static CardDefs> {
    catalog_at(version).map(|catalog| catalog.defs())
}

/// A carried version's catalog: `CATALOG` in order without the ids it lacked, each changed id's raw
/// entry patched back and read as a card, and the scripts kept for it.
fn build(delta: &VersionDelta) -> ShippedCatalog {
    let raw: Map<String, Value> = serde_json::from_str(catalog_json())
        .unwrap_or_else(|error| panic!("crates/cards/catalog.json: {error}"));
    let mut defs = CardDefs::new();
    for (id, def) in CATALOG.iter() {
        if delta.absent.contains(id) {
            continue;
        }
        let def = match delta.changed.get(id) {
            None => def.clone(),
            Some(&at) => patched(&delta.version, id, raw.get(id), &HISTORY.patches[at]),
        };
        defs.insert(id.clone(), def);
    }
    let scripts = defs
        .keys()
        .filter_map(|id| {
            covering(&HISTORY.order, KEPT_SCRIPTS, &delta.version, id).map(|script| (id.clone(), script()))
        })
        .collect();
    let shipped =
        ShippedCatalog::new(&delta.version, defs, scripts).with_win_rates(table_of(&HISTORY, delta));
    match lists_of(&HISTORY, delta) {
        Some(lists) => shipped.with_card_lists(lists),
        None => shipped,
    }
}

/// `version`'s definition of `id`: the catalog's raw entry with `patch` applied.
fn patched(version: &str, id: &str, raw: Option<&Value>, patch: &DefPatch) -> CardDef {
    let mut entry = raw.and_then(Value::as_object).cloned().unwrap_or_default();
    for key in &patch.unset {
        entry.remove(key);
    }
    for (key, value) in &patch.set {
        entry.insert(key.clone(), value.clone());
    }
    serde_json::from_value(Value::Object(entry)).unwrap_or_else(|error| {
        panic!("crates/cards/patches/{version}.json: \"{id}\" does not read as a card: {error}")
    })
}

/// R767000: the script kept for `id` that `version` ran: among `id`'s kept scripts, the one kept at
/// the earliest version at or after `version` in `order`, since a script is kept under the last
/// version that ran it. `None` when none is, or when `order` does not hold `version`.
fn covering<'a, T>(order: &[String], kept: &'a [(&str, &str, T)], version: &str, id: &str) -> Option<&'a T> {
    let from = order.iter().position(|listed| listed == version)?;
    kept.iter()
        .filter(|(kept_id, _, _)| *kept_id == id)
        .filter_map(|(_, at, script)| Some((order.iter().position(|listed| listed == at)?, script)))
        .filter(|(at, _)| *at >= from)
        .min_by_key(|(at, _)| *at)
        .map(|(_, script)| script)
}

/// R1266000, R1329000: the win-rate table `delta`'s version shipped with: the one its
/// `win_rates/<version>.json` holds, or, with no file, the empty table every version before v0.3.15
/// shipped with.
fn table_of(history: &History, delta: &VersionDelta) -> WinRateTable {
    delta
        .win_rates
        .map_or_else(WinRateTable::default, |at| history.win_rates[at].clone())
}

/// R2173000: the BUSTED and complicated lists `delta`'s version shipped with: the ones its
/// `card_metrics/<version>.json` holds, or none with no file, as every version shipped before the
/// first card that reads them, whose folds read the build's.
fn lists_of(history: &'static History, delta: &VersionDelta) -> Option<CardLists> {
    let file = &history.card_lists[delta.card_lists?];
    let ids = |list: &'static Vec<String>| list.iter().map(String::as_str).collect();
    Some(CardLists {
        busted: ids(&file.busted),
        complicated: ids(&file.complicated),
    })
}

/// R767000: whether `id` has a script on `version`: one kept for it, or the build's own.
pub fn has_script_at(version: &str, id: &str) -> bool {
    covering(&HISTORY.order, KEPT_SCRIPTS, version, id).is_some()
        || REGISTRY.iter().any(|(registered, _)| *registered == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r767000_a_kept_script_covers_its_version_and_those_before_back_to_the_previous() {
        let order: Vec<String> = ["v1", "v2", "v3", "v4", "v5"].map(String::from).to_vec();
        let kept = [("a", "v2", 1), ("a", "v4", 2), ("b", "v1", 3)];
        let at = |version: &str, id: &str| covering(&order, &kept, version, id).copied();
        assert_eq!(at("v1", "a"), Some(1));
        assert_eq!(at("v2", "a"), Some(1));
        assert_eq!(at("v3", "a"), Some(2));
        assert_eq!(at("v4", "a"), Some(2));
        assert_eq!(at("v5", "a"), None, "after the last kept, the build's own");
        assert_eq!(at("v1", "b"), Some(3));
        assert_eq!(at("v2", "b"), None);
        assert_eq!(at("v1", "c"), None);
        assert_eq!(at("v0", "a"), None, "a version the order does not hold");
    }

    #[test]
    fn r1329000_a_version_carries_the_table_its_file_held_and_one_with_no_file_the_empty_table() {
        let history: History = serde_json::from_value(serde_json::json!({
            "order": ["v1", "v2", "v3"],
            "winRates": [{
                "patch": "v2",
                "source": "provisional",
                "cards": [{ "id": "core-001", "wins": 30, "games": 40 }],
            }],
            "versions": [
                { "version": "v1" },
                { "version": "v2", "winRates": 0 },
                { "version": "v3", "winRates": 0 },
            ],
        }))
        .expect("a history");
        let tables: Vec<WinRateTable> = history
            .versions
            .iter()
            .map(|delta| table_of(&history, delta))
            .collect();
        assert_eq!(tables[0], WinRateTable::default(), "no file, no rows");
        assert_eq!(tables[1].cards.len(), 1);
        assert_eq!(tables[2], tables[1], "two versions share the one table");

        crate::register_all();
        // Every version through v0.3.7b shipped before `patches ship` kept a table: no rows.
        for version in ["v0.1.0", "v0.3.1"] {
            let carried = catalog_at(version).expect("in patches.json").win_rates();
            assert!(carried.cards.is_empty(), "{version}: {carried:?}");
        }
    }

    #[test]
    fn r2173000_a_version_carries_the_lists_its_file_held_and_one_with_no_file_none() {
        let history: &'static History = Box::leak(Box::new(
            serde_json::from_value(serde_json::json!({
                "order": ["v1", "v2", "v3"],
                "cardLists": [{ "busted": ["core-009", "core-001"], "complicated": ["core-001"] }],
                "versions": [
                    { "version": "v1" },
                    { "version": "v2", "cardLists": 0 },
                    { "version": "v3", "cardLists": 0 },
                ],
            }))
            .expect("a history"),
        ));
        let lists: Vec<Option<CardLists>> = history
            .versions
            .iter()
            .map(|delta| lists_of(history, delta))
            .collect();
        assert_eq!(lists[0], None, "no file, no lists: its folds read the build's");
        let kept = lists[1].clone().expect("v2 kept its lists");
        assert_eq!(kept.busted, vec!["core-009", "core-001"]);
        assert_eq!(kept.complicated, vec!["core-001"]);
        assert_eq!(lists[2], lists[1], "two versions share the one file");
        // `with_card_lists` sorts them for the engine's binary search.
        let shipped = ShippedCatalog::new("v2", CardDefs::new(), IndexMap::new()).with_card_lists(kept);
        assert_eq!(
            shipped.card_lists().map(|lists| lists.busted.clone()),
            Some(vec!["core-001", "core-009"])
        );

        crate::register_all();
        // v0.1.0 and v0.3.1 shipped before any card read the lists: none kept.
        for version in ["v0.1.0", "v0.3.1"] {
            assert!(
                catalog_at(version)
                    .expect("in patches.json")
                    .card_lists()
                    .is_none(),
                "{version}"
            );
        }
    }

    #[test]
    fn r767000_the_build_carries_every_shipped_version_but_its_own() {
        let patches: Value =
            serde_json::from_str(include_str!("../patches/patches.json")).expect("patches.json is JSON");
        let shipped: Vec<&str> = patches
            .as_array()
            .expect("patches.json is a list")
            .iter()
            .filter_map(|patch| patch.get("version").and_then(Value::as_str))
            .collect();
        let (own, older) = shipped.split_last().expect("a shipped patch");
        assert_eq!(*own, crate::CATALOG_VERSION);
        assert_eq!(carried_versions(), older.to_vec());
        assert!(shipped_catalog(own).is_none());
        assert!(shipped_catalog("v0.0.0-unknown").is_none());

        // The oldest is rebuilt, entry for entry, as its snapshot holds it.
        let oldest = shipped_catalog("v0.1.0").expect("v0.1.0 is carried");
        assert_eq!(oldest.version(), "v0.1.0");
        let snapshot: CardDefs =
            serde_json::from_str(include_str!("../patches/v0.1.0.json")).expect("v0.1.0 reads as cards");
        assert_eq!(
            serde_json::to_value(oldest.defs()).expect("serializes"),
            serde_json::to_value(&snapshot).expect("serializes")
        );
        assert!(
            std::ptr::eq(oldest, shipped_catalog("v0.1.0").expect("carried")),
            "built once"
        );
        assert!(older.iter().all(|version| has_script_at(version, "core-001")));
    }

    #[test]
    fn r767000_a_kept_script_runs_only_in_its_fold() {
        crate::register_all();
        // Big D-fender kept as Anti-oneshot Armor's script, as a patch that changed what it does
        // would keep its old one (R388).
        let kept = jackioh_engine::scripts::registered_entry("core-073")
            .cloned()
            .expect("Anti-oneshot Armor has a script");
        let shipped: &'static ShippedCatalog = Box::leak(Box::new(ShippedCatalog::new(
            "v-test",
            CATALOG.clone(),
            IndexMap::from([("core-001".to_string(), kept)]),
        )));
        let anti_oneshot = || {
            jackioh_engine::scripts::registered_entry("core-001")
                .and_then(|entry| entry.base.static_flags.as_ref())
                .and_then(|flags| flags.anti_oneshot)
                == Some(true)
        };
        assert!(!anti_oneshot());
        assert!(jackioh_engine::with_shipped(Some(shipped), || {
            assert_eq!(jackioh_engine::catalog::catalog_version(), "v-test");
            anti_oneshot()
        }));
        assert!(!anti_oneshot());
        assert_eq!(jackioh_engine::catalog::catalog_version(), crate::CATALOG_VERSION);
    }

    #[test]
    fn r767000_a_v0_3_7b_fold_runs_the_scripts_and_prices_1194_kept() {
        use jackioh_engine::testkit::*;

        crate::register_all();
        let v0_3_7b = catalog_at("v0.3.1").expect("v0.3.1 is carried");
        let menace = "core-019"; // (3) Unit
        let filler = "core-005";
        let board = || {
            crate::scenario(json!({
                "seed": "kept-1194",
                "p1": {
                    "hand": [menace, "classicplus-064", filler],
                    "backrow": [c080_aluneth::ID, c064_malzahars_recycler::ID, c033_ivory_tower::ID, c054_money_machine::ID],
                    "library": [menace, menace, menace, menace, menace, menace],
                },
                "p2": { "hand": [filler], "library": [filler, filler, filler] },
            }))
        };
        let held = |s: &Scenario, def_id: &str| {
            s.hand(PlayerId::P1)
                .into_iter()
                .find(|card| card.def_id == def_id)
                .expect("in hand")
        };
        let abilities = |s: &Scenario, def_id: &str| -> Vec<String> {
            let card = s.card(def_id).clone();
            jackioh_engine::scripts::script_of(s.state(), &card)
                .activations
                .iter()
                .map(|decl| decl.id.clone())
                .collect()
        };
        jackioh_engine::with_shipped(Some(v0_3_7b), || {
            let mut s = board();
            // Mulch Muncher's printed (10), Ivory Tower with no Aura, the Recycler and Money Machine's
            // old Activates.
            assert_eq!(
                effective_cost(s.state(), &held(&s, "classicplus-064"), Default::default()),
                10
            );
            assert!(!unit_has(s.state(), &held(&s, menace), KeywordKind::Flying));
            assert!(abilities(&s, c064_malzahars_recycler::ID).is_empty());
            assert_eq!(abilities(&s, c054_money_machine::ID), vec!["tribute".to_string()]);
            // The Recycler discards 2 at the end of the turn and Aluneth's 3 draws (and the
            // Recycler's 2) take no discount.
            s.end_turn();
            let discarded = s
                .last_events()
                .iter()
                .filter(|event| {
                    matches!(
                        event,
                        GameEvent::Discarded {
                            owner: PlayerId::P1,
                            ..
                        }
                    )
                })
                .count();
            assert_eq!(discarded, 2);
            let costs: Vec<i32> = s
                .hand(PlayerId::P1)
                .iter()
                .filter(|card| card.def_id == menace)
                .map(|card| effective_cost(s.state(), card, Default::default()))
                .collect();
            assert!(costs.iter().all(|cost| *cost == 3), "{costs:?}");
        });
        // The build's own: the new prices and scripts.
        let s = board();
        assert_eq!(
            effective_cost(s.state(), &held(&s, "classicplus-064"), Default::default()),
            5
        );
        assert!(unit_has(s.state(), &held(&s, menace), KeywordKind::Flying));
        assert_eq!(
            abilities(&s, c064_malzahars_recycler::ID),
            vec!["discard".to_string()]
        );
        assert_eq!(abilities(&s, c054_money_machine::ID), vec!["coin".to_string()]);
    }

    #[test]
    fn r388_catalog_defs_at_rebuilds_every_shipped_snapshot_exact() {
        let patches: Value =
            serde_json::from_str(include_str!("../patches/patches.json")).expect("patches.json is JSON");
        let shipped: Vec<&str> = patches
            .as_array()
            .expect("patches.json is a list")
            .iter()
            .filter_map(|patch| patch.get("version").and_then(Value::as_str))
            .collect();
        assert_eq!(shipped_versions(), shipped);

        // The oldest snapshot is rebuilt, entry for entry.
        let oldest = catalog_defs_at("v0.1.0").expect("v0.1.0 is carried");
        let snapshot_v0_1_0: CardDefs =
            serde_json::from_str(include_str!("../patches/v0.1.0.json")).expect("v0.1.0 reads as cards");
        assert_eq!(
            serde_json::to_value(oldest).expect("serializes"),
            serde_json::to_value(&snapshot_v0_1_0).expect("serializes")
        );

        // The newest is carried too, and v0.3.7b, the newest when this was written, is rebuilt as its
        // snapshot holds it, without the pending patches' changes `catalog.json` already has.
        assert!(
            catalog_defs_at(crate::CATALOG_VERSION).is_some(),
            "the newest is carried"
        );
        let v0_3_7b = catalog_defs_at("v0.3.1").expect("v0.3.1 is carried");
        let snapshot_v0_3_7b: CardDefs =
            serde_json::from_str(include_str!("../patches/v0.3.1.json")).expect("v0.3.1 reads as cards");
        assert_eq!(
            serde_json::to_value(v0_3_7b).expect("serializes"),
            serde_json::to_value(&snapshot_v0_3_7b).expect("serializes")
        );

        assert!(catalog_defs_at("v0.0.0-unknown").is_none());
        assert!(catalog_at("v0.0.0-unknown").is_none());
    }
}
