--!strict
local J = require("@jackioh")

-- Patch v0.3.9 (#831): the Activate arms one play from the graveyard this turn (R831015), the turn
-- it was armed on remembered here; the play that uses it, or the turn's end, disarms it.
local ARMED = "armed"

-- The memory the graveyard permission reads: armed this very turn.
local function armed(ctx: J.HookArgs): boolean
	local me = ctx:self_()
	return type(me) == "table" and me.memory ~= nil and me.memory[ARMED] == ctx:turn().number
end

local base = {
	-- Cry: place the declared `tokens` on this (R386); a Field Spell's Cry fires as it is played or
	-- cast (R1).
	cry = function(ctx: J.EffectContext)
		return {
			J.place_plague({ target = { of = "self" }, amount = ctx:param("tokens") }),
		}
	end,
	-- Activate, once per turn (R384): arm the permission below for one play.
	activations = {
		{
			id = "plant",
			label = "Play a Unit from your graveyard",
			uses = 1,
			run = function(ctx: J.EffectContext)
				return {
					J.remember({ key = ARMED, value = ctx:turn().number }),
				}
			end,
		},
	},
	-- A Unit from your graveyard, paid with Plague Counters (§6.3 Play), while armed.
	graveyardPlay = function(ctx: J.HookArgs)
		if not armed(ctx) then
			return {}
		end
		local permission: J.GraveyardPlayPermission = { units = true, plague = true }
		return { permission }
	end,
	-- The play from your graveyard that the arming allowed spends it. A field trigger's `when` is not
	-- read (only a trap's reveal condition is), so `run` asks it.
	triggers = {
		{
			id = "plantation-spent",
			on = { "cardPlayed" },
			run = function(ctx: J.EffectContext, event: J.GameEvent)
				if
					event.type == "cardPlayed"
					and event.player == ctx:controller()
					and event.from == "graveyard"
					and ctx:recalled(ARMED) == ctx:turn().number
				then
					return { J.remember({ key = ARMED, value = 0 }) }
				end
				return {}
			end,
		},
	},
}

-- The same script: the Radiant face's 4 tokens are its declared `tokens`.
return {
	base = base,
	radiant = "base",
}
