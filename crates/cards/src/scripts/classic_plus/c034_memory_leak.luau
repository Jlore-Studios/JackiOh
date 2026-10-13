--!strict
local J = require("@jackioh")

local MODE = "mode"
local END_OF_TURN = "endOfTurn"
local AFTER_PLAY = "afterPlay"

-- The opponent's played card, resolved: a Unit or a Field Spell, and not ours (a cast counts,
-- R70). `ctx & { event }` is the context and the event, two arguments here.
local function opponent_played_unit_or_field_spell(ctx: J.EffectContext, event: J.GameEvent): boolean
	if event.type ~= "cardResolved" then
		return false
	end
	if event.player == ctx:controller() then
		return false
	end
	local type_ = ctx:def_of(event.defId).type
	return type_ == "Unit" or type_ == "Field Spell"
end

local function memory_leak(has: (ctx: J.EffectContext, mode: string) -> boolean)
	return {
		endOfTurn = function(ctx: J.EffectContext)
			if has(ctx, END_OF_TURN) then
				return {
					J.lock_random_zone({ side = "enemy" }),
				}
			end
			return {}
		end,
		triggers = {
			{
				id = "memory-leak",
				on = { "cardResolved" },
				-- Not a trap, so the condition is read in `run` (only traps consult `when`,
				-- R99).
				run = function(ctx: J.EffectContext, event: J.GameEvent)
					if has(ctx, AFTER_PLAY) and opponent_played_unit_or_field_spell(ctx, event) then
						return {
							J.lock_played_zone({ event = event }),
						}
					end
					return {}
				end,
			},
		},
	}
end

local function remembered_mode(ctx: J.EffectContext, mode: string): boolean
	return ctx:recalled(MODE) == mode
end

local function always(ctx: J.EffectContext, mode: string): boolean
	return true
end

local base_hooks = memory_leak(remembered_mode)
local radiant_hooks = memory_leak(always)

local base = {
	modes = {
		{ kind = "mode", options = { END_OF_TURN, AFTER_PLAY } },
	},
	cry = function(ctx: J.EffectContext)
		local mode = ctx:chosen_options()[1]
		if mode ~= END_OF_TURN and mode ~= AFTER_PLAY then
			return {}
		end
		return {
			J.remember({ key = MODE, value = mode }),
		}
	end,
	endOfTurn = base_hooks.endOfTurn,
	triggers = base_hooks.triggers,
}

local radiant = {
	endOfTurn = radiant_hooks.endOfTurn,
	triggers = radiant_hooks.triggers,
}

return {
	base = base,
	radiant = radiant,
}
