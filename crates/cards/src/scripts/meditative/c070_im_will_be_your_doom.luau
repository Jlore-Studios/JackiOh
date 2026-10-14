--!strict
local J = require("@jackioh")

-- The token: Ready… I'm (M #70.1), summoned under the other seat (R360, R1066).
local READY = "meditative-070-1"

return {
	base = {
		cry = function(ctx: J.EffectContext)
			return {
				J.summon({ defId = READY, player = "enemy" }),
			}
		end,
	},
	radiant = {
		cry = function(ctx: J.EffectContext)
			return {
				J.summon({ defId = READY, player = "enemy" }),
				J.summon({ defId = READY, radiant = true }),
			}
		end,
	},
}
