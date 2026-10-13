--!strict
local J = require("@jackioh")

-- The step name the Discover's continuation carries (`resume[step]`).
local PICKED = "picked"

local base = {
	startOfTurn = function(ctx: J.EffectContext)
		return { J.add_random_from_graveyard({ player = "self" }) }
	end,
}

local radiant = {
	startOfTurn = function(ctx: J.EffectContext)
		return {
			J.discover_from_graveyard({
				step = PICKED,
				prompt = "Discover a card from your graveyard",
			}),
		}
	end,
	resume = {
		[PICKED] = function(ctx: J.EffectContext)
			return {
				J.bounce({ target = { of = "chosen" } }),
				-- R65: "costs 1 less" is a −1 `costMod` on the chosen instance, permanent and zone-proof
				-- (R78); the 1 is the declared number `discount` (R386). R4: it is its price in the hand,
				-- so a pick a full hand burns keeps its cost.
				J.set_cost_mod({
					target = { of = "chosen" },
					amount = -ctx:param("discount"),
					inHandOnly = true,
				}),
			}
		end,
	},
}

return {
	base = base,
	radiant = radiant,
}
