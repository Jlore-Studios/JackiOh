--!strict
local J = require("@jackioh")

local base = {
	targets = {
		{
			kind = "hand",
			min = 1,
			max = 1,
			filter = { of = { "hand" } },
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.set_radiant({ target = { of = "chosen", index = 0 } }),
		}
	end,
}

local radiant = {
	targets = {
		{
			kind = "hand",
			min = 2,
			max = 2,
			filter = { of = { "hand" } },
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.set_radiant({ target = { of = "chosen", index = 0 } }),
			J.set_radiant({ target = { of = "chosen", index = 1 } }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
