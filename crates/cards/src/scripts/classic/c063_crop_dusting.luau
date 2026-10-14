--!strict
local J = require("@jackioh")

local dusting = {
	id = "crop-dusting",
	on = { "turnStarted" },
	-- "At the start of your turn": its controller's `turnStarted`.
	when = function(ctx: J.EffectContext, event: J.GameEvent)
		return event.type == "turnStarted" and event.player == ctx:controller()
	end,
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		return {
			J.place_plague_each({
				scope = { side = "any", rows = { "units", "backrow" } },
				amount = ctx:param("tokens"),
			}),
			J.draw({ count = ctx:param("draw") }),
		}
	end,
}

-- The same script: the Radiant face's 3 counters and draw 3 are its declared numbers, which `param` reads
-- off the running face.
return {
	base = {
		triggers = { dusting },
	},
	radiant = "base",
}
