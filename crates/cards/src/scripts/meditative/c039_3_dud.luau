--!strict
local J = require("@jackioh")

-- §8 Conventions: any unit or hero, either side. The same script: the Radiant face's 5 is its
-- declared `damage`, which `param` reads off the running face.
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
			J.damage({ to = { of = "chosen" }, amount = ctx:param("damage") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
