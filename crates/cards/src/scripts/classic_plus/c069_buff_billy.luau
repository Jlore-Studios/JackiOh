--!strict
local J = require("@jackioh")

-- The Radiant face's "2X times".
local RADIANT_UPGRADES_PER_X = 2

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.upgrade({ target = { of = "self" }, times = ctx:x() }),
		}
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.upgrade({ target = { of = "self" }, times = RADIANT_UPGRADES_PER_X * ctx:x() }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
