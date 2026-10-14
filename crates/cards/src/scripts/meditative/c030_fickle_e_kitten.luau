--!strict
local J = require("@jackioh")

-- M #30.1 Love Bomb, the token the "Otherwise" clause shuffles (§7).
local LOVE_BOMB = "meditative-030-1"

-- "Shuffle a Love Bomb into your deck".
local BOMBS = 1

-- Whether the opponent's most expensive permanent beats the controller's (R901, MD-B5), on the
-- Radiant face only when their deck is also larger. A side with no permanent has no highest cost.
local function opponent_outbids(ctx: J.EffectContext, controller: J.PlayerId, needs_larger_deck: boolean): boolean
	local foe = ctx:opponent_of(controller)
	local theirs: number? = ctx:highest_permanent_cost(foe)
	local ours: number? = ctx:highest_permanent_cost(controller)
	if theirs == nil or ours == nil then
		return false
	end
	if theirs <= ours then
		return false
	end
	if needs_larger_deck and ctx:zone_count(foe, "library") <= ctx:zone_count(controller, "library") then
		return false
	end
	return true
end

-- Start of its controller's turn (§2.2, R62's queue): when the opponent outbids, they gain control
-- of this (R1423: R15's placement, R171's entry; with no free unit zone it stays and the
-- "Otherwise" clause does not run); otherwise a Love Bomb, Radiant on the Radiant face, goes into
-- its controller's deck.
local function kitten(radiant: boolean)
	return {
		startOfTurn = function(ctx: J.EffectContext)
			local me = ctx:live_self()
			if me == nil then
				return {}
			end
			if opponent_outbids(ctx, me.controller, radiant) then
				return { J.give_control({ instanceId = me.id }) }
			end
			return {
				J.shuffle_into({ defId = LOVE_BOMB, count = BOMBS, player = "self", radiant = radiant }),
			}
		end,
	}
end

return {
	base = kitten(false),
	radiant = kitten(true),
}
