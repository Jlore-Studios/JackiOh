//! The dev dashboard (SPEC §14.10, §14.13; issue #3074): a separate app (`apps/admin`) that only a
//! dev account may use, behind Cloudflare Access, its own sign-in and, on a strict server, the
//! two-step code (R3074000). Its routes are `api::admin`'s and `api::dev`'s; this module holds what
//! they keep: [`store`], the audit of every dev and admin action (R3074000), the hand-set ratings
//! and their ledger (R3074001) and the news posts (R3074003), migration 0080's.

pub mod store;
