--!strict
local J = require("@jackioh")

-- The amount is the declared number `heal` (R386), read off the dying card's own face and tuning.
-- The Radiant face's 8 is its declared `heal`, and its Divine Shield is catalog data.
local base = {
	death = function(ctx: J.EffectContext)
		local hero: J.TargetSpec = { of = "selfHero" }
		return {
			J.heal({ target = hero, amount = ctx:param("heal") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
