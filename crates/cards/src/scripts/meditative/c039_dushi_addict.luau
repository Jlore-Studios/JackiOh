--!strict
local J = require("@jackioh")

-- The token this adds: Auspicious Rock (M #39.1), named, so no pool rule applies (R382).
local ROCK = "meditative-039-1"

return {
	base = {
		cry = function(ctx: J.EffectContext)
			return {
				J.add_to_hand({ defId = ROCK }),
			}
		end,
	},
	radiant = {
		cry = function(ctx: J.EffectContext)
			return {
				J.add_to_hand({ defId = ROCK, radiant = true }),
			}
		end,
	},
}
