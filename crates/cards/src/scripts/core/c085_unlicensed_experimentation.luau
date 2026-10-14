--!strict
local J = require("@jackioh")

-- R61 and §5.1: "Field Trap counts as Trap", in both directions, so both read as one key.
local function type_key(card_type: string): string
	if card_type == "Field Trap" then
		return "Trap"
	end
	return card_type
end

local function holds(list: { string }, wanted: string): boolean
	for _, item in ipairs(list) do
		if item == wanted then
			return true
		end
	end
	return false
end

-- The permanent this play put on the opponent's field, or nil when the event is not one this trap
-- answers: the controller's own play, a Spell or anything else that is no longer in play (R61's
-- `permanent`), a token (R61), or a card the instance table can no longer name.
local function played_permanent(ctx: J.EffectContext, event: J.GameEvent): any
	if event.type ~= "cardResolved" then
		return nil
	end
	-- §8: "the opponent plays". A trap never answers its own controller's play.
	if event.player == ctx:controller() then
		return nil
	end
	-- R61: "played permanents only". Step 7 read this as it landed, so no board check is needed.
	if not event.permanent then
		return nil
	end
	local card = ctx:find_instance(event.instanceId)
	if card == nil then
		return nil
	end
	local played = ctx:def_of(card.defId)
	-- R61: "tokens … never set it off", including a token card played from a hand.
	if played.token or holds(played.tags, "Token") then
		return nil
	end
	return card
end

-- "a permanent of yours of that type" (§8), which R61 narrows twice: the firing trap is "neither
-- matched nor fused onto", and R13 leaves a card dormant under a Stack off the field. Never the
-- played card either (#52 Silly Silas rotated onto this side, R99). The controller's unit row, then
-- its backrow, lane by lane: `permanents_on_field` walks the controller's side first, in that order,
-- each card where its zone says it stands.
local function matching_permanents(ctx: J.EffectContext, card_type: string, played: any): { any }
	local wanted = type_key(card_type)
	local me = ctx:self_()
	local self_id = if me ~= nil then me.id else nil
	local controller = ctx:controller()
	local out: { any } = {}
	for _, card in ipairs(ctx:permanents_on_field(controller)) do
		if card.zone.player == controller and card.id ~= self_id and card.id ~= played.id then
			if type_key(ctx:def_of(card.defId).type) == wanted then
				out[#out + 1] = card
			end
		end
	end
	return out
end

-- `on_all` is the radiant face: R77's "onto each matching permanent separately".
local function experimentation(on_all: boolean)
	return {
		id = if on_all then "85r-unlicensed-experimentation" else "85-unlicensed-experimentation",
		on = { "cardResolved" },
		when = function(ctx: J.EffectContext, event: J.GameEvent)
			if event.type ~= "cardResolved" then
				return false
			end
			local played = played_permanent(ctx, event)
			if played == nil then
				return false
			end
			-- §8: "whose type matches one you control". R61 counts an Immutable permanent of yours
			-- here, so the trap fires and is consumed even though nothing can be fused onto it.
			local card_type = ctx:def_of(played.defId).type
			return #matching_permanents(ctx, card_type, played) > 0
		end,
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			if event.type ~= "cardResolved" then
				return {}
			end
			local played = played_permanent(ctx, event)
			if played == nil then
				return {}
			end
			-- R23: "Immutable permanents are never chosen" as the Fuse target.
			local card_type = ctx:def_of(played.defId).type
			local target_ids: { string } = {}
			for _, card in ipairs(matching_permanents(ctx, card_type, played)) do
				if not ctx:unit_has(card.id, "Immutable") then
					target_ids[#target_ids + 1] = card.id
				end
			end
			-- R61: with no legal target the trap fires, is consumed and does nothing, and the
			-- played permanent stays: an empty effect list.
			if #target_ids == 0 then
				return {}
			end
			-- R77, §6.3: one fusion per target, each its own transient definition, in one effect that
			-- holds the resolved ingredient across them; the base face's random pick is the rng
			-- inside that effect (§9.3, §10.7).
			return {
				J.fuse_cards({
					instanceIds = { played.id },
					targetInstanceIds = target_ids,
					pick = if on_all then "all" else "random",
				}),
			}
		end,
	}
end

-- R662: armed while a permanent of its controller's, other than this trap, could take a Fuse (not
-- Immutable, R23). Which type the opponent plays is theirs to choose, so the glow only says the Fuse
-- has somewhere to land, the same on both faces.
local function condition_met(ctx: J.ConditionContext): boolean
	return ctx:zone() == "field" and #ctx:fusable_permanents_of(ctx:controller(), ctx:self_().id) > 0
end

return {
	base = {
		triggers = { experimentation(false) },
		conditionMet = condition_met,
	},
	-- "Onto every such permanent" (R77: one fusion at a time, each target keeping its own instance).
	radiant = {
		triggers = { experimentation(true) },
		conditionMet = condition_met,
	},
}
