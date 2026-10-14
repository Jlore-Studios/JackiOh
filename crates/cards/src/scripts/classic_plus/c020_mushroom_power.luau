--!strict
local J = require("@jackioh")

-- The Units beside it (§3.1), read as the Cry resolves, each given +{buff}/+{buff}. One buff per
-- neighbour, in `adjacent_to`'s order: `for_each_card`'s `each` is handed only an id, so the amount
-- read here could not reach it.
local base = {
	cry = function(ctx: J.EffectContext)
		local amount = ctx:param("buff")
		local effects = {}
		for _, unit in ipairs(ctx:adjacent_to({ of = "self" }, {})) do
			effects[#effects + 1] = J.buff({
				target = { of = "instance", instanceId = unit.id },
				attack = amount,
				health = amount,
			})
		end
		return effects
	end,
}

-- The same script: the Radiant +4/+4 is its declared `buff`.
return {
	base = base,
	radiant = "base",
}
