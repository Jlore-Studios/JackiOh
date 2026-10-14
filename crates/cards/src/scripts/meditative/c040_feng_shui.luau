--!strict
local J = require("@jackioh")

-- The declared number `luck` (R987): "Aura: You have Luck {luck}".
local LUCK = 1

-- R982: the two "last element" preview labels — exact substrings of line 3 (R280).
local YOUR_LAST = "Your last element"
local THEIR_LAST = "your opponent's last element"

-- ME-ELEMENT is the engine's (`subsystems::feng_shui`); the card carries its flag, its Luck and,
-- through `preview`, both records (R982): the glyph as their `display` (R372), and "—" before any
-- play. No zone guard: the records read the same wherever the card is read.
local base = {
	staticFlags = {
		fengShui = true,
		luck = LUCK,
	},
	preview = function(ctx: J.ConditionContext)
		local me = ctx:controller()
		return {
			ctx:subsystems_feng_shui_element_preview(YOUR_LAST, ctx:subsystems_feng_shui_last_element(me)),
			ctx:subsystems_feng_shui_element_preview(
				THEIR_LAST,
				ctx:subsystems_feng_shui_last_element(ctx:opponent_of(me))
			),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
