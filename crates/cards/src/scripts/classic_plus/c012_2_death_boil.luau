--!strict
local J = require("@jackioh")

-- Which clause applies is read at resolution from who controls the chosen target; gone, nothing.
local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit", "hero" } },
		},
	},
	cry = function(ctx: J.EffectContext)
		local amount = ctx:param("amount")
		local chosen: J.TargetSpec = { of = "chosen" }
		local target = ctx:resolve_target(chosen)
		if type(target) ~= "table" then
			return {}
		end
		local controller = if target.kind == "hero" then target.player else target.instance.controller
		if controller == ctx:controller() then
			return { J.heal({ target = chosen, amount = amount }) }
		end
		return { J.damage({ to = chosen, amount = amount }) }
	end,
}

-- The Radiant face is the same text at 12, a catalog value.
return {
	base = base,
	radiant = "base",
}
