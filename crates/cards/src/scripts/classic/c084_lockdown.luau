--!strict
local J = require("@jackioh")

-- A trigger on `cardPlayed` Locks the zone the permanent entered. R119: not its own arrival; the
-- Radiant face answers only the opponent's plays.
local function lock_after_play(opponent_only: boolean)
	return {
		id = "lock-played-zone",
		on = { "cardPlayed" },
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			if event.type ~= "cardPlayed" then
				return {}
			end
			local me = ctx:self_()
			if me ~= nil and me.id == event.instanceId then
				return {}
			end
			if opponent_only and event.player == ctx:controller() then
				return {}
			end
			return { J.lock_played_zone({ event = event }) }
		end,
	}
end

-- Activate (R384), once per turn: Tribute this, paid as it activates; the effect list is empty.
local TRIBUTE = {
	id = "tribute",
	label = "Tribute this",
	uses = 1,
	cost = { tributeSelf = true },
	run = function(ctx: J.EffectContext)
		return {}
	end,
}

return {
	base = {
		triggers = { lock_after_play(false) },
		activations = { TRIBUTE },
	},
	radiant = {
		triggers = { lock_after_play(true) },
		activations = { TRIBUTE },
	},
}
