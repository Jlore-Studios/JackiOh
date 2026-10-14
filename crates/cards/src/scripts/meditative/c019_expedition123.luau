--!strict
local J = require("@jackioh")

-- M #19.1 Temporal Rift, made by this card.
local RIFT_ID = "meditative-019-1"

-- ME-TURN's lost refreshes (R844), then the Rift into hand. The Rift's face is the running face;
-- everything else is the same script.
local base = {
	staticFlags = { quickdraw = true },
	cry = function(ctx: J.EffectContext)
		return {
			J.lose_refreshes({ turns = ctx:param("turns") }),
			J.add_to_hand({ defId = RIFT_ID, radiant = ctx:radiant() }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
