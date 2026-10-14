--!strict
local J = require("@jackioh")

-- One ability, once per turn, its mode carried in the `activate` action (R81). The third mode reduces
-- the Cost of a card in your hand (patch v0.3.9, #831; it gained mana before): the card is chosen as
-- the activation resolves, a prompt the `resume` table answers (R831009).
local base = {
	activations = {
		{
			id = "thrive",
			label = "Choose one",
			uses = 1,
			modes = { { kind = "mode", options = { "heal", "draw", "reduce" } } },
			run = function(ctx: J.EffectContext)
				local mode = ctx:chosen_options()[1]
				if mode == "heal" then
					local target: J.TargetSpec = { of = "selfHero" }
					return { J.heal({ target = target, amount = ctx:param("heal") }) }
				end
				if mode == "draw" then
					return { J.draw({ count = ctx:param("draw") }) }
				end
				if mode == "reduce" then
					return {
						J.choose_from_hand({
							step = "reduce",
							count = 1,
							prompt = "The Power to Thrive: choose a card",
						}),
					}
				end
				return {}
			end,
		},
	},
	resume = {
		reduce = function(ctx: J.EffectContext)
			return {
				J.set_cost_mod({ target = { of = "chosen" }, amount = -ctx:param("discount"), inHandOnly = true }),
			}
		end,
	},
}

-- The same script: the Radiant face's 6, 2 and 2 are its declared numbers.
return {
	base = base,
	radiant = "base",
}
