--!strict
local J = require("@jackioh")

-- §6.3 "Tribute 1": one of your Units, as #66 The Rock's.
local TRIBUTE_COST = 1

-- The same script on both faces: the Radiant face's draw 3 is its declared `draw`, which `param`
-- reads off the running face; its stats are the catalog's.
local base = {
	staticFlags = { tribute = TRIBUTE_COST },
	cry = function(ctx: J.EffectContext)
		return {
			J.place_plague_tokens({ count = ctx:param("tokens") }),
			J.draw({ count = ctx:param("draw") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
