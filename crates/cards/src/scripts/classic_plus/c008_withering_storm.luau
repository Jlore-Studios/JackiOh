--!strict
local J = require("@jackioh")

-- The deck cards the base face reaches, in deck order (R242); read as the list reaches them, so a
-- pause resumes over them (R113). R60, R129: {cards} different changeable cards, or all of them
-- (and no draw) when there are no more than that; R569 pads the pick to {cards} (or the deck's size)
-- with cards no change reaches.
local function picks(ctx: J.EffectContext): { string }
	local deck = ctx:zone_cards(ctx:opponent_of(ctx:controller()), "library")
	local count = math.max(math.min(ctx:param("cards"), #deck), 0)
	local changeable = {}
	local is_changeable: { [string]: boolean } = {}
	for _, card in ipairs(deck) do
		if #ctx:applicable_changes(card.id, "degrade") > 0 then
			changeable[#changeable + 1] = card
			is_changeable[card.id] = true
		end
	end
	local chosen = changeable
	if #changeable > count then
		local shuffled = ctx:rng_shuffle(changeable)
		chosen = {}
		for at = 1, count do
			chosen[at] = shuffled[at]
		end
	end
	local picked: { [string]: boolean } = {}
	local size = 0
	for _, card in ipairs(chosen) do
		if not picked[card.id] then
			picked[card.id] = true
			size += 1
		end
	end
	for _, card in ipairs(deck) do
		if size < count and not is_changeable[card.id] and not picked[card.id] then
			picked[card.id] = true
			size += 1
		end
	end
	local ids = {}
	for _, card in ipairs(deck) do
		if picked[card.id] then
			ids[#ids + 1] = card.id
		end
	end
	return ids
end

-- A `cards` function cannot answer an empty list (an empty table crosses as a record, which
-- `for_each_card` refuses), so the empty deck, the one deck the hook can see picks nothing (`cards` is
-- at least 1), hands over its empty list outright; the picks are otherwise read as the list reaches
-- them.
local base = {
	cry = function(ctx: J.EffectContext)
		local deck = ctx:zone_cards(ctx:opponent_of(ctx:controller()), "library")
		local cards: J.CardsSource = if #deck == 0 then {} else "each_cards"
		return {
			J.for_each_card({ cards = cards, each = "each_card" }),
			J.draw({ count = ctx:param("draw") }),
		}
	end,
	each_cards = function(ctx: J.EffectContext)
		return picks(ctx)
	end,
	each_card = function(id: string)
		return { J.degrade({ instanceId = id }) }
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.degrade({ scope = { side = "enemy", zones = { "library" } } }),
			J.draw({ count = ctx:param("draw") }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
