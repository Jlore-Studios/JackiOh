--!strict
local J = require("@jackioh")

-- "It costs (0)": the declared number `setCost` (R386), on both faces.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.take_from_library({
				from = "enemy",
				pick = "random",
				filter = { type = "Unit" },
				costOverride = ctx:param("setCost"),
			}),
		}
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.take_from_library({
				from = "enemy",
				pick = "random",
				filter = { type = "Unit" },
				costOverride = ctx:param("setCost"),
				radiant = true,
			}),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
