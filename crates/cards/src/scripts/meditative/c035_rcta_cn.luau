--!strict
local J = require("@jackioh")

-- A declared non-CN permanent on either side (`aim: help`, `required`, R81); a face-down card its
-- chooser cannot read is always offered (MD-B16, R924). Both faces share the one script: the
-- declared `stats` (2, radiant 5) and `times` (1, radiant 2) are what differ.
local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = {
				side = "any",
				of = { "unit", "backrow" },
				notTags = { "CN" },
			},
			aim = "help",
			required = true,
		},
	},
	cry = function(ctx: J.EffectContext)
		local target = ctx:instance_of({ of = "chosen" })
		if target == nil then
			return {}
		end
		-- A target that is CN by the time the Spell resolves fails its filter (§10.5): the
		-- effect fizzles on it (MD-B16, R924).
		for _, tag in ipairs(ctx:tags_of(target.id)) do
			if tag == "CN" then
				return {}
			end
		end
		local id = target.id
		local stats = ctx:param("stats")
		local times = ctx:param("times")
		local it: J.TargetSpec = { of = "instance", instanceId = id }
		return {
			J.grant_tag({ instanceId = id, tag = "CN" }),
			J.translate({ instanceId = id }),
			J.buff({ target = it, attack = stats, health = stats }),
			J.upgrade({ target = it, times = times }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
