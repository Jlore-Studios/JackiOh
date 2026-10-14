--!strict
local J = require("@jackioh")

-- The Radiant face's printed Echo 1 (§6.1, R30): a repeat that runs the Cry again with fresh picks.
local ECHO = 1

-- `castRandom` (B5 E12, R452) is the whole card: {casts} random non-token Spells (not Field Spell or
-- Trap) of every set but Jogg's Box (R380, R387), each drawn as its cast begins, every choice made
-- from the match rng.
local function cry(ctx: J.EffectContext)
	return {
		J.cast_random({ query = { type = "Spell" }, count = ctx:param("casts") }),
	}
end

-- One function on both faces, so one adapter (#464 gap 14), as the Rust's `..base.clone()` shares
-- the base face's Cry.
return {
	base = { cry = cry },
	radiant = {
		staticFlags = { echo = ECHO },
		cry = cry,
	},
}
