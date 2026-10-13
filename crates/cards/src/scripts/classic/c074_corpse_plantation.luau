--!strict
local J = require("@jackioh")

local base = {
	-- Activate, once per turn (R384): place the declared `tokens` on this (R386).
	activations = {
		{
			id = "plant",
			label = "Place Plague Counters on this",
			uses = 1,
			run = function(ctx: J.EffectContext)
				return {
					J.place_plague({ target = { of = "self" }, amount = ctx:param("tokens") }),
				}
			end,
		},
	},
	-- Units from your graveyard, paid with Plague Counters (§6.3 Play).
	graveyardPlay = function(ctx: J.HookArgs)
		local permission: J.GraveyardPlayPermission = { units = true, plague = true }
		return { permission }
	end,
}

-- The same script: the Radiant face's 4 tokens are its declared `tokens`.
return {
	base = base,
	radiant = "base",
}
