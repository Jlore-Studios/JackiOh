--!strict
local J = require("@jackioh")

-- §8.7: "their 3rd card", Radiant "their 2nd". An AI card declares no params. Each face closes
-- over its own count, so the two faces keep their own adapters.
local function rate_limit(nth: number): J.TriggerDef
	return {
		id = "rate-limit",
		on = { "cardPlayed" },
		when = function(ctx: J.EffectContext, event: J.GameEvent): boolean
			if event.type ~= "cardPlayed" then
				return false
			end
			return event.player ~= ctx:controller()
				and ctx:turn().active == event.player
				and ctx:cards_played_this_turn(event.player) == nth
		end,
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			return {
				J.end_turn({ player = "enemy" }),
			}
		end,
	}
end

return {
	base = {
		triggers = { rate_limit(3) },
	},
	radiant = {
		triggers = { rate_limit(2) },
	},
}
