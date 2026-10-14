--!strict
local J = require("@jackioh")

-- The fixed range the number is chosen from (`GLITCH_NUMBERS` in `engine/config.rs`), as the
-- choice's options. The range is not in `J.config` (part 28's gap-7 list), and a batch may not
-- extend the API, so the bounds are named here, next to the use.
local GLITCH_FIRST = 0
local GLITCH_LAST = 10

local function glitch_options(): { string }
	local options: { string } = {}
	for n = GLITCH_FIRST, GLITCH_LAST do
		options[#options + 1] = tostring(n)
	end
	return options
end

-- The number the play carried: the one integer option it named, or nothing (§10.6's choices are
-- strings; the play's mode is validated against these options, so the parse below reads it back).
local function chosen_glitch_number(ctx: J.EffectContext): number?
	local picked = ctx:chosen_number()
	if picked == nil then
		return nil
	end
	if picked < GLITCH_FIRST or picked > GLITCH_LAST then
		return nil
	end
	return picked
end

-- Every card on the field and in the hands and decks of `whose` sides, the Spell's controller
-- first, or the field alone (the whole board) when `field_only`, the base face's scope.
local function reachable(ctx: J.EffectContext, side: J.ScopeSide, field_only: boolean): { any }
	local field = ctx:cards_in_scope({ side = side, rows = { "units", "backrow" } })
	if field_only then
		return field
	end
	local out = field
	if side == "any" then
		for _, card in ipairs(ctx:zone_cards(ctx:controller(), "hand")) do
			out[#out + 1] = card
		end
		for _, card in ipairs(ctx:zone_cards(ctx:controller(), "library")) do
			out[#out + 1] = card
		end
	end
	for _, player in ipairs(ctx:sides_of("enemy")) do
		for _, card in ipairs(ctx:zone_cards(player, "hand")) do
			out[#out + 1] = card
		end
		for _, card in ipairs(ctx:zone_cards(player, "library")) do
			out[#out + 1] = card
		end
	end
	return out
end

local function glitch(side: J.ScopeSide, field_only: boolean)
	return {
		modes = {
			{ kind = "number", options = glitch_options() },
		},
		cry = function(ctx: J.EffectContext)
			local number = chosen_glitch_number(ctx)
			if number == nil then
				return {}
			end
			return {
				J.for_each_card({ cards = "each_cards", each = "each_card", data = { number = number } }),
			}
		end,
		each_cards = function(ctx: J.EffectContext, data: any)
			if type(data) ~= "table" or type(data.number) ~= "number" then
				return {}
			end
			local number = data.number
			local ids: { string } = {}
			for _, card in ipairs(reachable(ctx, side, field_only)) do
				if ctx:cost_now(card.id) == number then
					ids[#ids + 1] = card.id
				end
			end
			return ids
		end,
		each_card = function(id: string)
			return { J.exile({ target = { of = "instance", instanceId = id } }) }
		end,
	}
end

return {
	base = glitch("any", true),
	radiant = glitch("enemy", false),
}
