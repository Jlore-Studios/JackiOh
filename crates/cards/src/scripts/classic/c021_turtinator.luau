--!strict
local J = require("@jackioh")

-- Activate ♾️ (R384): the cost is one Tribute carried in the `activate` action, never Turtinator
-- itself (R683). The hit is the tributed unit's Attack as it stood when paid (R78, recorded by
-- `subsystems.activationPaid`) times the declared multiplier; an amount of 0 is no hit (R63).
local base = {
	activations = {
		{
			id = "eat",
			label = "Tribute another Unit. Deal damage equal to {multiplier}× its Attack",
			uses = "unlimited",
			cost = { tribute = 1, tributeExcludesSelf = true },
			targets = {
				{ kind = "target", min = 1, max = 1, filter = { of = { "unit", "hero" } } },
			},
			run = function(ctx: J.EffectContext)
				local tributed = ctx:subsystems_activation_paid().tributed[1]
				local attack = if tributed ~= nil then tributed.attack else 0
				local amount = attack * ctx:param("multiplier")
				if amount > 0 then
					return { J.damage({ to = { of = "chosen" }, amount = amount }) }
				end
				return {}
			end,
		},
	},
}

-- The same script: the Radiant face's "twice its Attack" is its declared multiplier (2), which
-- `param` reads off the running face.
return {
	base = base,
	radiant = "base",
}
