--!strict
local J = require("@jackioh")

-- §8.2's two modes, as the play action spells them (R81).
local HIGHEST = "highest"
local LOWEST = "lowest"

-- R24: the library card with the extreme current cost, ties going to the card nearest the top.
-- Costs are R65's out-of-play numbers: X counts 0, embiggen its base price, the instance's `costMod`
-- persists (R78), and player discounts never price a library card. `effective_cost` is the one
-- calculation; `query_cost` reads a DEFINITION, blind to #7's `costMod` and #95's library discount.
-- Reads through `zone_cards` and writes nothing (CLAUDE.md rule 5).
local function extreme(ctx: J.EffectContext, want: string): any
	local best: any = nil
	local bestCost = 0
	for _, card in ipairs(ctx:zone_cards(ctx:controller(), "library")) do
		local cost = ctx:effective_cost(card.id)
		local better
		if want == HIGHEST then
			better = cost > bestCost
		else
			better = cost < bestCost
		end
		-- Strict, and top down, so a tie keeps the card already held — the one nearer the top (R24).
		if best == nil or better then
			best = card
			bestCost = cost
		end
	end
	return best
end

-- §6.3 Draw of one named library card (R24), or nothing for an empty library.
local function draw_named(card: any): { J.EffectCall }
	if card == nil then
		return {}
	end
	return { J.draw_from_library({ instanceId = card.id }) }
end

local base = {
	modes = {
		{ kind = "mode", options = { HIGHEST, LOWEST } },
	},
	cry = function(ctx: J.EffectContext)
		-- R81: the mode arrived with the play, so there is nothing to wait for. R90 validates the
		-- declaration, so anything else means no mode was carried at all — fizzle rather than guess.
		local mode = ctx:chosen_options()[1]
		if mode == HIGHEST or mode == LOWEST then
			return draw_named(extreme(ctx, mode))
		end
		return {}
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		-- A one-card library is its own highest and lowest, so "draw both" draws it once.
		-- The two are read once, as the Cry begins: a highest card that is cast on draw and asks has
		-- left the library by the answer, and a rebuilt pair would name other cards (R113,
		-- `for_each_card`, which keeps the ids it began with for the rebuild). The clause is the
		-- list's head, so the ids read here are the ones it reads as the list reaches it.
		local high = extreme(ctx, HIGHEST)
		local low = extreme(ctx, LOWEST)
		local ids: { string } = {}
		if high ~= nil then
			ids[#ids + 1] = high.id
		end
		if low ~= nil and (high == nil or low.id ~= high.id) then
			ids[#ids + 1] = low.id
		end
		-- An empty table crosses as a record, not a list, so an empty library names no `cards` at all,
		-- which the verb reads as the empty set.
		if #ids == 0 then
			return { J.for_each_card({ each = "each_card" }) }
		end
		return { J.for_each_card({ cards = ids, each = "each_card" }) }
	end,
	each_card = function(id: string)
		return { J.draw_from_library({ instanceId = id }) }
	end,
}

return {
	base = base,
	radiant = radiant,
}
