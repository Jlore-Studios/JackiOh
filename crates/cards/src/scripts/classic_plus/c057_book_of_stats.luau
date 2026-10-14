--!strict
local J = require("@jackioh")

-- The same script on both faces: the Radiant face's 10 is its declared `buff`, which `param` reads.
local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit" } },
			-- R656: a buff helps, so a random cast that targets enemies aims this at friends.
			aim = "help",
		},
	},
	cry = function(ctx: J.EffectContext)
		local amount = ctx:param("buff")
		return {
			J.buff({ target = { of = "chosen" }, attack = amount, health = amount }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
