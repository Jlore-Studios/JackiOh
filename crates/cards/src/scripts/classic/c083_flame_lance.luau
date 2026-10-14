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
				trample = true,
			}),
		}
	end,
}

-- The same script: the Radiant face's 22 is its declared `damage`, which `param` reads off the face.
return {
	base = base,
	radiant = "base",
}
