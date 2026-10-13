--!strict
local J = require("@jackioh")

-- §5.1's KY pool: the KY tag minus tokens (automatic) minus this card (`excludeDefId`, R387).
local function ky_pool(): J.CatalogQueryArgs
	return { tags = { "KY" }, excludeDefId = "core-057" }
end

return {
	-- A spell's script is its `cry` hook (§10.9). The counts are the declared numbers `cards` (3,
	-- and 2 on the Radiant face) and the Radiant face's `radiantCards` (2), R386.
	base = {
		cry = function(ctx: J.EffectContext)
			return {
				J.add_random_from_catalog({ query = ky_pool(), count = ctx:param("cards") }),
			}
		end,
	},
	-- "2 random plus 2 random Radiant KY cards": a restated clause replaces the base one (§8
	-- Conventions), so the radiant face adds 4 cards, not 3 + 4. The plain pair is rolled first so
	-- the radiant flag lands on exactly the last two adds.
	radiant = {
		cry = function(ctx: J.EffectContext)
			local plain = ctx:param("cards")
			local radiant = ctx:param("radiantCards")
			return {
				J.add_random_from_catalog({ query = ky_pool(), count = plain }),
				J.add_random_from_catalog({ query = ky_pool(), count = radiant, radiant = true }),
			}
		end,
	},
}
