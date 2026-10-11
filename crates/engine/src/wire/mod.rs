//! The wire types every layer shares (← `packages/shared/src`, SURFACE §1, §5): the action and
//! event unions, the view, the card definitions, and the small helpers that travel with them. The
//! client's TypeScript types are generated from these with `--features ts` (SURFACE §5.1).
//!
//! `packages/shared/src/index.ts` re-exported every module whole; so does this barrel. Written
//! once by part 1; `actions`, `catalog_types`, `events` and `view` are part 1's type freeze
//! (SURFACE §6.4), `aim`, `codes`, `emotes` and `stats` are part 5's.

pub mod actions;
pub mod aim;
pub mod catalog_types;
pub mod codes;
pub mod craft;
pub mod emotes;
pub mod events;
pub mod replays;
pub mod stats;
pub mod view;

pub use actions::*;
pub use aim::*;
pub use catalog_types::*;
pub use codes::*;
pub use craft::*;
pub use emotes::*;
pub use events::*;
pub use replays::*;
pub use stats::*;
pub use view::*;

/// Declares a TypeScript string-literal union as a Rust enum (SURFACE §4.3, §5.1): one unit variant
/// per literal, serialised as exactly that literal, with `as_str`, `Display`, `FromStr` and `ALL`
/// (the literals in declaration order). Unit-variant unions derive `Copy, Eq, Hash, PartialOrd,
/// Ord` (§5.1), and the client's TS type is generated from it under `--features ts`.
///
/// ```ignore
/// crate::wire::string_union! {
///     /// §3: a field row.
///     pub enum Row { Units = "units", Backrow = "backrow" }
/// }
/// ```
macro_rules! string_union {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident = $text:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(
            serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord,
        )]
        #[cfg_attr(
            feature = "ts",
            derive(ts_rs::TS),
            ts(export, export_to = "../../../apps/web/src/wire/generated/")
        )]
        $vis enum $name {
            $( $(#[$vmeta])* #[serde(rename = $text)] $variant ),+
        }

        impl $name {
            /// Every literal of the union, in the TS declaration order.
            pub const ALL: &'static [$name] = &[ $( $name::$variant ),+ ];

            /// The literal itself, as TS writes it (and as error messages interpolate it).
            pub fn as_str(self) -> &'static str {
                match self {
                    $( $name::$variant => $text ),+
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::str::FromStr for $name {
            type Err = String;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                match text {
                    $( $text => Ok($name::$variant), )+
                    other => Err(format!(concat!("not a ", stringify!($name), ": {:?}"), other)),
                }
            }
        }
    };
}

pub(crate) use string_union;
