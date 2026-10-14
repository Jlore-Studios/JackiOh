--!strict
local J = require("@jackioh")

-- On the opponent's `cardPlayed` (§10.5 step 4: a cast is a play, R70), the top `exiles` cards of
-- the controller's library are exiled by C+ #12.6 Frozen Wastes' pattern: `for_each_card` over the
-- library's top (index 0), one `exile` each. The player and the count are read as the trigger runs
-- and reach the `cards` hook through `data`. A short deck exiles what it has; an empty one nothing.
local exile_top = {
	id = "jlockwork-machine-exile",
	on = { "cardPlayed" },
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		if event.type ~= "cardPlayed" then
			return {}
		end
		if event.player == ctx:controller() then
			return {}
		end
		return {
			J.for_each_card({
				cards = "each_cards",
				each = "each_card",
				data = { player = ctx:controller(), count = math.max(ctx:param("exiles"), 0) },
			}),
		}
	end,
}

local base = {
	triggers = { exile_top },
	each_cards = function(ctx: J.EffectContext, data: any): { string }
		local ids: { string } = {}
		for _, card in ipairs(ctx:zone_cards(data.player, "library")) do
			if #ids >= data.count then
				break
			end
			ids[#ids + 1] = card.id
		end
		return ids
	end,
	each_card = function(id: string)
		local card: J.TargetSpec = { of = "instance", instanceId = id }
		return { J.exile({ target = card }) }
	end,
}

-- Rush is catalog data, so the Radiant face's script is the base's.
return {
	base = base,
	radiant = "base",
}
