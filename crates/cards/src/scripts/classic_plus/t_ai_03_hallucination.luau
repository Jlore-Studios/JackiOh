--!strict
local J = require("@jackioh")

-- AI cards declare no numbers (B8), so the counts are this file's.
local COPIES = 1
local RADIANT_COPIES = 2
local BRITTLE = 2

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.add_library_copies({ of = "enemy", count = COPIES, brittle = BRITTLE }),
		}
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.add_library_copies({ of = "enemy", count = RADIANT_COPIES, brittle = BRITTLE }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
