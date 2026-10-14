--!strict
local J = require("@jackioh")

local targets = {
	{
		kind = "target",
		min = 1,
		max = 1,
		filter = {
			side = "any",
			of = { "unit" },
		},
	},
}

local base = {
	targets = targets,
	cry = function(ctx: J.EffectContext)
		return {
			J.destroy({ target = { of = "chosen" } }),
		}
	end,
}

local radiant = {
	targets = targets,
	cry = function(ctx: J.EffectContext)
		return {
			J.destroy({ target = { of = "chosen" } }),
			J.destroy_adjacent_to({ target = { of = "chosen" } }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
