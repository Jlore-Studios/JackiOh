--!strict
local J = require("@jackioh")

-- Where the Radiant face's discount travels from the firing to the answer (§10.6).
local DISCOUNT = "discount"

local function taxes(ctx: J.EffectContext, event: J.GameEvent): boolean
	if event.type ~= "drawn" then
		return false
	end
	return event.player == ctx:opponent_of(ctx:controller()) and event.turnDraw == ctx:param("draws")
end

local function discount_of(ctx: J.EffectContext): number
	-- Only an integer counts, else 0: the firing wrote an integer, and anything else is narrowed out.
	local data = ctx:data()
	if type(data) ~= "table" then
		return 0
	end
	local value = data[DISCOUNT]
	if type(value) ~= "number" then
		return 0
	end
	if value ~= math.floor(value) then
		return 0
	end
	return value
end

-- One shared trigger body: the Radiant face carries its discount in the prompt's data, the trap
-- being in the graveyard when the answer arrives.
local function tax(radiant: boolean): J.TriggerDef
	return {
		id = "income-tax",
		on = { "drawn" },
		when = taxes,
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			if ctx:zone_count(ctx:opponent_of(ctx:controller()), "hand") <= 1 then
				return {}
			end
			local data: { [string]: J.JsonValue } = {}
			if radiant then
				data[DISCOUNT] = ctx:param("discount")
			end
			return {
				J.choose_from_hand({
					of = "enemy",
					by = "enemy",
					count = 1,
					step = "keep",
					prompt = "Income Tax: keep one card; your opponent takes the rest",
					data = data,
				}),
			}
		end,
	}
end

local base = {
	triggers = { tax(false) },
	resume = {
		keep = function(ctx: J.EffectContext)
			return {
				J.give_from_hand({ from = "enemy", cards = "unchosen" }),
			}
		end,
	},
}

local radiant = {
	triggers = { tax(true) },
	resume = {
		keep = function(ctx: J.EffectContext)
			return {
				J.give_from_hand({ from = "enemy", cards = "unchosen", costMod = -discount_of(ctx) }),
			}
		end,
	},
}

return {
	base = base,
	radiant = radiant,
}
