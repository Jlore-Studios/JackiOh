--!strict
local J = require("@jackioh")

local targets = {
	{
		kind = "target",
		min = 1,
		max = 1,
		filter = { side = "any", of = { "unit" } },
	},
}

-- The chosen Unit's health now, before the destroy marks it; 0 when it is gone (nothing then).
local function health_of_chosen(ctx: J.EffectContext): number
	local unit = ctx:instance_of({ of = "chosen" })
	if type(unit) ~= "table" then
		return 0
	end
	return math.max(ctx:unit_view(unit.id).health, 0)
end

local base = {
	targets = targets,
	cry = function(ctx: J.EffectContext)
		return {
			J.destroy({ target = { of = "chosen" } }),
			J.damage({ to = { of = "selfHero" }, amount = health_of_chosen(ctx) }),
		}
	end,
}

local radiant = {
	targets = targets,
	cry = function(ctx: J.EffectContext)
		local health = health_of_chosen(ctx)
		local effects = { J.destroy({ target = { of = "chosen" } }) }
		if health > 0 then
			local hero: J.TargetSpec = { of = "selfHero" }
			effects[#effects + 1] = J.heal({ target = hero, amount = health })
		end
		return effects
	end,
}

return {
	base = base,
	radiant = radiant,
}
