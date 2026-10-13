--!strict
local J = require("@jackioh")

local TRAP_TYPES = { "Trap", "Field Trap" }

-- Your Traps and Field Traps cost (0).
local function aura(ctx: J.HookArgs)
	local rule: J.CostAura = {
		types = { "Trap", "Field Trap" },
		setTo = 0,
		whose = "yours",
	}
	return { rule }
end

-- After you play a Trap or a Field Trap (a cast is a play, R70), gain the card's mana. A permanent's
-- trigger reads its condition in `run` (a `when` predicate is a trap's, R99): any other play answers
-- with nothing.
local after_you_play_a_trap = {
	id = "toe-cracker-mana",
	on = { "cardPlayed" },
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		if event.type ~= "cardPlayed" then
			return {}
		end
		if event.player ~= ctx:controller() then
			return {}
		end
		local played = ctx:def_of(event.defId).type
		local is_trap = false
		for _, kind in ipairs(TRAP_TYPES) do
			if kind == played then
				is_trap = true
			end
		end
		if not is_trap then
			return {}
		end
		return { J.gain_mana({ amount = ctx:param("mana") }) }
	end,
}

return {
	base = { costAura = aura },
	radiant = {
		costAura = aura,
		triggers = { after_you_play_a_trap },
	},
}
