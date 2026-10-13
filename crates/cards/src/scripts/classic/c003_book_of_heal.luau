--!strict
local J = require("@jackioh")

local base = {
	-- R19: any unit or hero, either side.
	targets = {
		-- R656: a heal helps, so a random cast that targets enemies aims this at friends.
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit", "hero" } },
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

-- The same script: the Radiant face's 18 is its declared `heal`, which `param` reads off the running face.
return {
	base = base,
	radiant = "base",
}
