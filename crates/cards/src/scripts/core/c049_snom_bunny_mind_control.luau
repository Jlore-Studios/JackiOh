--!strict
local J = require("@jackioh")

local targets = {
	{
		kind = "target",
		min = 1,
		max = 1,
		filter = {
			side = "enemy",
			of = { "unit", "backrow" },
		},
	},
}

local base = {
	targets = targets,
	cry = function(ctx: J.EffectContext)
		return {
			J.steal({ target = { of = "chosen" } }),
		}
	end,
}

local radiant = {
	targets = targets,
	cry = function(ctx: J.EffectContext)
		return {
			J.steal({ target = { of = "chosen" } }),
			J.set_radiant({ target = { of = "chosen" } }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
