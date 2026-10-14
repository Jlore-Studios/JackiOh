--!strict
local J = require("@jackioh")

-- A reader's answer, or nil: a reader with nothing to answer hands back a `null` that is not nil.
local function found(value: any): any
	if type(value) == "table" then
		return value
	end
	return nil
end

-- The Unit standing on this Tower now, if any: only a backrow zone carries one (R446).
local function rider_of(ctx: J.EffectContext, tower: any): any
	local at = found(ctx:slot_of(tower.id))
	if at == nil or at.row ~= "backrow" then
		return nil
	end
	return found(ctx:carried_at(at.player, at.lane))
end

-- R653: the event is the play of the Unit stacked onto this Tower, landing or resolved.
local function is_stacked_play(ctx: J.EffectContext, event: J.GameEvent): boolean
	local tower = found(ctx:live_self())
	if tower == nil then
		return false
	end
	local played: string? = nil
	if event.type == "cardPlayed" then
		played = event.instanceId
	elseif event.type == "cardResolved" then
		played = event.instanceId
	end
	if played == nil then
		return false
	end
	return ctx:stacked_onto(tower.id) == played
end

-- R653: once the stacked Unit's play has resolved, the Unit standing on this (that card, or what an
-- answer to the play left in its place) is fused into it. Not a trap, so the condition is read in
-- `run` (only traps consult `when`, R99).
local function fuse_in(radiant: boolean)
	return {
		id = "ivory-tower-fuse",
		on = { "cardResolved" },
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			if not is_stacked_play(ctx, event) then
				return {}
			end
			local tower = found(ctx:live_self())
			if tower == nil then
				return {}
			end
			local rider = rider_of(ctx, tower)
			if rider == nil then
				return {}
			end
			local args: J.FuseCardsArgs = { instanceIds = { rider.id }, targetInstanceId = tower.id }
			if radiant then
				args.radiantIngredients = true
			end
			return {
				J.fuse_cards(args),
			}
		end,
	}
end

-- The Radiant face's landing trigger: the stacked Unit becomes Radiant as it is played onto this.
local RADIANT_ON_LANDING = {
	id = "ivory-tower-radiant",
	on = { "cardPlayed" },
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		if event.type == "cardPlayed" and is_stacked_play(ctx, event) then
			return {
				J.set_radiant({ instanceId = event.instanceId }),
			}
		end
		return {}
	end,
}

local base = {
	staticFlags = { fusesCarried = true },
	triggers = { fuse_in(false) },
}

local radiant = {
	staticFlags = { fusesCarried = true },
	triggers = { RADIANT_ON_LANDING, fuse_in(true) },
}

return {
	base = base,
	radiant = radiant,
}
