--!strict
local J = require("@jackioh")

-- Every non-CN permanent, both sides, both rows.
local function non_cn_permanents(): J.BoardScope
	return { side = "any", rows = { "units", "backrow" }, notTags = { "CN" } }
end

local function mandate(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local scope = non_cn_permanents()
			-- R1142: counted before the exile, by the controller as it stands now. MD-B1, R940: the
			-- exile is harmful, so an immune card is in neither the count nor the exile.
			local me = ctx:controller()
			local yours = 0
			for _, card in ipairs(ctx:cards_in_scope_aimed(scope, "harm")) do
				if card.controller == me then
					yours += 1
				end
			end
			local effects = { J.exile_all(scope) }
			if yours > 0 then
				local add: J.AddRandomFromCatalogArgs = { query = { tags = { "CN" } }, count = yours }
				if radiant then
					add.radiant = true
					add.costMod = -ctx:param("discount")
				end
				effects[#effects + 1] = J.add_random_from_catalog(add)
			end
			return effects
		end,
	}
end

return {
	base = mandate(false),
	radiant = mandate(true),
}
