--!strict
local J = require("@jackioh")

-- M #28.1 Ancient Curse, the token this card shuffles (§7).
local CURSE = "meditative-028-1"

-- Both faces share the one script: only the declared `curses` differs (1, radiant 2).
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.shuffle_into({ defId = CURSE, count = ctx:param("curses"), player = "enemy" }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
