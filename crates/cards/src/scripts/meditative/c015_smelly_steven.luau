--!strict
local J = require("@jackioh")

-- A price rule on the opponent (`add_cost_rule`, R455) for Spells only, lasting "theirNextTurn"
-- (R458). Both faces run this one script: the Radiant face's 2 is its declared `surcharge`.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.add_cost_rule({
				player = "enemy",
				rule = { types = { "Spell" }, amount = ctx:param("surcharge") },
				lasts = "theirNextTurn",
			}),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
