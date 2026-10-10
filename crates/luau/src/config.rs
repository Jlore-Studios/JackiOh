//! Every number the Luau host states (CLAUDE.md rule 9).

/// #442 L5: how many interrupts one hook call may take before it is stopped, naming its card. Luau
/// interrupts about once per loop iteration and three times per function call, so a card's hook,
/// which runs a few loops over the board, never comes near it, and a loop that never ends stops
/// within a few milliseconds.
pub const LUAU_HOOK_INTERRUPTS: u32 = 1_000_000;

/// #442 L6: how deep a value from Luau may nest before the walk refuses it. A table that holds
/// itself would otherwise never end; an effect's arguments nest a few levels at most.
pub const LUAU_VALUE_DEPTH: usize = 32;

/// #442 L6: how many values and keys, all told, one value from Luau may hold before the walk refuses
/// it. Under `LUAU_VALUE_DEPTH` a table can still hold another twice over at every level, which
/// would walk to billions of values; a hook's effects hold a few dozen.
pub const LUAU_VALUE_NODES: usize = 10_000;
