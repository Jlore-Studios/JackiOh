--!strict
local J = require("@jackioh")

-- The other seat. `ConditionContext` binds no `opponent_of`, and a game has two seats.
local function opponent(player: J.PlayerId): J.PlayerId
	if player == "p1" then
		return "p2"
	end
	return "p1"
end

-- §10.9: a hook may READ state to compute an effect's arguments; it never writes. The two library
-- sizes come through the read-only `zone_count` (BUILD M3-T1), "yours" being the trap's controller
-- (R62, R52); "more cards than" is strictly greater, so an equal count does nothing.
local function library_is_larger(mine: number, theirs: number): boolean
	return mine > theirs
end

local function library_is_larger_now(ctx: J.EffectContext): boolean
	local me = ctx:controller()
	return library_is_larger(ctx:zone_count(me, "library"), ctx:zone_count(ctx:opponent_of(me), "library"))
end

-- The cost limit is the whole of the radiant text: the declared number `costLimit` (R386), read off
-- the face that is up.
local base = {
	triggers = {
		{
			id = "intern-stimmy-window",
			on = { "turnEnded" },
			when = function(ctx: J.EffectContext, event: J.GameEvent)
				return library_is_larger_now(ctx)
			end,
			run = function(ctx: J.EffectContext, event: J.GameEvent)
				if library_is_larger_now(ctx) then
					-- R65: outside play an X-cost card counts as 0 and an embiggen card as its base price, which
					-- is what `recruit`'s filter reads off the library (`query_cost` in effects/summon.rs).
					local filter: J.RecruitFilter = { type = "Unit", costRange = { max = ctx:param("costLimit") } }
					return {
						J.recruit({ filter = filter }),
					}
				end
				return {}
			end,
		},
	},
	-- R195: hand and field alike — the condition is the board's, not the play's.
	conditionMet = function(ctx: J.ConditionContext)
		local me = ctx:controller()
		return library_is_larger(ctx:zone_count(me, "library"), ctx:zone_count(opponent(me), "library"))
	end,
}

return {
	base = base,
	radiant = "base",
}
