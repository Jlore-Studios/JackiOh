--!strict
local J = require("@jackioh")

-- A declared attack (a forced one opens no window, R121) by an enemy Unit on this trap's controller's hero.
local function attacks_your_hero(ctx: J.EffectContext, event: J.GameEvent): boolean
	if event.type ~= "attackDeclared" then
		return false
	end
	if event.forced then
		return false
	end
	local you = ctx:controller()
	local attacker = ctx:find_instance(event.attackerId)
	if type(attacker) ~= "table" or attacker.controller ~= ctx:opponent_of(you) then
		return false
	end
	local target = ctx:attack_target_of(event.targetId)
	return type(target) == "table" and target.kind == "hero" and target.player == you
end

local function doom_shroom(side: J.ScopeSide)
	return {
		triggers = {
			{
				id = "doom-shroom",
				on = { "attackDeclared" },
				when = attacks_your_hero,
				run = function(ctx: J.EffectContext, event: J.GameEvent)
					-- Its Crater Locks this zone as it is revealed (R783011), before this runs.
					return {
						J.exile_all({ side = side }),
					}
				end,
			},
		},
	}
end

return {
	base = doom_shroom("any"),
	radiant = doom_shroom("enemy"),
}
