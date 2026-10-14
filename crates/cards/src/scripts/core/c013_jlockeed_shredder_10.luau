--!strict
local J = require("@jackioh")

-- R51: every enemy unit and the enemy hero, one instance each, in one effect (R59). The amount is
-- the declared number `damage` (R386): 2, 5 on the Radiant face.
local base = {
	endOfTurn = function(ctx: J.EffectContext)
		return {
			J.damage_all({ side = "enemy", amount = ctx:param("damage"), heroes = true }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
