--!strict
local J = require("@jackioh")

-- C+ #12's pool, the eight Pancake tokens, on their base faces (R60). Reborn is catalog data; the
-- Death is the same on the Radiant face.
local base = {
	death = function(ctx: J.EffectContext)
		return {
			J.add_random_from_catalog({
				query = { tags = { "Pancake", "Token" } },
				count = ctx:param("tokens"),
			}),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
