--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.set_radiant_random({
				zones = { "hand", "library", "field" },
				count = ctx:param("cards"),
			}),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
