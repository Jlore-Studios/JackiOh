--!strict
local J = require("@jackioh")

local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit" } },
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.vanilla({}),
		}
	end,
}

-- R407: any permanent, a backrow card face-down or not included.
local radiant = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit", "backrow" } },
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.vanilla({}),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
