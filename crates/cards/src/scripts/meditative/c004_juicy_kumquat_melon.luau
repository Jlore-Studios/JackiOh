--!strict
local J = require("@jackioh")

-- The six costs the card draws, in order.
local COSTS = { 0, 1, 2, 3, 4, 5 }

-- Where the resolving card notes its six picks as the effect begins.
local PICKED_KEY = "kumquatPicked"

-- For each cost in `COSTS`, the topmost library card (the list's first is the top) whose set is
-- Meditative and whose cost, read as `matches_library_filter` reads it (R65), is that cost (R801).
-- The picks are written into the library list the host handed over, emptied first: a host list
-- crosses back as a list even when empty (`[]`, as the Rust remembers no picks), where a script's
-- own `{}` crosses as a record (`numbers.rs`, #464 gap 1).
local function choices(ctx: J.EffectContext): { string }
	local library = ctx:zone_cards(ctx:controller(), "library")
	local meditative: { { id: string, cost: number } } = {}
	for _, card in ipairs(library) do
		if ctx:def_of(card.defId).set == "Meditative" then
			meditative[#meditative + 1] = { id = card.id, cost = ctx:effective_cost(card.id) }
		end
	end
	table.clear(library)
	for _, cost in ipairs(COSTS) do
		for _, card in ipairs(meditative) do
			if card.cost == cost then
				library[#library + 1] = card.id
				break
			end
		end
	end
	return library
end

-- The picks the resolving card noted as the effect began.
local function picked(ctx: J.EffectContext): { string }
	local ids: { string } = {}
	local noted = ctx:recalled(PICKED_KEY)
	if type(noted) ~= "table" then
		return ids
	end
	for _, id in ipairs(noted) do
		if type(id) == "string" then
			ids[#ids + 1] = id
		end
	end
	return ids
end

-- "They": the picked cards that reached the controller's hand, in hand order (a cast-on-draw card
-- never gets there, R58; a burned one isn't there either).
local function picked_in_hand(ctx: J.EffectContext): { string }
	local wanted: { [string]: boolean } = {}
	for _, id in ipairs(picked(ctx)) do
		wanted[id] = true
	end
	local ids: { string } = {}
	for _, card in ipairs(ctx:zone_cards(ctx:controller(), "hand")) do
		if wanted[card.id] then
			ids[#ids + 1] = card.id
		end
	end
	return ids
end

-- The six picks, chosen as the effect begins and noted on the resolving card (`remember`), so a
-- cast-on-draw pause cannot lose them; then one `draw_from_library` each, a real §2.4 draw.
local function cry(ctx: J.EffectContext): { J.EffectCall }
	return {
		J.remember({ key = PICKED_KEY, value = choices(ctx) }),
		J.for_each_card({ cards = "each_cards", each = "each_card" }),
	}
end

local function each_cards(ctx: J.EffectContext)
	return picked(ctx)
end

local function each_card(id: string)
	return {
		J.draw_from_library({ instanceId = id }),
	}
end

local base = {
	cry = cry,
	each_cards = each_cards,
	each_card = each_card,
}

local radiant = {
	-- "They cost (2) less": the declared number `discount` (R386), on each picked card that reached
	-- the hand (R4).
	cry = function(ctx: J.EffectContext)
		local discount = ctx:param("discount")
		local effects = cry(ctx)
		effects[#effects + 1] = J.for_each_card({
			cards = "each_cards_2",
			each = "each_card_2",
			data = { discount = discount },
		})
		return effects
	end,
	each_cards = each_cards,
	each_card = each_card,
	each_cards_2 = function(ctx: J.EffectContext)
		return picked_in_hand(ctx)
	end,
	each_card_2 = function(id: string, data: any)
		local target: J.TargetSpec = { of = "instance", instanceId = id }
		return {
			J.set_cost_mod({ target = target, amount = -data.discount, inHandOnly = true }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
