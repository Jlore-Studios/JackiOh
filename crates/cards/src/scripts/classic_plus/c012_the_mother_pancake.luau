--!strict
local J = require("@jackioh")

-- The eight Pancake tokens: Pancake- and Token-tagged (Mother Pancake and Mommy Barker are no
-- tokens). The Radiant face differs only in its stats and its `tokens` value, both catalog data.
local base = {
	endOfTurn = function(ctx: J.EffectContext)
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
