--!strict
local J = require("@jackioh")

-- §8: two cards, both Units in the 3-4 bracket.
local COUNT = 2

-- §5.1's pool: Units costing 3 or 4, no tokens (automatic), never Straaza herself (R387).
local function unit_pool(): J.CatalogQueryArgs
	return {
		type = "Unit",
		costRange = { min = 3, max = 4 },
		excludeDefId = "core-054",
	}
end

-- The two faces differ in what the generated cards cost — "they cost 1", "they cost 0": the
-- declared number `setCost` (R386), less being better — and whether they are Radiant.
local function straaza(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.add_random_from_catalog({
					query = unit_pool(),
					count = COUNT,
					costOverride = ctx:param("setCost"),
					radiant = radiant,
				}),
			}
		end,
	}
end

return {
	base = straaza(false),
	radiant = straaza(true),
}
