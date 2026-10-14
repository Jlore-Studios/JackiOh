--!strict
local J = require("@jackioh")

-- Each card in the hand takes `times` separate Buffs (R386, R442); the Radiant face's three are its
-- declared `times`, read off the running face.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.upgrade({
				scope = { side = "self", zones = { "hand" } },
				times = ctx:param("times"),
			}),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
