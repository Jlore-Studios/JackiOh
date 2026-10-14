--!strict
local J = require("@jackioh")

local targets = {
	{
		kind = "target",
		min = 1,
		max = 1,
		filter = {
			side = "any",
			of = { "backrow" },
		},
	},
}

local base = {
	targets = targets,
	cry = function(ctx: J.EffectContext)
		return {
			J.lock({ zone = { of = "chosen" } }),
			J.destroy({ target = { of = "chosen" } }),
		}
	end,
}

local radiant = {
	targets = targets,
	cry = function(ctx: J.EffectContext)
		return {
			J.lock({ zone = { of = "chosen" } }),
			J.steal({ target = { of = "chosen" } }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
