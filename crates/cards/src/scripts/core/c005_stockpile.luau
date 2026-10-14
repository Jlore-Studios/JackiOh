--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		local hero: J.TargetSpec = { of = "selfHero" }
		return {
			J.draw({ count = ctx:param("draw") }),
			J.heal({ target = hero, amount = ctx:param("heal") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
