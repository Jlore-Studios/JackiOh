--!strict
local J = require("@jackioh")

-- One ability, once per turn, its mode carried in the `activate` action (R81).
local base = {
	activations = {
		{
			id = "thrive",
			label = "Choose one",
			uses = 1,
			modes = { { kind = "mode", options = { "heal", "draw", "mana" } } },
			run = function(ctx: J.EffectContext)
				local mode = ctx:chosen_options()[1]
				if mode == "heal" then
					local target: J.TargetSpec = { of = "selfHero" }
					return { J.heal({ target = target, amount = ctx:param("heal") }) }
				end
				if mode == "draw" then
					return { J.draw({ count = ctx:param("draw") }) }
				end
				if mode == "mana" then
					return { J.gain_mana({ amount = ctx:param("mana") }) }
				end
				return {}
			end,
		},
	},
}

-- The same script: the Radiant face's 6, 2 and 2 are its declared numbers.
return {
	base = base,
	radiant = "base",
}
