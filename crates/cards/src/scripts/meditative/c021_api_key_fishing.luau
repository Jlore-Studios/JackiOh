--!strict
local J = require("@jackioh")

local ID = "meditative-021"

-- A percent roll is out of a hundred.
local PERCENT = 100

-- §5.1 and R68, as Core #23 Reoccurring Dream reads them: at the end of the turn it was played on,
-- the spell adds its copy from the graveyard — flagged for the return, or in this turn's play log.
local function returns_this_turn(ctx: J.EffectContext): boolean
	local me = ctx:live_self()
	if me == nil then
		return false
	end
	if me.returnToHandAtEndOfTurn == true then
		return true
	end
	return ctx:was_played_this_turn(me.controller, me.id)
end

-- The same script: the Radiant face differs only in its declared numbers, read through `param`.
local base = {
	cry = function(ctx: J.EffectContext)
		-- R129: with no hand to steal, there is nothing to roll for.
		if ctx:zone_count(ctx:opponent_of(ctx:controller()), "hand") == 0 then
			return {}
		end
		local key = if ctx:embiggened() then "paidChance" else "chance"
		if ctx:rng_chance(ctx:param(key), PERCENT) then
			return {
				J.give_from_hand({ from = "enemy", cards = "all" }),
			}
		end
		return {}
	end,
	endOfTurn = function(ctx: J.EffectContext)
		if returns_this_turn(ctx) then
			return {
				J.add_to_hand({ defId = ID }),
			}
		end
		return {}
	end,
}

return {
	base = base,
	radiant = "base",
}
