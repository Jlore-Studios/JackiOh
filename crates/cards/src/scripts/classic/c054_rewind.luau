--!strict
local J = require("@jackioh")

-- `triggerCry` is the engine's whole sequence: this card only declares the pick (a `check` on
-- `hasTriggerableCry`) and runs it `param(ctx, "repeats")` times on the chosen Unit.
local function rewind(side: string)
	return {
		targets = {
			{
				kind = "target",
				min = 1,
				max = 1,
				filter = { side = side, of = { "unit", "graveyard" }, check = "hasCry" },
			},
		},
		targetChecks = {
			hasCry = function(args)
				local candidate = args:candidate()
				return candidate ~= nil and args:has_triggerable_cry(candidate.id)
			end,
		},
		cry = function(ctx: J.EffectContext)
			local effects = {}
			for _ = 1, ctx:param("repeats") do
				effects[#effects + 1] = J.trigger_cry({})
			end
			return effects
		end,
	}
end

-- Base: one of your Units; Radiant: any Unit, either side.
return {
	base = rewind("ally"),
	radiant = rewind("any"),
}
