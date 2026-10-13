--!strict
local J = require("@jackioh")

-- The chosen permanent if it is still on the field, and whether the steal branch holds for it:
-- an enemy permanent with at least its cost in Plague Counters — or one this run's steal has
-- just taken, which the draw half reads as the steal branch too (R136: this script's own
-- `controlChanged`, read off the action's events window).
type Outcome = { id: string, steals: boolean, tokens: number }

local function outcome(ctx: J.EffectContext): Outcome?
	local card = ctx:instance_of({ of = "chosen" })
	if type(card) ~= "table" then
		return nil
	end
	if card.zone.z ~= "field" then
		return nil
	end
	local tokens = ctx:plague_on(card.id)
	local taken_now = false
	for _, event in ipairs(ctx:events()) do
		if event.type == "controlChanged" and event.instanceId == card.id then
			taken_now = true
		end
	end
	local steals = taken_now
		or (card.controller ~= ctx:controller() and tokens >= ctx:cost_now(card.id))
	local read: Outcome = { id = card.id, steals = steals, tokens = tokens }
	return read
end

local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit", "backrow" } },
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.place_plague({ target = { of = "chosen" }, amount = ctx:param("tokens") }),
			J.for_each_card({ cards = "each_cards", each = "each_card" }),
			J.for_each_card({ cards = "each_cards_2", each = "each_card_2" }),
		}
	end,
	each_cards = function(ctx: J.EffectContext)
		local read = outcome(ctx)
		if read == nil or not read.steals then
			return {}
		end
		return { read.id }
	end,
	each_card = function(id: string)
		return { J.steal({ instanceId = id }) }
	end,
	each_cards_2 = function(ctx: J.EffectContext)
		local read = outcome(ctx)
		if read == nil or read.steals then
			return {}
		end
		local ids: { string } = {}
		for _ = 1, read.tokens do
			ids[#ids + 1] = read.id
		end
		return ids
	end,
	each_card_2 = function(id: string)
		return { J.draw({ count = 1 }) }
	end,
}

-- The same script: the Radiant face's 2 tokens are its declared `tokens`, which `param` reads
-- off the running face.
return {
	base = base,
	radiant = "base",
}
