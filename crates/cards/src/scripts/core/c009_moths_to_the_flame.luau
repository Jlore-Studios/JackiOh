--!strict
local J = require("@jackioh")

-- Both faces: every enemy unit, in enemy lane order, is forced to attack this card (R53).
local base = {
	startOfTurn = function(ctx: J.EffectContext)
		return {
			J.forced_attacks_on({ target = { of = "self" }, attackers = "enemy" }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
