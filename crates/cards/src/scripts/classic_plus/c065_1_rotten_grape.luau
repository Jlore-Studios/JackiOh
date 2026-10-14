--!strict
local J = require("@jackioh")

-- §8.7: the health each face costs its own hero, 5, and the Radiant face's smaller 1, its upgrade
-- (R275): the declared number `loss` (R386), less being better.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.lose_health({ player = "self", amount = ctx:param("loss") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
