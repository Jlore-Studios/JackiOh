//! Audit: what a player is shown (`view_for`, events) and what a player may send (`reduce`'s gates).
//! This file holds only tests that prove a reported finding, and the audit of this area reported none:
//! every differential and forged-action probe it ran came back clean. The probes themselves (twins of a
//! state that differ only in what one seat may not read, forged actions held to the listed outcomes,
//! per-card sweeps) are kept outside the repository, in the audit's scratchpad.
