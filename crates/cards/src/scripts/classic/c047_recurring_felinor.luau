--!strict
local J = require("@jackioh")

local ANCIENT_ACQUISITION = "classic-034"

-- "Cry: Cast Ancient Acquisition": a generated C #34 on its base face, free (R70, R684, R87).
local function cry(ctx: J.EffectContext)
	return {
		J.cast_new({ def = { defId = ANCIENT_ACQUISITION, radiant = false } }),
	}
end

-- "While this is in your graveyard: When one of your Traps reveals, return this to hand": a graveyard
-- trigger's `when` is not consulted outside the trap window, so "one of your Traps" is checked in
-- `run`, which answers anything else with no effects. `priced` is the Radiant face's "It costs
-- ({returnCost})", read as the trigger runs.
local function recur(priced: boolean)
	return {
		id = "recurring-felinor",
		on = { "trapFired" },
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			if event.type ~= "trapFired" then
				return {}
			end
			if event.controller ~= ctx:controller() then
				return {}
			end
			local target: J.TargetSpec = { of = "self" }
			if priced then
				return { J.add_to_hand({ instance = target, costOverride = ctx:param("returnCost") }) }
			end
			return { J.add_to_hand({ instance = target }) }
		end,
	}
end

return {
	base = {
		cry = function(ctx: J.EffectContext)
			return cry(ctx)
		end,
		graveyardTriggers = { recur(false) },
	},
	radiant = {
		cry = function(ctx: J.EffectContext)
			return cry(ctx)
		end,
		graveyardTriggers = { recur(true) },
	},
}
