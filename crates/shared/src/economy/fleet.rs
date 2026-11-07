//! The Fleet track (SPEC §14.15, #3335), pure: a cosmetic track of its own, apart from the Stinkyscape set track
//! (§14.7), earned by playing Stinkyscape in the games the server records, each level paying one Stinkyscape
//! cosmetic gated as the Jlorious card back is (R3335002).
//!
//! [[R3335001]]: a recorded 1v1 game (ladder, unranked or Conquest; never practice, hotseat, co-op or a dungeon run)
//! pays its seat Fleet XP for each Stinkyscape card of its own deck it played, each Starship Piece it fused, each
//! Starship it launched, and a win with a deck at least `FLEET_TRACK_DECK_SHARE` Stinkyscape cards, each counted
//! once off the game's public log (`SeatSummary`). A day's games pay at most `FLEET_TRACK_XP_DAILY_MAX`: the game
//! that crosses it is paid the part that fits. Every `FLEET_TRACK_XP_PER_LEVEL` XP is a level, up to
//! `FLEET_TRACK_LEVELS`, and each level reached pays its reward once ([`crate::cosmetics::fleet_reward`]), with
//! the reason `track`. Every number is a [`crate::config`] constant (CLAUDE.md rule 9).

use jackioh_engine::wire::{CardDefs, SeatSummary, SetName};
use serde::{Deserialize, Serialize};

use crate::config::{
    FLEET_TRACK_DECK_SHARE, FLEET_TRACK_LEVELS, FLEET_TRACK_NEXT_REWARDS_SHOWN, FLEET_TRACK_XP_DAILY_MAX,
    FLEET_TRACK_XP_PER_LAUNCH, FLEET_TRACK_XP_PER_LEVEL, FLEET_TRACK_XP_PER_PIECE_FUSED,
    FLEET_TRACK_XP_PER_STINKYSCAPE_PLAY, FLEET_TRACK_XP_STINKYSCAPE_WIN,
};
use crate::cosmetics::fleet_reward;
use crate::economy::ledger::{Cause, LedgerGrant};
use crate::economy::rewards::GameMode;

/// [[R3335001]]: what one seat did in a recorded game that the Fleet track pays for: the Stinkyscape cards of its
/// own deck it played, the Starship Pieces it fused, the Starships it launched, and whether its deck held
/// `FLEET_TRACK_DECK_SHARE` Stinkyscape cards. A game of a mode that pays none, and one read before #3335, is
/// all zero.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase", default)]
pub struct FleetPlay {
    pub stinkyscape_played: i32,
    pub pieces_fused: i32,
    pub launched: i32,
    pub deck_share_met: bool,
}

/// [[R3335001]]: where an account's Fleet track stands, the store's read: its XP, the level it reached and the XP
/// its games paid it in the UTC day being paid, not counting this game's own. No row reads as all zero.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase", default)]
pub struct FleetTrack {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub xp: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub level: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub xp_today: i64,
}

/// [[R3335001]]: the Fleet track as `GET /api/tracks` spells it, so a client draws its bar without knowing the
/// curve: its level and XP, the XP a level spans and the XP to the next (neither on its last level), the XP paid
/// today, and the ids of the next levels' rewards, nearest first.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FleetTrackView {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub level: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub xp: i64,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub level_xp: Option<i64>,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub xp_to_next: Option<i64>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub xp_today: i64,
    pub next_rewards: Vec<String>,
}

/// [[R3335001]]: whether a game of `mode` pays Fleet XP: a 1v1 game the server ran and recorded. Practice, which the
/// browser runs, and co-op, whose log the server does not fold (§14.14), pay none.
pub fn counts_fleet(mode: GameMode) -> bool {
    matches!(mode, GameMode::Ladder | GameMode::Unranked | GameMode::Conquest)
}

/// [[R3335001]]: what `seat` did that the Fleet track pays for, off the server's fold of the log: its dealt plays
/// (`played`, never `created_played`) of Stinkyscape cards its own deck holds, no token, the way proficiency counts
/// them (R3013000); its fusions and launches; and whether its deck is `FLEET_TRACK_DECK_SHARE` Stinkyscape or more.
/// No fold reads as nothing done.
pub fn fleet_play(seat: Option<&SeatSummary>, defs: &CardDefs) -> FleetPlay {
    let Some(seat) = seat else {
        return FleetPlay::default();
    };
    let stinkyscape = |id: &String| {
        defs.get(id.as_str())
            .is_some_and(|def| def.set == SetName::Stinkyscape && !def.is_token())
    };
    let played = seat
        .played
        .iter()
        .filter(|id| seat.deck.contains(*id) && stinkyscape(id))
        .count();
    let in_deck = seat.deck.iter().filter(|id| stinkyscape(id)).count();
    FleetPlay {
        stinkyscape_played: i32::try_from(played).unwrap_or(i32::MAX),
        pieces_fused: seat.starship_pieces_fused.unwrap_or(0),
        launched: seat.starships_launched.unwrap_or(0),
        deck_share_met: !seat.deck.is_empty()
            && in_deck as f64 >= FLEET_TRACK_DECK_SHARE * seat.deck.len() as f64,
    }
}

/// [[R3335001]]: the Fleet XP one game pays before the daily cap: each source at its rate, and the win bonus for a
/// win (`won == Some(true)`) with the deck share met.
pub fn fleet_game_xp(play: &FleetPlay, won: Option<bool>) -> i64 {
    let won_bonus = if won == Some(true) && play.deck_share_met {
        FLEET_TRACK_XP_STINKYSCAPE_WIN
    } else {
        0
    };
    i64::from(play.stinkyscape_played) * i64::from(FLEET_TRACK_XP_PER_STINKYSCAPE_PLAY)
        + i64::from(play.pieces_fused) * i64::from(FLEET_TRACK_XP_PER_PIECE_FUSED)
        + i64::from(play.launched) * i64::from(FLEET_TRACK_XP_PER_LAUNCH)
        + i64::from(won_bonus)
}

/// [[R3335001]]: the level `xp` stands on, a level every `FLEET_TRACK_XP_PER_LEVEL`, from 0 to `FLEET_TRACK_LEVELS`.
pub fn fleet_level(xp: i64) -> i64 {
    (xp.max(0) / i64::from(FLEET_TRACK_XP_PER_LEVEL)).min(i64::from(FLEET_TRACK_LEVELS))
}

/// [[R3335001]]: `track` as `GET /api/tracks` answers it ([`FleetTrackView`]).
pub fn fleet_view(track: &FleetTrack) -> FleetTrackView {
    let per_level = i64::from(FLEET_TRACK_XP_PER_LEVEL);
    let last = i64::from(FLEET_TRACK_LEVELS);
    let level = track.level.clamp(0, last);
    let (level_xp, xp_to_next) = if level >= last {
        (None, None)
    } else {
        (Some(per_level), Some(((level + 1) * per_level - track.xp).max(0)))
    };
    let next_rewards = (level + 1..=last)
        .filter_map(|next| u32::try_from(next).ok().and_then(fleet_reward))
        .take(FLEET_TRACK_NEXT_REWARDS_SHOWN)
        .map(|cosmetic| cosmetic.id.clone())
        .collect();
    FleetTrackView {
        level,
        xp: track.xp,
        level_xp,
        xp_to_next,
        xp_today: track.xp_today,
        next_rewards,
    }
}

/// [[R3335001]]: what one recorded game pays the Fleet track: its XP held to what the day has left of
/// `FLEET_TRACK_XP_DAILY_MAX` ([`LedgerGrant::Fleet`], with the levels it reached), then each level reached in
/// `(track.level, new level]` its reward ([`LedgerGrant::FleetReward`]), every row naming `reference` (the match).
/// No XP left today, or none earned, writes nothing. XP past the last level still lands on the track.
pub fn pay_fleet(track: &FleetTrack, play: &FleetPlay, won: Option<bool>, reference: &str) -> Vec<LedgerGrant> {
    let left = (FLEET_TRACK_XP_DAILY_MAX - track.xp_today).max(0);
    let xp = fleet_game_xp(play, won).min(left);
    if xp <= 0 {
        return Vec::new();
    }
    let level = fleet_level(track.xp.saturating_add(xp)).max(track.level);
    let mut grants = vec![LedgerGrant::Fleet {
        xp,
        levels: level - track.level,
        cause: Cause::Game,
        reference: Some(reference.to_string()),
    }];
    for reached in track.level + 1..=level {
        if let Some(reward) = u32::try_from(reached).ok().and_then(fleet_reward) {
            grants.push(LedgerGrant::FleetReward {
                cosmetic_id: reward.id.clone(),
                reference: Some(reference.to_string()),
            });
        }
    }
    grants
}

#[cfg(test)]
mod tests {
    use super::*;

    use jackioh_engine::wire::CreatedUse;

    use crate::cosmetics::{CosmeticTrack, FLEET_CAPSTONE, registry};

    fn defs() -> &'static CardDefs {
        crate::cosmetics::shipped_defs()
    }

    /// A Stinkyscape card that is no token, and another set's, by the shipped catalog.
    fn ids() -> (String, String, String) {
        let stinky = defs()
            .iter()
            .find(|(_, def)| def.set == SetName::Stinkyscape && !def.is_token())
            .map(|(id, _)| id.clone())
            .unwrap_or_default();
        let token = defs()
            .iter()
            .find(|(_, def)| def.set == SetName::Stinkyscape && def.is_token())
            .map(|(id, _)| id.clone())
            .unwrap_or_default();
        let other = defs()
            .iter()
            .find(|(_, def)| def.set != SetName::Stinkyscape && !def.is_token())
            .map(|(id, _)| id.clone())
            .unwrap_or_default();
        (stinky, token, other)
    }

    fn levels_paid(grants: &[LedgerGrant]) -> Vec<String> {
        grants
            .iter()
            .filter_map(|grant| match grant {
                LedgerGrant::FleetReward { cosmetic_id, .. } => Some(cosmetic_id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r3335001_fleet_play_counts_deck_stinkyscape_plays_pieces_and_launches_once() {
        let (stinky, token, other) = ids();
        assert!(!stinky.is_empty() && !token.is_empty() && !other.is_empty());
        let seat = SeatSummary {
            deck: vec![stinky.clone(), stinky.clone(), other.clone(), token.clone()],
            // Two plays of the deck's Stinkyscape card count; the other set's card, the token and a card the deck
            // does not hold count nothing, and a Created card is never in `played`.
            played: vec![stinky.clone(), other.clone(), token.clone(), stinky.clone()],
            created_played: Some(vec![CreatedUse {
                card: stinky.clone(),
                turn: 3,
                radiant: false,
            }]),
            starship_pieces_fused: Some(3),
            starships_launched: Some(1),
            ..SeatSummary::default()
        };
        let play = fleet_play(Some(&seat), defs());
        assert_eq!(
            play,
            FleetPlay {
                stinkyscape_played: 2,
                pieces_fused: 3,
                launched: 1,
                // Two of four cards is the share, the token not counted Stinkyscape.
                deck_share_met: true,
            }
        );
        assert_eq!(
            fleet_game_xp(&play, Some(false)),
            2 * i64::from(FLEET_TRACK_XP_PER_STINKYSCAPE_PLAY)
                + 3 * i64::from(FLEET_TRACK_XP_PER_PIECE_FUSED)
                + i64::from(FLEET_TRACK_XP_PER_LAUNCH)
        );
        let off_deck = SeatSummary {
            deck: vec![other.clone()],
            played: vec![stinky.clone()],
            ..SeatSummary::default()
        };
        assert_eq!(fleet_play(Some(&off_deck), defs()), FleetPlay::default());
        assert_eq!(fleet_play(None, defs()), FleetPlay::default());
        // Only a recorded 1v1 game counts.
        assert!(counts_fleet(GameMode::Ladder) && counts_fleet(GameMode::Unranked) && counts_fleet(GameMode::Conquest));
        assert!(!counts_fleet(GameMode::Practice) && !counts_fleet(GameMode::Coop));
    }

    #[test]
    fn r3335001_the_win_bonus_needs_the_deck_share() {
        let (stinky, _, other) = ids();
        let half = SeatSummary {
            deck: vec![stinky.clone(), other.clone()],
            ..SeatSummary::default()
        };
        let less = SeatSummary {
            deck: vec![stinky.clone(), other.clone(), other.clone()],
            ..SeatSummary::default()
        };
        let met = fleet_play(Some(&half), defs());
        let short = fleet_play(Some(&less), defs());
        assert!(met.deck_share_met && !short.deck_share_met);
        assert_eq!(fleet_game_xp(&met, Some(true)), i64::from(FLEET_TRACK_XP_STINKYSCAPE_WIN));
        assert_eq!(fleet_game_xp(&met, Some(false)), 0);
        assert_eq!(fleet_game_xp(&met, None), 0);
        assert_eq!(fleet_game_xp(&short, Some(true)), 0);
        assert!(!fleet_play(Some(&SeatSummary::default()), defs()).deck_share_met);
    }

    #[test]
    fn r3335001_pay_fleet_holds_the_daily_cap() {
        let play = FleetPlay {
            launched: 4,
            ..FleetPlay::default()
        };
        let earned = fleet_game_xp(&play, None);
        let xp_of = |grants: &[LedgerGrant]| -> i64 {
            grants
                .iter()
                .map(|grant| match grant {
                    LedgerGrant::Fleet { xp, .. } => *xp,
                    _ => 0,
                })
                .sum()
        };
        let fresh = FleetTrack::default();
        assert_eq!(xp_of(&pay_fleet(&fresh, &play, None, "m")), earned);
        // The game that crosses the cap is paid the part that fits; the ones after it nothing.
        let near = FleetTrack {
            xp_today: FLEET_TRACK_XP_DAILY_MAX - 30,
            ..FleetTrack::default()
        };
        assert_eq!(xp_of(&pay_fleet(&near, &play, None, "m")), 30);
        let full = FleetTrack {
            xp_today: FLEET_TRACK_XP_DAILY_MAX,
            ..FleetTrack::default()
        };
        assert!(pay_fleet(&full, &play, None, "m").is_empty());
        // Nothing earned writes nothing.
        assert!(pay_fleet(&fresh, &FleetPlay::default(), Some(true), "m").is_empty());
    }

    #[test]
    fn r3335001_each_level_crossed_pays_its_reward_once() {
        let per_level = i64::from(FLEET_TRACK_XP_PER_LEVEL);
        let play = FleetPlay {
            launched: 1,
            ..FleetPlay::default()
        };
        let xp = fleet_game_xp(&play, None);
        // A track one launch short of level 2 reaches it and is paid level 2's reward alone.
        let short = FleetTrack {
            xp: 2 * per_level - xp,
            level: 1,
            xp_today: 0,
        };
        let grants = pay_fleet(&short, &play, None, "m");
        assert_eq!(
            grants.first(),
            Some(&LedgerGrant::Fleet {
                xp,
                levels: 1,
                cause: Cause::Game,
                reference: Some("m".to_owned()),
            })
        );
        let level_2 = fleet_reward(2).map(|cosmetic| cosmetic.id.clone());
        assert_eq!(levels_paid(&grants), level_2.into_iter().collect::<Vec<_>>());
        // Already on level 2 with the same XP: no level is paid again.
        let again = FleetTrack {
            xp: 2 * per_level,
            level: 2,
            xp_today: 0,
        };
        assert!(levels_paid(&pay_fleet(&again, &play, None, "m")).is_empty());
        // Past the last level the XP lands and pays nothing more.
        let top = FleetTrack {
            xp: i64::from(FLEET_TRACK_LEVELS) * per_level,
            level: i64::from(FLEET_TRACK_LEVELS),
            xp_today: 0,
        };
        let past = pay_fleet(&top, &play, None, "m");
        assert_eq!(past.len(), 1);
        assert!(matches!(past[0], LedgerGrant::Fleet { levels: 0, .. }));
        // Every level once, the capstone last.
        let paid: Vec<String> = (0..i64::from(FLEET_TRACK_LEVELS))
            .flat_map(|level| {
                let track = FleetTrack {
                    xp: (level + 1) * per_level - xp,
                    level,
                    xp_today: 0,
                };
                levels_paid(&pay_fleet(&track, &play, None, "m"))
            })
            .collect();
        let expected: Vec<String> = registry()
            .iter()
            .filter(|cosmetic| cosmetic.track == Some(CosmeticTrack::Fleet))
            .map(|cosmetic| cosmetic.id.clone())
            .collect();
        assert_eq!(paid, expected);
        assert_eq!(paid.last().map(String::as_str), Some(FLEET_CAPSTONE));
    }

    #[test]
    fn r3335001_the_view_names_the_next_rewards_and_ends_at_the_last_level() {
        let per_level = i64::from(FLEET_TRACK_XP_PER_LEVEL);
        let view = fleet_view(&FleetTrack {
            xp: per_level + 20,
            level: 1,
            xp_today: 20,
        });
        assert_eq!(view.level_xp, Some(per_level));
        assert_eq!(view.xp_to_next, Some(per_level - 20));
        assert_eq!(view.xp_today, 20);
        let next: Vec<String> = (2..2 + FLEET_TRACK_NEXT_REWARDS_SHOWN as u32)
            .filter_map(fleet_reward)
            .map(|cosmetic| cosmetic.id.clone())
            .collect();
        assert_eq!(view.next_rewards, next);
        let last = fleet_view(&FleetTrack {
            xp: i64::from(FLEET_TRACK_LEVELS) * per_level,
            level: i64::from(FLEET_TRACK_LEVELS),
            xp_today: 0,
        });
        assert_eq!((last.level_xp, last.xp_to_next), (None, None));
        assert!(last.next_rewards.is_empty());
        assert_eq!(fleet_view(&FleetTrack::default()).next_rewards.len(), FLEET_TRACK_NEXT_REWARDS_SHOWN);
    }
}
