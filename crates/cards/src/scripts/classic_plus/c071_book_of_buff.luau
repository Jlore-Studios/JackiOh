--!strict
local J = require("@jackioh")

-- The same script on both faces: the Radiant ten is the declared `times`.
local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit", "backrow", "hand" } },
			-- R656: an Upgrade helps, so a random cast that targets enemies aims this at friends.
			aim = "help",
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.upgrade({ target = { of = "chosen" }, times = ctx:param("times") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
