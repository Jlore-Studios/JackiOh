--!strict
local J = require("@jackioh")

-- The aura "Fruits you play are Radiant": Core #64 Gifted Program's static flag at §10.5 step 3, by
-- tag (R213, R214), on both faces.
local STATIC_FLAGS: J.StaticFlags = { radiantPlaysTagged = { "Fruit" } }

-- The Fruit pool (R382), never Organic Produce itself (R387).
local FRUIT: J.CatalogQueryArgs = { tags = { "Fruit" } }

local base = {
	staticFlags = STATIC_FLAGS,
	cry = function(ctx: J.EffectContext)
		return {
			J.add_random_from_catalog({ query = FRUIT, count = ctx:param("fruits") }),
		}
	end,
}

-- The Radiant face's "Each costs (0).": the declared number `setCost` (R386), less being better.
local radiant = {
	staticFlags = STATIC_FLAGS,
	cry = function(ctx: J.EffectContext)
		return {
			J.add_random_from_catalog({
				query = FRUIT,
				count = ctx:param("fruits"),
				costOverride = ctx:param("setCost"),
			}),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
