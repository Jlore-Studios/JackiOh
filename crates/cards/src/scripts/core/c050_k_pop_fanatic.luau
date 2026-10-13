--!strict
local J = require("@jackioh")

-- The step `state.delayed` re-enters at the start of this player's next turn (§10.6, R62).
local STEAL_STEP = "steal"

-- The one thing the continuation captures: which permanent was chosen (§8.2 Engine cell).
local TARGET_KEY = "targetId"

-- R437: the pending steal, as the mark its target carries while it waits — purple.
local STEAL_MARK = "steal"
local STEAL_COLOR = "purple"

-- The captured id, narrowed rather than cast: `data` is JSON that crossed a turn boundary.
local function captured_target_id(ctx: J.EffectContext): string?
	local data = ctx:data()
	if type(data) ~= "table" then
		return nil
	end
	local target = data[TARGET_KEY]
	if type(target) ~= "string" then
		return nil
	end
	return target
end

-- R13: a card on the field on top of its pile — the only card a steal can take. In a unit zone
-- the pile tops leave out a card dormant under a Stack pile (§3.2); in a backrow zone the acting
-- card leaves out a card dormant beneath it in its backrow pile.
local function on_top_of_its_pile(ctx: J.EffectContext, card: any): boolean
	if type(card) ~= "table" then
		return false
	end
	local zone = card.zone
	if type(zone) ~= "table" or zone.z ~= "field" then
		return false
	end
	if zone.row == "units" then
		for _, top in ipairs(ctx:active_units_of(card.controller)) do
			if top.id == card.id then
				return true
			end
		end
		return false
	end
	if zone.row == "backrow" then
		-- One acting card per zone: a card dormant beneath it in its backrow pile is not the top.
		for _, other in ipairs(ctx:permanents_on_field(zone.player)) do
			local at = other.zone
			if at.z == "field" and at.row == "backrow" and at.player == zone.player and at.lane == zone.lane then
				return other.id == card.id
			end
		end
		return false
	end
	return false
end

-- R282, before the steal: the card stands on the field under the other player, so it can be taken.
local function takeable(ctx: J.EffectContext, id: string): boolean
	local card = ctx:find_instance(id)
	if type(card) ~= "table" then
		return false
	end
	return on_top_of_its_pile(ctx, card) and card.controller ~= ctx:controller()
end

-- R282, after the steal: the card stands on the field under this player.
local function held_now(ctx: J.EffectContext, id: string): boolean
	local card = ctx:find_instance(id)
	if type(card) ~= "table" then
		return false
	end
	return on_top_of_its_pile(ctx, card) and card.controller == ctx:controller()
end

-- R62's continuation, registered once in the `resume` table (R126). R76's fizzles are steal's own
-- no-ops: a target gone from the field, or one already yours.
local function steal_step(ctx: J.EffectContext): { J.EffectCall }
	local target_id = captured_target_id(ctx)
	if target_id == nil then
		return {}
	end
	return {
		J.steal({ instanceId = target_id }),
	}
end

-- The radiant face's step: the same steal, then R282's rider for the card the steal took and
-- nothing else. `takeable` is read as the list is built, before the steal; `for_each_card` reads
-- its set when the list reaches it, after, so the set is the stolen card or nothing.
local function radiant_steal_step(ctx: J.EffectContext): { J.EffectCall }
	local target_id = captured_target_id(ctx)
	if target_id == nil then
		return {}
	end
	local effects: { J.EffectCall } = {
		J.steal({ instanceId = target_id }),
	}
	if not takeable(ctx, target_id) then
		return effects
	end
	effects[#effects + 1] = J.for_each_card({ cards = "each_cards", each = "each_card" })
	return effects
end

-- The Cry both faces share: choose the enemy permanent and arm the steal (§8.2 Engine cell). The
-- play-time pick is read as the engine aims every chosen target (`instance_of`, R174); the delay
-- is keyed to the target, not this unit, so it fires even if K-Pop Fanatic died (R76, R127).
local function cry(ctx: J.EffectContext): { J.EffectCall }
	local target = ctx:instance_of({ of = "chosen" })
	if type(target) ~= "table" then
		return {}
	end
	local target_id = target.id
	return {
		J.delay({
			at = { phase = "start", player = "self" },
			step = STEAL_STEP,
			hook = "resume",
			data = { [TARGET_KEY] = target_id },
			-- R174: a target that leaves the field before the steal fires is gone for good, even
			-- if the same card is back by then — R76's fizzle.
			watch = target_id,
			-- R437: and it carries the steal's mark in both views while it waits.
			mark = { mark = STEAL_MARK, color = STEAL_COLOR },
		}),
	}
end

-- §6.3: a permanent is a Unit, Field Spell, Trap or Field Trap, so both enemy rows are
-- offered.
local TARGETS = {
	{
		kind = "target",
		min = 1,
		max = 1,
		filter = { side = "enemy", of = { "unit", "backrow" } },
	},
}

-- The face the Cry ran is the one stored with the delay, whatever the Fanatic shows when it
-- fires: each face registers its own step. Only the radiant face names the rider's `for_each_card`
-- pair.
local base = {
	cry = cry,
	targets = TARGETS,
	resume = {
		[STEAL_STEP] = steal_step,
	},
}

local radiant = {
	cry = cry,
	targets = TARGETS,
	resume = {
		[STEAL_STEP] = radiant_steal_step,
	},
	each_cards = function(ctx: J.EffectContext)
		local target_id = captured_target_id(ctx)
		if target_id == nil then
			return {}
		end
		if held_now(ctx, target_id) then
			return { target_id }
		end
		return {}
	end,
	each_card = function(id: string)
		return { J.set_radiant({ instanceId = id }) }
	end,
}

return {
	-- "Divine Shield" is printed in the catalog; the step adds R282's rider.
	base = base,
	radiant = radiant,
}
