--!strict
local J = require("@jackioh")

-- The cards this list put in its controller's hand (R136: its own events, from `eventsFrom` —
-- the window `ctx:events()` already answers).
local function added_by_this_list(ctx: J.EffectContext): { string }
	local me = ctx:controller()
	local ids: { string } = {}
	for _, event in ipairs(ctx:events()) do
		if event.type == "addedToHand" and event.player == me then
			ids[#ids + 1] = event.instanceId
		end
	end
	return ids
end

local function dropship(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local brittle = ctx:param("brittle")
			local add: J.AddRandomFromCatalogArgs = {
				query = { withTokens = true, excludeDefId = "classicplus-023" },
				count = ctx:param("cards"),
			}
			if radiant then
				add.costOverride = ctx:param("setCost")
			end
			return {
				J.add_random_from_catalog(add),
				J.for_each_card({ cards = "each_cards", each = "each_card", data = { n = brittle } }),
			}
		end,
		each_cards = function(ctx: J.EffectContext)
			return added_by_this_list(ctx)
		end,
		each_card = function(id: string, data: any)
			local n = 0
			if type(data) == "table" and type(data.n) == "number" then
				n = data.n
			end
			return { J.give_brittle({ instanceId = id, n = n }) }
		end,
	}
end

return {
	base = dropship(false),
	radiant = dropship(true),
}
