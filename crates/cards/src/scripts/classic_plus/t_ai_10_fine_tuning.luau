--!strict
local J = require("@jackioh")

-- AI cards declare no numbers (B8): one card, two on the Radiant face.
local CARDS = 1
local RADIANT_CARDS = 2

local base = {
	endOfTurn = function(ctx: J.EffectContext)
		return {
			J.upgrade({ scope = { zones = { "hand" } }, random = CARDS }),
		}
	end,
}

local radiant = {
	endOfTurn = function(ctx: J.EffectContext)
		return {
			J.upgrade({ scope = { zones = { "hand" } }, random = RADIANT_CARDS }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
