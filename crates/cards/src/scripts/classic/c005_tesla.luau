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

local zap = {
	id = "tesla-zap",
	on = { "summoned", "cardResolved" },
	when = function(ctx: J.EffectContext, event: J.GameEvent)
		return arrival(ctx, event) ~= nil
	end,
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		local target = arrival(ctx, event)
		if target == nil then
			return {}
		end
		local hit: J.TargetSpec = { of = "instance", instanceId = target }
		return {
			J.damage({ to = hit, amount = ctx:param("damage") }),
			J.animate({ position = "DEF" }),
		}
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
