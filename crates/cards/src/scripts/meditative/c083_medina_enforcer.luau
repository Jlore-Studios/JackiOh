--!strict
local J = require("@jackioh")

-- End of turn: M #82's effect, a Buff on every card in your hand (R386), `times` times — once on the
-- base face, three times on the Radiant one.
local base = {
	endOfTurn = function(ctx: J.EffectContext)
		return {
			J.upgrade({ scope = { side = "self", zones = { "hand" } }, times = ctx:param("times") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
