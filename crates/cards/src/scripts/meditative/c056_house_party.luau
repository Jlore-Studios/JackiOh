--!strict
local J = require("@jackioh")

-- §7's fill with core-003 Right-house defender (R64).
local DEFENDER = "core-003"

return {
	base = {
		cry = function(ctx: J.EffectContext)
			return {
				J.fill_board({ defId = DEFENDER }),
			}
		end,
	},
	radiant = {
		cry = function(ctx: J.EffectContext)
			return {
				J.fill_board({ defId = DEFENDER, radiant = true }),
			}
		end,
	},
}
