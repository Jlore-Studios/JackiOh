--!strict
local J = require("@jackioh")

-- §8.7: "Deal 1 damage" with no target named is targeted: any unit or hero, either side.
local TARGETS = {
	{ kind = "target", min = 1, max = 1, filter = { side = "any", of = { "unit", "hero" } } },
}

-- §8.7: "Activate" is once a turn, the Radiant face's "Activate 2" twice.
local BASE_USES = 1
local RADIANT_USES = 2

local function ping(uses: number)
	return {
		id = "ping",
		label = "Deal damage",
		uses = uses,
		targets = TARGETS,
		run = function(ctx: J.EffectContext)
			return { J.damage({ to = { of = "chosen" }, amount = ctx:param("damage") }) }
		end,
	}
end

return {
	base = { activations = { ping(BASE_USES) } },
	radiant = { activations = { ping(RADIANT_USES) } },
}
