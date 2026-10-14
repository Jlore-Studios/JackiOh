--!strict
local J = require("@jackioh")

local function ai_cards(): J.CatalogQueryArgs
	return { tags = { "AI" }, token = true }
end

-- The same script on both faces: the Radiant face's picks are Radiant through `ctx:radiant()`.
local base = {
	cry = function(ctx: J.EffectContext)
		local count = ctx:param("cards")
		if count < 2 then
			return {
				J.add_random_from_catalog({
					query = ai_cards(),
					count = count,
					costOverride = 0,
					radiant = ctx:radiant(),
				}),
			}
		end
		return {
			J.fuse_generated({ count = count, query = ai_cards(), radiant = ctx:radiant() }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
