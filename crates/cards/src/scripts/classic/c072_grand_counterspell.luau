--!strict
local J = require("@jackioh")

-- The opponent's announce of anything but a Unit (§6.3 Counter, B5 E1).
local function answers(to: J.CounterDestination)
	return {
		id = "grand-counter",
		on = { "cardAnnounced" },
		when = function(ctx: J.EffectContext, event: J.GameEvent)
			return event.type == "cardAnnounced" and event.player ~= ctx:controller() and event.cardType ~= "Unit"
		end,
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			if event.type ~= "cardAnnounced" then
				return {}
			end
			local target: J.TargetSpec = { of = "instance", instanceId = event.instanceId }
			return {
				J.counter_play({ to = to, target = target }),
			}
		end,
	}
end

return {
	base = {
		triggers = { answers("graveyard") },
	},
	-- E2, R12: countered and taken into this trap's controller's hand, as theirs.
	radiant = {
		triggers = { answers("thief") },
	},
}
