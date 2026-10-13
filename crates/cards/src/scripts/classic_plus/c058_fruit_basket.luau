--!strict
local J = require("@jackioh")

-- A `tags = { "Fruit" }` query is the Fruit pool, Grapes included (R382); the verb excludes this card
-- (R387).
local function fruit_basket(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.add_random_from_catalog({
					query = { tags = { "Fruit" } },
					count = ctx:param("fruits"),
					radiant = radiant,
				}),
			}
		end,
	}
end

return {
	base = fruit_basket(false),
	radiant = fruit_basket(true),
}
