--!strict
local J = require("@jackioh")

-- The one cost the card names.
local RECRUIT_COST = 1

local function recruits_face(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.for_each_card({ cards = "each_cards", each = "each_card", data = { radiant = radiant } }),
			}
		end,
		-- {cards} different Units costing (1) in the deck (R65/R66, X excluded, R1061), all of them
		-- if fewer, read once as the clause reaches them (R113).
		each_cards = function(ctx: J.EffectContext)
			local count = math.max(ctx:param("cards"), 0)
			local matching = {}
			for _, card in ipairs(ctx:zone_cards(ctx:controller(), "library")) do
				if
					ctx:card_type_of(card.id) == "Unit"
					and not ctx:is_x_cost(card.id)
					and ctx:effective_cost(card.id) == RECRUIT_COST
				then
					matching[#matching + 1] = card.id
				end
			end
			if #matching <= count then
				return matching
			end
			local ids: { string } = {}
			for at, id in ipairs(ctx:rng_shuffle(matching)) do
				if at > count then
					break
				end
				ids[#ids + 1] = id
			end
			return ids
		end,
		each_card = function(id: string, data: any)
			return { J.draw_from_library({ instanceId = id, radiant = data.radiant }) }
		end,
	}
end

return {
	base = recruits_face(false),
	radiant = recruits_face(true),
}
