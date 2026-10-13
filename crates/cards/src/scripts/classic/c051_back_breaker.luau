--!strict
local J = require("@jackioh")

local base = {
	death = function(ctx: J.EffectContext)
		return {
			J.destroy_all({ side = "any", rows = { "backrow" } }),
		}
	end,
}

local radiant = {
	death = function(ctx: J.EffectContext)
		return {
			J.destroy_all({ side = "enemy", rows = { "backrow" } }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
