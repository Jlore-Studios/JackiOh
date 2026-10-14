--!strict
local J = require("@jackioh")

-- §7's shared Felinor Token.
local FELINOR_TOKEN = "core-t-felinor"

return {
	base = {
		cry = function(ctx: J.EffectContext)
			return {
				J.fill_board({ defId = FELINOR_TOKEN }),
			}
		end,
	},
	radiant = {
		cry = function(ctx: J.EffectContext)
			-- The +2/+2 is the declared number `buff` (R386).
			local buff = ctx:param("buff")
			return {
				J.fill_board({ defId = FELINOR_TOKEN }),
				-- "Then your units get +2/+2": after the fill, so the new tokens are included.
				J.buff_all_units({ side = "self", attack = buff, health = buff }),
			}
		end,
	},
}
