--!strict
local J = require("@jackioh")

local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = {
				side = "any",
				of = { "unit", "hero" },
			},
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.damage({
				to = { of = "chosen" },
				amount = ctx:param("damage"),
				ignoreArmor = true,
			}),
			J.exile({ target = { of = "self" } }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
