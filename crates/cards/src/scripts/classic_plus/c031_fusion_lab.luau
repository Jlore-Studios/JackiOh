--!strict
local J = require("@jackioh")

-- The `targetChecks` predicate the ability's declared pick names: a card a Fuse may keep (R23).
local FUSABLE = "fusable"

-- B5 E23: a random card of every set but this one into the chosen hand card, keeping its cost.
-- R81: one card of the controller's own hand, declared with the activation.
local function fusion_lab(radiant: boolean)
	return {
		targetChecks = {
			[FUSABLE] = function(args)
				local candidate = args:candidate()
				return candidate ~= nil and not args:unit_has(candidate.id, "Immutable")
			end,
		},
		activations = {
			{
				id = "fuse",
				label = "Choose a card in your hand. Fuse a random card into it",
				uses = 1,
				targets = { { kind = "hand", min = 1, max = 1, filter = { check = FUSABLE } } },
				run = function(ctx: J.EffectContext)
					return {
						J.fuse_random_into({
							into = { target = { of = "chosen" } },
							radiant = radiant,
							keepCost = true,
						}),
					}
				end,
			},
		},
	}
end

return {
	base = fusion_lab(false),
	radiant = fusion_lab(true),
}
