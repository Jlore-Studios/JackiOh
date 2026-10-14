--!strict
local J = require("@jackioh")

-- The one clause both faces share: §6.3's Heal with `toFull` takes all damage off.
local base = {
	endOfTurn = function(ctx: J.EffectContext)
		local target: J.TargetSpec = { of = "self" }
		return {
			J.heal({ target = target, toFull = true }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
