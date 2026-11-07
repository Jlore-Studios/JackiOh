//! The match host's decisions (#3101, #2956 Part B), pure: which clock runs and what an expiry becomes,
//! what a disconnect and a reconnect do, what a rematch offer meets, what an aim may show, and the order
//! every view and clock is sent in. `README.md` is the contract; this file is the map.
//!
//!  - [`duel`] is the 1v1 machine: [`DuelHost`], [`DuelEvent`], [`DuelOutput`] and [`step`]. [`duos`] is the
//!    co-op machine, its twin under the same event and output shapes (#3101's second part): [`DuosHost`],
//!    [`DuosHostEvent`], [`DuosHostOutput`] and [`step_duos`].
//!  - [`clock`] is its clock, [`relay`] its aim and choice-hover rules, [`snapshot`] the public
//!    bookkeeping both read, [`timer`] the timers it asks its host to keep, and [`bot`] what it decides of
//!    a seat the host plays.
//!  - [`rematch`] is the rematch book and [`again`] the co-op Play again's, which a host holds beside the
//!    machine because an offer outlives the host that played the match.
//!  - [`post_game`] is what a seat's open socket is told of its rematch or its series after the result
//!    (R3213000).
//!  - [`practice`] is the practice session: a game against the AI seat, with practice's own setup and rules
//!    (#3099).
//!  - [`boundary`] is the one JSON call the browser and Roblox make.

pub mod again;
pub mod bot;
pub mod boundary;
pub mod clock;
pub mod duel;
pub mod duos;
pub mod post_game;
pub mod practice;
pub mod relay;
pub mod rematch;
pub mod snapshot;
pub mod timer;
pub mod types;

pub use bot::{BotSeat, DecideJob};
pub use duel::{DuelHost, DuelSetup, step};
pub use duos::{DuosHost, DuosHostEvent, DuosHostOutput, DuosSetup, step_duos};
pub use types::*;
