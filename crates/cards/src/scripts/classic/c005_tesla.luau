--!strict
local J = require("@jackioh")

-- The instance id of the opponent's Unit this event summons (or resolves, for a play), or nil.
local function arrival(ctx: J.EffectContext, event: J.GameEvent): string?
	if event.type == "summoned" then
		if event.player == ctx:controller() or event.row ~= "units" then
			return nil
		end
		-- A play's own placement (§10.5 step 4) is answered when the play resolves, below.
		if event.exitsFrom ~= nil then
			return nil
		end
		return event.instanceId
	end
	if event.type == "cardResolved" then
		if not event.permanent or ctx:def_of(event.defId).type ~= "Unit" then
			return nil
		end
		local unit = ctx:find_instance(event.instanceId)
		if type(unit) ~= "table" then
			return nil
		end
		if unit.zone.z ~= "field" or unit.controller == ctx:controller() then
			return nil
		end
		return event.instanceId
	end
	return nil
end

-- What Tesla keeps once it has revealed: it zaps only the Unit that revealed it (patch v0.3.9, #831).
local REVEALED = "revealed"

local function has_revealed(ctx: J.EffectContext): boolean
	return ctx:recalled(REVEALED) == true
end

local zap = {
	id = "tesla-zap",
	on = { "summoned", "cardResolved" },
	when = function(ctx: J.EffectContext, event: J.GameEvent)
		return not has_revealed(ctx) and arrival(ctx, event) ~= nil
	end,
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		local target = arrival(ctx, event)
		if target == nil or has_revealed(ctx) then
			return {}
		end
		-- R781001: a Field Trap can't target an Immutable Unit in play, so Tesla still fires and
		-- animates but deals it nothing.
		local me = ctx:live_self()
		local shielded = me ~= nil and ctx:immutable_bars(me.id, target)
		local effects: { J.EffectCall } = { J.remember({ key = REVEALED, value = true }) }
		if not shielded then
			local hit: J.TargetSpec = { of = "instance", instanceId = target }
			effects[#effects + 1] = J.damage({ to = hit, amount = ctx:param("damage") })
		end
		effects[#effects + 1] = J.animate({ position = "DEF" })
		return effects
	end,
}

-- The same script: the Radiant face's 8 is its declared damage, which `param` reads off the face;
-- its 2/8 body and keywords are printed on the catalog face.
return {
	base = {
		triggers = { zap },
	},
	radiant = "base",
}
