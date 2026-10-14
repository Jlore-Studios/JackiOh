--!strict
local J = require("@jackioh")

-- The Prime pool (R1421): a Prime tag query, whose pool takes its tokens (R382).
local function dragon_fruit(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.add_random_from_catalog({
					query = { tags = { "Prime" } },
					count = ctx:param("cards"),
					radiant = radiant,
				}),
			}
		end,
	}
end

return {
	base = dragon_fruit(false),
	radiant = dragon_fruit(true),
}
