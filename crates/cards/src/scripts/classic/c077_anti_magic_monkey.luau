--!strict
local J = require("@jackioh")

-- A flat rung of the declared surcharge on every player's cards of the Spell type (B5 E15, R65).
local base = {
	costAura = function(ctx: J.HookArgs)
		local aura: J.CostAura = {
			types = { "Spell" },
			amount = ctx:param("surcharge"),
			whose = "all",
		}
		return { aura }
	end,
}

-- The same script: the Radiant face differs only in its declared surcharge (2) and its doubled stats.
return {
	base = base,
	radiant = "base",
}
