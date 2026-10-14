--!strict
local J = require("@jackioh")

-- The three draws, in order: a KY card, a CN card, a Book.
local TAGS: { J.Tag } = { "KY", "CN", "Book" }

-- §5.1's pool for one tag: the tag's non-token cards of every set, minus this card (R387).
local function pool(tag: J.Tag): J.CatalogQueryArgs
	return { tags = { tag }, excludeDefId = "meditative-046" }
end

local function conjure(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			-- "(paid 4: it costs 0)": the embiggen price, read as the Cry resolves, so the paid face
			-- is the declared `setCost` (R386).
			local paid: number? = nil
			if ctx:embiggened() then
				paid = ctx:param("setCost")
			end
			local effects = {}
			for _, tag in ipairs(TAGS) do
				local args: J.AddRandomFromCatalogArgs = { query = pool(tag), count = 1 }
				if radiant then
					args.radiant = true
				end
				if paid ~= nil then
					args.costOverride = paid
				end
				effects[#effects + 1] = J.add_random_from_catalog(args)
			end
			return effects
		end,
	}
end

return {
	base = conjure(false),
	radiant = conjure(true),
}
