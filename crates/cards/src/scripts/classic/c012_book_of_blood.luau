--!strict
local J = require("@jackioh")

local base = {
	-- "a Unit": units only, either side.
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
			J.damage({
				to = { of = "chosen" },
				amount = ctx:param("damage"),
				lifesteal = true,
			}),
		}
	end,
}

-- Same script: `param` reads the Radiant face's 10 off the running face.
return {
	base = base,
	radiant = "base",
}
