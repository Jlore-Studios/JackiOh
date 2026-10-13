--!strict
local J = require("@jackioh")

local targets = {
	{
		kind = "target",
		min = 1,
		max = 1,
		filter = {
			side = "any",
			of = { "unit", "backrow" },
		},
	},
}

local base = {
	targets = targets,
	cry = function(ctx: J.EffectContext)
		return {
			J.exile({ target = { of = "chosen" } }),
			J.exile_random_from_library({ count = 1, player = "enemy" }),
		}
	end,
}

local radiant = {
	targets = targets,
	cry = function(ctx: J.EffectContext)
		return {
			J.exile_adjacent_to({ target = { of = "chosen" } }),
			J.exile({ target = { of = "chosen" } }),
			J.exile_random_from_library({ count = 1, player = "enemy" }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
