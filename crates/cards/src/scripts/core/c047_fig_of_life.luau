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
			aim = "help",
		},
	},
	cry = function(ctx: J.EffectContext)
		local target: J.TargetSpec = { of = "chosen" }
		return {
			J.heal({ target = target, amount = ctx:param("heal") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
