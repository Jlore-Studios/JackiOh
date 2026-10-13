--!strict
local J = require("@jackioh")

local base = {
	-- §8 Conventions: any unit or hero, either side.
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit", "hero" } },
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.damage({ to = { of = "chosen" }, amount = ctx:param("damage") }),
		}
	end,
}

-- The same script: the Radiant face's 8 is its declared `damage`, which `param` reads off the running face.
return {
	base = base,
	radiant = "base",
}
