--!strict
local J = require("@jackioh")

-- Core #6 Mana Well, summoned into every empty backrow zone.
local WELL = "core-006"

-- MD-B2, R941: no natural mana for either player while this acts; MD-B3, R942: at its controller's
-- end of turn, a Mana Well into every empty, unlocked backrow zone of both players.
local base = {
	staticFlags = { noNaturalMana = true },
	endOfTurn = function(ctx: J.EffectContext)
		return {
			J.fill_board({ defId = WELL, row = "backrow", side = "any" }),
		}
	end,
}

-- "Yours are Radiant."
local radiant = {
	staticFlags = { noNaturalMana = true },
	endOfTurn = function(ctx: J.EffectContext)
		return {
			J.fill_board({ defId = WELL, row = "backrow", side = "any", radiantFor = "self" }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
