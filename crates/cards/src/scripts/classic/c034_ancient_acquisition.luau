--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.add_random_from_graveyard({ count = ctx:param("cards") }),
		}
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.add_random_from_graveyard({ count = ctx:param("cards"), exile = true }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
