--!strict
local J = require("@jackioh")

local ID = "meditative-074"

-- "Costs ({discount}) less for each card in your hand", as C+ #64 Mulch Muncher's cost (R65, floor
-- 0), counting the hand itself included (MD-D33, R1065). The price is read off the catalog's printed
-- cost; anywhere it is not played from, it is that printed cost.
local function giant_cost(args: J.CostArgs): number
	local printed = J.cost(ID)
	local me = args:instance()
	local at = me.zone
	local for_play = at.z == "hand" or (at.z == "graveyard" and args:playable_from_graveyard(me.id))
	if not for_play then
		return printed
	end
	return math.max(printed - args:param("discount") * args:zone_count(at.player, "hand"), 0)
end

-- At its controller's end of turn with a full hand: the base face makes itself Radiant (§5.2), the
-- Radiant face gains +`buff`/+`buff`.
local function full_hand(ctx: J.EffectContext): boolean
	return ctx:zone_count(ctx:controller(), "hand") >= J.config.HAND_CAP
end

local base = {
	cost = giant_cost,
	endOfTurn = function(ctx: J.EffectContext)
		if not full_hand(ctx) then
			return {}
		end
		return {
			J.set_radiant({ target = { of = "self" } }),
		}
	end,
}

local radiant = {
	cost = giant_cost,
	endOfTurn = function(ctx: J.EffectContext)
		if not full_hand(ctx) then
			return {}
		end
		local gain = ctx:param("buff")
		return {
			J.buff({ target = { of = "self" }, attack = gain, health = gain }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
