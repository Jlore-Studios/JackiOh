--!strict
local J = require("@jackioh")

-- Shuffle every graveyard card into your deck at random positions (R80's
-- cap keeps what doesn't fit), then draw {draw}; the Radiant face discounts
-- every shuffled card by {discount} (`costMod`, R78). The graveyard as the
-- Spell begins to resolve is read once for both halves: the discount half's
-- `cards` hook finds those ids in the library, and its `each` hook sets the
-- mod from `data`, which carries both the ids and the discount. A card's
-- declared numbers do not move while its Spell resolves, so the discount is
-- read in the Cry, where the context is in hand.
local function recycle(ctx: J.EffectContext, withDiscount: boolean)
	local shuffled: { string } = {}
	for _, card in ipairs(ctx:zone_cards(ctx:controller(), "graveyard")) do
		shuffled[#shuffled + 1] = card.id
	end
	local effects = {
		J.for_each_card({ cards = "each_cards", each = "each_card", data = { shuffled = shuffled } }),
	}
	if withDiscount then
		effects[#effects + 1] = J.for_each_card({
			cards = "each_cards_2",
			each = "each_card_2",
			data = { shuffled = shuffled, discount = ctx:param("discount") },
		})
	end
	effects[#effects + 1] = J.draw({ count = ctx:param("draw") })
	return effects
end

-- The graveyard's ids, as the Cry read them.
local function shuffled_cards(_ctx: J.EffectContext, data: any): { string }
	return data.shuffled
end

local function shuffle_each(id: string)
	return { J.shuffle_card_into({ instanceId = id }) }
end

-- The cards that went in: the graveyard's, now in the library.
local function discount_cards(ctx: J.EffectContext, data: any): { string }
	local ids: { string } = {}
	for _, card in ipairs(ctx:zone_cards(ctx:controller(), "library")) do
		for _, wanted in ipairs(data.shuffled) do
			if card.id == wanted then
				ids[#ids + 1] = card.id
			end
		end
	end
	return ids
end

local function discount_each(id: string, data: any)
	return {
		J.set_cost_mod({ target = { of = "instance", instanceId = id }, amount = -data.discount }),
	}
end

local base = {
	cry = function(ctx: J.EffectContext)
		return recycle(ctx, false)
	end,
	each_cards = shuffled_cards,
	each_card = shuffle_each,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return recycle(ctx, true)
	end,
	each_cards = shuffled_cards,
	each_card = shuffle_each,
	each_cards_2 = discount_cards,
	each_card_2 = discount_each,
}

return {
	base = base,
	radiant = radiant,
}
