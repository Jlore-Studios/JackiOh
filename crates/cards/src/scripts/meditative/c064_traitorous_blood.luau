--!strict
local J = require("@jackioh")

-- A declared, unforced attack (a forced one never opens the window, M10) by one of the controller's
-- enemies that has somewhere to go.
local function enemy_declared_with_neighbour(ctx: J.EffectContext, event: J.GameEvent): boolean
	if event.type ~= "attackDeclared" then
		return false
	end
	if event.forced then
		return false
	end
	local attacker = ctx:find_instance(event.attackerId)
	if attacker == nil then
		return false
	end
	if attacker.controller ~= ctx:opponent_of(ctx:controller()) then
		return false
	end
	return #ctx:redirect_neighbours(attacker.id) > 0
end

-- The attack half of §6.3 Redirect (MD-D19, R1122): the attack is re-aimed at one of the attacker's
-- neighbours, picked at random (M10); the Radiant face then summons a copy of each Unit that combat
-- destroyed (MD-D20, R1123).
local function traitorous_blood(copyDestroyed: boolean)
	return {
		triggers = {
			{
				id = "traitorousBlood",
				on = { "attackDeclared" },
				when = enemy_declared_with_neighbour,
				run = function(ctx: J.EffectContext, event: J.GameEvent)
					if event.type ~= "attackDeclared" then
						return {}
					end
					local attacker = ctx:find_instance(event.attackerId)
					if attacker == nil then
						return {}
					end
					local to = ctx:rng_pick(ctx:redirect_neighbours(attacker.id))
					if to == nil then
						return {}
					end
					return { J.redirect_attack({ to = to.id, copyDestroyed = copyDestroyed }) }
				end,
			},
		},
	}
end

return {
	base = traitorous_blood(false),
	radiant = traitorous_blood(true),
}
