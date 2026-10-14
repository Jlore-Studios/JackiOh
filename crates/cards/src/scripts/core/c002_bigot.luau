--!strict
local J = require("@jackioh")

local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = {
				side = "enemy",
				of = { "unit" },
				notTags = { "Human" },
			},
		},
	},
	cry = function(ctx)
		return {
			J.destroy({ target = { of = "chosen" } }),
		}
	end,
}

local radiant = {
	cry = function(ctx)
		return {
			J.destroy_all({
				side = "enemy",
				rows = { "units" },
				notTags = { "Human" },
			}),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
