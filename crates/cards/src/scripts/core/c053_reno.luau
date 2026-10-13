--!strict
local J = require("@jackioh")

-- "If your hero is below `floor`": strictly below, read through the engine's `hero_of`.
local function below_floor(health: number, floor: number): boolean
	return health < floor
end

-- The two faces differ only in the number, so one script serves both (§8 Conventions): the floor the
-- hero is raised to is the declared number `health` (R386), 30 and 60 on the Radiant face.
local base = {
	cry = function(ctx: J.EffectContext)
		local floor = ctx:param("health")
		if below_floor(ctx:hero_of(ctx:controller()).health, floor) then
			local target: J.TargetSpec = { of = "selfHero" }
			return {
				J.heal({ target = target, upTo = floor }),
			}
		end
		return {}
	end,
	-- R195: hand only — the glow asks whether playing Reno now would set the hero to the floor.
	conditionMet = function(ctx: J.ConditionContext)
		return ctx:zone() == "hand" and below_floor(ctx:hero_of(ctx:controller()).health, ctx:param("health"))
	end,
}

return {
	base = base,
	radiant = "base",
}
