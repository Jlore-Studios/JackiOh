--!strict
local J = require("@jackioh")

-- End of turn: `times` separate Buffs (Upgrade, R386) on the acting Unit in the next lane on its own
-- side (MD-D16, R1060); no neighbour, or an Immutable one, gets nothing (R129). The Radiant face
-- differs only in stats and its `times` value, both catalog data.
local base = {
	endOfTurn = function(ctx: J.EffectContext)
		local me = ctx:live_self()
		if me == nil then
			return {}
		end
		local at = ctx:slot_of(me.id)
		if at == nil then
			return {}
		end
		if at.row ~= "units" or at.lane >= J.config.UNIT_ZONES then
			return {}
		end
		local neighbour = ctx:card_at(at.player, at.lane + 1)
		if neighbour == nil then
			return {}
		end
		return { J.upgrade({ instanceId = neighbour.id, times = ctx:param("times") }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
