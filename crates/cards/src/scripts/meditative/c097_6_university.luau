--!strict
local J = require("@jackioh")

-- §8.8: "Activate" is once a turn.
local USES = 1

-- Activate (R384): separate Buffs (Upgrade, R386) on each other permanent acting on your side (unit
-- tops, backrow cards, face-down ones; this one left out since patch v0.3.9, #831), each application
-- its own draw; an Immutable card is left alone (R23). One script serves both faces: only the declared `times` (2, Radiant 5)
-- and the health differ.
local base = {
	activations = {
		{
			id = "study",
			label = "Buff your other permanents",
			uses = USES,
			run = function(ctx: J.EffectContext)
				return {
					J.upgrade({
						scope = { side = "self", zones = { "field" }, excludeSelf = true },
						times = ctx:param("times"),
					}),
				}
			end,
		},
	},
}

return {
	base = base,
	radiant = "base",
}
