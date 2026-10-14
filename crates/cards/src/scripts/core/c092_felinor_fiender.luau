--!strict
local J = require("@jackioh")

-- §5, §8: the tag that makes a unit one of "your Felinors".
local FELINOR = "Felinor"

-- "Has the stats of all your Felinors": the sum once (R39); "twice the stats": the sum twice (R362).
local BASE_MULTIPLE = 1
local RADIANT_MULTIPLE = 2

-- Whether `tags` holds the Felinor tag (MD-B15, R923: a granted one counts).
local function is_felinor(tags: { string }): boolean
	for _, tag in ipairs(tags) do
		if tag == FELINOR then
			return true
		end
	end
	return false
end

-- "All your Felinors, including ones under a Stack": every Felinor this card's controller has on the
-- field, dormant ones included (R13, §3.2), never this card itself (R131).
local function felinors_of(ctx: J.HookArgs): { any }
	local me = ctx:self_()
	local mine = {}
	for _, unit in ipairs(ctx:active_units_of(me.controller)) do
		mine[#mine + 1] = unit
	end
	for _, unit in ipairs(ctx:dormant_units_of(me.controller)) do
		mine[#mine + 1] = unit
	end
	local felinors = {}
	for _, unit in ipairs(mine) do
		if unit.id ~= me.id and is_felinor(ctx:tags_of(unit.id)) then
			felinors[#felinors + 1] = unit
		end
	end
	return felinors
end

-- §10.4 layer 2, which ADDS this result to layer 1's printed face, so this returns only the sum (R116).
-- Each Felinor contributes its layer-4 stats (`statsWithBuffs`): printed plus permanent buffs, before
-- auras, which is also what stops the layers recursing. R39's "never below printed" is the floor at 0.
local function set_stat_times(multiple: number)
	return function(ctx: J.HookArgs)
		local attack = 0
		local max_health = 0
		for _, felinor in ipairs(felinors_of(ctx)) do
			local stats = ctx:stats_with_buffs(felinor.id)
			attack += stats.attack
			max_health += stats.maxHealth
		end
		return {
			attack = math.max(attack, 0) * multiple,
			maxHealth = math.max(max_health, 0) * multiple,
		}
	end
end

return {
	base = { setStat = set_stat_times(BASE_MULTIPLE) },
	radiant = { setStat = set_stat_times(RADIANT_MULTIPLE) },
}
