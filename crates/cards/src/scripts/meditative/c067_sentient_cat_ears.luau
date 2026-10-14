--!strict
local J = require("@jackioh")

-- M #30.1 Love Bomb, the token this card shuffles (§7).
local BOMB = "meditative-030-1"

-- End of turn: `bombs` fresh Love Bombs into your deck (R80's cap turning a full library away).
-- Equip (R1086) is the engine's, off the printed keyword. Both faces share the one script: only
-- the declared `bombs` differs (1, radiant 2).
local base = {
	endOfTurn = function(ctx: J.EffectContext)
		return { J.shuffle_into({ defId = BOMB, count = ctx:param("bombs") }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
