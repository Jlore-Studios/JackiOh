--!strict
local J = require("@jackioh")

-- Whirlwind (C+ #21), which the base face casts round after round.
local WHIRLWIND = "classicplus-021"

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.cast_rounds_until_death({ def = WHIRLWIND, rounds = ctx:param("rounds") }),
		}
	end,
}

-- The Radiant face's hit of each round, "Deal 1 damage", is the declared number `damage` (R386);
-- the base face's rounds cast Whirlwind, whose own 1 is Whirlwind's number.
local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.damage_rounds_until_death({
				amount = ctx:param("damage"),
				rounds = ctx:param("rounds"),
				side = "enemy",
			}),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
