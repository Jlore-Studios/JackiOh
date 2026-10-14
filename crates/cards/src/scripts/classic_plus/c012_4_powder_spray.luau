--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.damage_all({ amount = ctx:param("damage"), side = "enemy", heroes = true }),
		}
	end,
}

-- The Radiant face is the same text at 6, a catalog value.
return {
	base = base,
	radiant = "base",
}
