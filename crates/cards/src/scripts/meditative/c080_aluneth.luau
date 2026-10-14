--!strict
local J = require("@jackioh")

-- What the end of its controller's turn draws.
local DRAWS = 3

-- Quickdraw is the static flag `setup.rs` reads (R640); Indestructible, Untributable and Immutable
-- are catalog keywords. The draw is one `draw` of 3, burning past the cap and taking fatigue past
-- the deck card by card (§2.4).
local function end_of_turn(ctx: J.EffectContext)
	return { J.draw({ count = DRAWS }) }
end

return {
	base = {
		staticFlags = { quickdraw = true },
		endOfTurn = end_of_turn,
	},
	-- The Radiant face drops Untributable (catalog data) and adds an Activate (R384) that exiles it.
	radiant = {
		staticFlags = { quickdraw = true },
		endOfTurn = end_of_turn,
		activations = {
			{
				id = "exile",
				label = "Exile this",
				uses = 1,
				run = function(ctx: J.EffectContext)
					return { J.exile({ target = { of = "self" } }) }
				end,
			},
		},
	},
}
