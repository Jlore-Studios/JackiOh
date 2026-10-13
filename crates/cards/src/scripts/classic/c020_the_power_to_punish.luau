--!strict
local J = require("@jackioh")

-- The options, word for word: `legal_actions` offers these strings and the run matches on them.
local DAMAGE_MODE = "deal damage"
local DISCARD_MODE = "opponent discards"
local DOOM_MODE = "destroy a Unit at the start of your next turn"
local DOOM_ALL_MODE = "destroy all enemy Units at the start of your next turn"

-- R81: each target travels in the `activate` action, bound to its mode (`forModes`, R90).
local DAMAGE_TARGET = {
	kind = "target",
	min = 1,
	max = 1,
	filter = { of = { "unit", "hero" } },
	forModes = { DAMAGE_MODE },
}

local DOOM_TARGET = {
	kind = "target",
	min = 1,
	max = 1,
	filter = { of = { "unit" } },
	forModes = { DOOM_MODE },
}

-- The unit marked for death wears #50 K-Pop Fanatic's aura, in red (R437; Radiant, every enemy Unit).
local MARK = { mark = "destroy", color = "red" }

-- The mode the activation carried, resolved on the face that runs it.
local function punish(ctx: J.EffectContext, radiant: boolean): { J.EffectCall }
	local mode = ctx:chosen_options()[1]
	if mode == DAMAGE_MODE then
		return { J.damage({ to = { of = "chosen" }, amount = ctx:param("damage") }) }
	end
	if mode == DISCARD_MODE then
		return { J.discard_random({ count = ctx:param("discards"), player = "enemy" }) }
	end
	if mode == DOOM_MODE then
		if radiant then
			return {}
		end
		local doom: J.DestroyAtNextTurnStartArgs = { target = { of = "chosen" }, mark = MARK }
		return { J.destroy_at_next_turn_start(doom) }
	end
	if mode == DOOM_ALL_MODE then
		if radiant then
			local doom: J.DestroyAtNextTurnStartArgs = { scope = { side = "enemy" }, mark = MARK }
			return { J.destroy_at_next_turn_start(doom) }
		end
		return {}
	end
	return {}
end

local base = {
	activations = {
		{
			id = "punish",
			label = "Choose one",
			uses = 1,
			targets = { DAMAGE_TARGET, DOOM_TARGET },
			modes = { { kind = "mode", options = { DAMAGE_MODE, DISCARD_MODE, DOOM_MODE } } },
			run = function(ctx: J.EffectContext)
				return punish(ctx, false)
			end,
		},
	},
}

local radiant = {
	activations = {
		{
			id = "punish",
			label = "Choose one",
			uses = 1,
			targets = { DAMAGE_TARGET },
			modes = { { kind = "mode", options = { DAMAGE_MODE, DISCARD_MODE, DOOM_ALL_MODE } } },
			run = function(ctx: J.EffectContext)
				return punish(ctx, true)
			end,
		},
	},
}

return {
	base = base,
	radiant = radiant,
}
