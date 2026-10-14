--!strict
local J = require("@jackioh")

-- §7: the Sheep Token, whose only generator is this card.
local SHEEP_TOKEN = "core-t-sheep"
-- #55 Lava Golem, added by the radiant text at cost 0 (`costOverride`, R65): the declared number
-- `setCost` (R386).
local LAVA_GOLEM = "core-055"

-- A `run` returning `{}` still consumes the trap, so every condition lives in `when`.
local function when(ctx: J.EffectContext, event: J.GameEvent): boolean
	if event.type ~= "cardResolved" then
		return false
	end
	-- §8: "your opponent". A trap never answers its own controller's play.
	if event.player == ctx:controller() then
		return false
	end
	-- §5.1: a Unit, so a Spell or a backrow card leaves the trap armed (R61).
	if ctx:def_of(event.defId).type ~= "Unit" then
		return false
	end
	-- R174, R427: not a Unit an earlier answer to this play has already taken off the field.
	return event.permanent or not ctx:left_field_since_resolved(event)
end

-- The two faces differ only in whether the Lava Golem comes with the Sheep.
local function sheepish(lava_golem: boolean)
	return {
		id = if lava_golem then "sheepish-radiant" else "sheepish",
		on = { "cardResolved" },
		when = when,
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			if event.type ~= "cardResolved" then
				return {}
			end
			-- Named by the event, not by a TargetSpec: nobody chose this unit, the play produced it.
			-- R427: only a Unit still in play from that play is transformed.
			local effects = {}
			if event.permanent then
				effects[#effects + 1] = J.transform({ instanceId = event.instanceId, defId = SHEEP_TOKEN })
			end
			-- R120: §8's conventions make an "Also" clause independent, so it still lands when an
			-- Immutable target refused the Transform (R17, R23) and the trap is still consumed (R61).
			if lava_golem then
				effects[#effects + 1] = J.add_to_hand({ defId = LAVA_GOLEM, costOverride = ctx:param("setCost") })
			end
			return effects
		end,
	}
end

return {
	base = { triggers = { sheepish(false) } },
	radiant = { triggers = { sheepish(true) } },
}
