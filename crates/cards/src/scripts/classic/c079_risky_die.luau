--!strict
local J = require("@jackioh")

-- Where the resolving card notes its controller's library as the Spell begins.
local LIBRARY_KEY = "riskyDieLibrary"

-- "They": the cards in its controller's hand now that were in their library as the Spell began.
local function drawn_into_hand(ctx: J.EffectContext): { string }
	local noted = ctx:recalled(LIBRARY_KEY)
	local library: { [string]: boolean } = {}
	if type(noted) == "table" then
		for _, id in ipairs(noted) do
			if type(id) == "string" then
				library[id] = true
			end
		end
	end
	local ids: { string } = {}
	for _, card in ipairs(ctx:zone_cards(ctx:controller(), "hand")) do
		if library[card.id] then
			ids[#ids + 1] = card.id
		end
	end
	return ids
end

local base = {
	cry = function(ctx: J.EffectContext)
		local library: { string } = {}
		for _, card in ipairs(ctx:zone_cards(ctx:controller(), "library")) do
			library[#library + 1] = card.id
		end
		-- "They cost (1) less": the declared number `discount`, carried in `data` so the later
		-- clauses read the same number (R386).
		local discount = ctx:param("discount")
		local threshold = ctx:param("threshold")
		return {
			J.remember({ key = LIBRARY_KEY, value = library }),
			J.draw({ count = ctx:param("draw") }),
			J.for_each_card({
				cards = "each_cards",
				each = "each_card",
				data = { discount = discount },
			}),
			J.for_each_card({
				cards = "each_cards_2",
				each = "each_card_2",
				data = { threshold = threshold },
			}),
		}
	end,
	each_cards = function(ctx: J.EffectContext)
		return drawn_into_hand(ctx)
	end,
	each_card = function(id: string, data: any)
		local discount = 0
		if type(data) == "table" and type(data.discount) == "number" then
			discount = data.discount
		end
		return { J.set_cost_mod({ target = { of = "instance", instanceId = id }, amount = -discount }) }
	end,
	each_cards_2 = function(ctx: J.EffectContext, data: any)
		local threshold = 0
		if type(data) == "table" and type(data.threshold) == "number" then
			threshold = data.threshold
		end
		local ids: { string } = {}
		for _, id in ipairs(drawn_into_hand(ctx)) do
			if ctx:effective_cost(id) > threshold then
				ids[#ids + 1] = id
			end
		end
		return ids
	end,
	each_card_2 = function(id: string)
		return { J.exile({ target = { of = "instance", instanceId = id } }) }
	end,
}

-- The same script: the Radiant face differs only in its declared threshold, read through `param`.
return {
	base = base,
	radiant = "base",
}
