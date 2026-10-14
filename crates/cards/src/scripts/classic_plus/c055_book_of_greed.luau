--!strict
local J = require("@jackioh")

local function book_of_greed(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.add_random_from_catalog({
					query = { rarity = { "Legendary", "Mythic" } },
					count = ctx:param("cards"),
					radiant = radiant,
				}),
			}
		end,
	}
end

return {
	base = book_of_greed(false),
	radiant = book_of_greed(true),
}
