//! The ladder bots (SPEC §9.5, §9.12, R1453, R1454, R1451, R1452, R641000; #638, #3103): opponents the
//! queue seats in a ranked match or Conquest series when no person is available, decided in pure code
//! so a server of any kind runs them the way it runs a person, with no bot code of its own.
//!
//! A bot is a seat kind of the match host's machine (`host/duel.rs`, `host/bot.rs`): the machine asks
//! the host for the AI's move ([`decide`]) and paces it, shows its aim, emotes, concedes, offers and
//! answers draws, idles and stays for a rematch itself, as timers and actions. This module holds what
//! those are made of and what the queue and the pool decide around a bot:
//!
//!  - [`decide`]: the move the AI finds at the bot's strength; [`think`]: how long a person-paced bot
//!    waits before it; [`aim`]: the arrow it shows; [`signals`]: its habits at the end of a game
//!    (R639000); [`persona`]: its emotes (R645).
//!  - [`pick`]: its sealed picks in a Conquest series (R641000).
//!  - [`fallback`]: the empty-queue pairing of a ticket with a bot, [`pool`]: the pool's size, who grinds,
//!    a new bot's identity, decks and trio (R1454), [`names`]: its username.
//!  - [`cosmetics`]: the loadout it wears, drawn as a person's from a wardrobe it owns past the starters
//!    (R1834005).
//!
//! What exists only so that a bot passes for a person (R1452's pace, aim, emotes, concessions and
//! rematches; R1453's fallback time) is behind one setting, `BotSeat.hidden`, which the server turns on.
//! [`boundary`] is the JSON call the browser and Roblox make for the decisions around a bot.

pub mod aim;
pub mod boundary;
pub mod cosmetics;
pub mod decide;
pub mod fallback;
pub mod names;
pub mod persona;
pub mod pick;
pub mod pool;
pub mod signals;
pub mod think;

use jackioh_engine::Rng;

/// A whole number in `low..=high`, drawn from `rng`.
pub fn int_in(rng: &mut Rng, low: i64, high: i64) -> i64 {
    low + i64::from(rng.int(i32::try_from(high - low + 1).unwrap_or(1)))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use jackioh_engine::Rng;

    use super::int_in;

    /// R1454: a bot's drawn numbers (its active hours and account age, its name's year, digits and
    /// syllables) are whole numbers from the least to the most, both ends included and none outside.
    #[test]
    fn r1454_int_in_draws_every_whole_number_from_low_to_high_and_none_outside() {
        for (low, high) in [(3, 5), (-2, 2), (7, 7)] {
            let mut rng = Rng::new("r1454-int-in", 0);
            let drawn: BTreeSet<i64> = (0..200).map(|_| int_in(&mut rng, low, high)).collect();
            assert_eq!(drawn, (low..=high).collect::<BTreeSet<i64>>(), "{low}..={high}");
        }
    }
}
