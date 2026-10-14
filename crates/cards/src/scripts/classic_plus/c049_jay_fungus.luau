--!strict
local J = require("@jackioh")

-- The Radiant face's 20 is its declared `discount`, which `param` reads.
local base = {
	endOfTurn = function(ctx: J.EffectContext)
		return {
			J.discount_random_in_hand({ amount = ctx:param("discount") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
