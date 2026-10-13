--!strict
local J = require("@jackioh")

-- The name the prompt's continuation is filed under, in `resume` and in `step` (§10.6).
local STEP_CHOSEN = "chosen"

local function cry(ctx: J.EffectContext): { J.EffectCall }
	return {
		J.discover_from_graveyard({
			step = STEP_CHOSEN,
			prompt = "Discover a card from your graveyard",
		}),
		-- Parked while the prompt is open; runs straight through on an empty graveyard.
		J.exile({ target = { of = "self" } }),
	}
end

-- R4: the price is the card's in the hand it reached, so a pick a full hand burns keeps its cost.
-- "It costs (1) less" and "It costs (0)" are the declared numbers `discount` and `setCost` (R386).
-- The answered selection arrives in `ctx.targets`, which `{ of = "chosen" }` reads (§10.6).
local base = {
	cry = cry,
	resume = {
		[STEP_CHOSEN] = function(ctx: J.EffectContext)
			local amount = -ctx:param("discount")
			return {
				J.add_to_hand({ instance = { of = "chosen" } }),
				J.set_cost_mod({ target = { of = "chosen" }, amount = amount, inHandOnly = true }),
			}
		end,
	},
}

local radiant = {
	cry = cry,
	resume = {
		[STEP_CHOSEN] = function(ctx: J.EffectContext)
			local cost = ctx:param("setCost")
			return {
				J.add_to_hand({ instance = { of = "chosen" } }),
				J.set_cost_override({ target = { of = "chosen" }, cost = cost, inHandOnly = true }),
			}
		end,
	},
}

return {
	base = base,
	radiant = radiant,
}
