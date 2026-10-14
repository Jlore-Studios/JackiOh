--!strict
local J = require("@jackioh")

-- The table the Rock rolls on (R960): Dud < Jade < Red Jade, worst to best.
local ROCK_TABLE = "auspiciousRock"

-- `lose_health` on your hero (R18), then one weighted roll (`add_rolled`, MD-C1). Both faces run
-- this one script: the Radiant face's Lucky 2 is its printed Lucky, which `add_rolled` reads off
-- the running face, and `life` is 2 on both.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.lose_health({ player = "self", amount = ctx:param("life") }),
			J.add_rolled({ table = ROCK_TABLE }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
