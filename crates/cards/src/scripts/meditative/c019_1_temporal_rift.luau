--!strict
local J = require("@jackioh")

-- ME-TURN's extra turn (R845–R847): `take_extra_turn` owes the caster one unless their once-a-game
-- Rift flag is set, and sets it. The Radiant face's riders are its declared numbers; the turn is
-- both faces' one verb.
local base = {
	cry = function(ctx: J.EffectContext)
		local effects = { J.take_extra_turn({ rift = true }) }
		if ctx:radiant() then
			effects[#effects + 1] = J.gain_mana({ amount = ctx:param("mana") })
			effects[#effects + 1] = J.next_turn_mana({ amount = ctx:param("nextMana") })
			effects[#effects + 1] = J.draw({ count = ctx:param("draw") })
		end
		return effects
	end,
}

return {
	base = base,
	radiant = "base",
}
