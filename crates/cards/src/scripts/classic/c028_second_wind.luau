--!strict
local J = require("@jackioh")

-- The instance-memory mark the Cry holds while its own discard lands (R393).
local CRY_RUNNING = "secondWindCry"

-- Both faces' Cry: exile the deck, each card its own exile (R135), then discard the hand (R16: a whole
-- hand needs no choice). Its discard must land in the graveyard before the Aura starts exiling (R393),
-- so the Cry marks itself as running (`remember`) around it.
local function cry(ctx: J.EffectContext)
	return {
		J.remember({ key = CRY_RUNNING, value = true }),
		J.exile_matching({ zones = { "library" } }),
		J.discard_hand({}),
		J.remember({ key = CRY_RUNNING, value = false }),
	}
end

-- Both faces' Aura (`Script.graveyardPlay`, B5 E11): while this is on the field, any card of your own
-- graveyard is played as from a hand (§10.5), at a price of at least the declared `minCost` as paid.
local function graveyard_play(ctx: J.HookArgs)
	local permission: J.GraveyardPlayPermission = { minPrice = ctx:param("minCost") }
	return { permission }
end

return {
	base = {
		cry = function(ctx: J.EffectContext)
			return cry(ctx)
		end,
		graveyardPlay = graveyard_play,
		-- B5 E5: your cards that would go to a graveyard are exiled, but not while the Cry's mark is on
		-- (R393); a Second Wind that arrives without its Cry (a Recruit, a summon) has no mark.
		replacements = {
			{
				id = "second-wind-exile",
				on = "toGraveyard",
				when = function(ctx: J.ReplacementContext)
					local event = ctx:event()
					local owned = event.moment == "toGraveyard" and event.owner == ctx:controller()
					return owned and ctx:self_().memory[CRY_RUNNING] ~= true
				end,
				instead = { to = "exile" },
			},
		},
	},
	radiant = {
		cry = function(ctx: J.EffectContext)
			return cry(ctx)
		end,
		graveyardPlay = graveyard_play,
	},
}
