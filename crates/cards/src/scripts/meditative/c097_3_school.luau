--!strict
local J = require("@jackioh")

-- §8.8: "Activate" is once a turn.
local USES = 1

-- Activate (R384): one uniform pick of the non-token Units of every set at that printed cost (R380,
-- R60), placed per R64, with no Cry (R1); no zone to land in draws nothing (R129). The cost is the
-- declared `cost`.
local function class(radiant: boolean)
	return {
		id = "class",
		label = "Summon a Unit",
		uses = USES,
		run = function(ctx: J.EffectContext)
			return {
				J.summon_random({
					query = { type = "Unit", cost = ctx:param("cost") },
					radiant = radiant,
				}),
			}
		end,
	}
end

return {
	base = { activations = { class(false) } },
	radiant = { activations = { class(true) } },
}
