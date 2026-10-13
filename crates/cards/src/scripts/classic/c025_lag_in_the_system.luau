--!strict
local J = require("@jackioh")

-- Every card on the field and in the hands and decks of `side`'s sides, in the order the header
-- gives — or the field alone (the whole board) when `fieldOnly`, which is the base face's scope
-- (balance patch 1).
local function reachable(ctx: J.EffectContext, side: J.ScopeSide, fieldOnly: boolean): { any }
	local cards = ctx:cards_in_scope({ side = side, rows = { "units", "backrow" } })
	if fieldOnly then
		return cards
	end
	local players = {}
	if side == "any" then
		players[#players + 1] = ctx:controller()
	end
	for _, player in ipairs(ctx:sides_of("enemy")) do
		players[#players + 1] = player
	end
	for _, player in ipairs(players) do
		for _, card in ipairs(ctx:zone_cards(player, "hand")) do
			cards[#cards + 1] = card
		end
		for _, card in ipairs(ctx:zone_cards(player, "library")) do
			cards[#cards + 1] = card
		end
	end
	return cards
end

-- The ids the sweep exiles: the reachable cards whose cost now (R396) is at most `threshold`. Read as
-- the Cry runs, which is the moment the list's first and only effect reads its set, so the set is
-- the one the Spell resolves against; an empty set leaves `cards` out, for an empty table crosses as
-- a record, not a list.
local function lag(ctx: J.EffectContext, side: J.ScopeSide, fieldOnly: boolean)
	local threshold = ctx:param("threshold")
	local ids: { string } = {}
	for _, card in ipairs(reachable(ctx, side, fieldOnly)) do
		if ctx:cost_now(card.id) <= threshold then
			ids[#ids + 1] = card.id
		end
	end
	return {
		J.for_each_card({
			cards = if #ids > 0 then ids else nil,
			each = "each_card",
		}),
	}
end

local function exileIt(id: string)
	return { J.exile({ target = { of = "instance", instanceId = id } }) }
end

local base = {
	cry = function(ctx: J.EffectContext)
		return lag(ctx, "any", true)
	end,
	each_card = exileIt,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return lag(ctx, "enemy", false)
	end,
	each_card = exileIt,
}

return {
	base = base,
	radiant = radiant,
}
