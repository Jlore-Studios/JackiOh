--!strict
local J = require("@jackioh")

-- End of turn: `traps` random Traps or Field Traps (R380, R1420), each face-down into the leftmost
-- open backrow zone (R64); a full backrow summons nothing and draws no random number (R129). The
-- Radiant face's two picks may repeat (R60); `param` reads the face's count. `J.TRAP_TYPES` is
-- typed `{string}`, not `{CardType}`, so it is cast.
local base = {
	endOfTurn = function(ctx: J.EffectContext)
		local effects = {}
		for _ = 1, math.max(ctx:param("traps"), 0) do
			effects[#effects + 1] = J.summon_random({ query = { type = J.TRAP_TYPES :: any }, player = "self" })
		end
		return effects
	end,
}

return {
	base = base,
	radiant = "base",
}
