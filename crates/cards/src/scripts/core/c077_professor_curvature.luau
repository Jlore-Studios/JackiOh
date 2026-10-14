--!strict
local J = require("@jackioh")

-- R363: the least current cost the discount reaches, checked after every other modifier (R65).
local TARGET_COST = 4

-- The two faces differ only in how much the discount is worth: the declared number `discount`
-- (R386), 1 and 2 on the Radiant face, read off the face that is running.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.add_player_modifier({
				player = "self",
				mod = {
					kind = "costDiscount",
					amount = ctx:param("discount"),
					minCurrentCost = TARGET_COST,
					-- R48: it covers the controller's NEXT turn, so it survives the turn it was created on.
					expiry = { ["until"] = "nextTurnOf", player = ctx:controller(), fromTurn = ctx:turn().number },
				},
			}),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
