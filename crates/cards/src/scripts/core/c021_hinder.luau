--!strict
local J = require("@jackioh")

local base = {
	staticFlags = { castOnDraw = true },
	cry = function(ctx: J.EffectContext)
		return {
			J.next_turn_mana({ amount = -ctx:param("mana"), player = "enemy" }),
			J.discard_random({ count = 1 }),
		}
	end,
}

local radiant = {
	staticFlags = { castOnDraw = true },
	cry = function(ctx: J.EffectContext)
		return {
			J.next_turn_mana({ amount = -ctx:param("mana"), player = "enemy" }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
