--!strict
local J = require("@jackioh")

-- Combo 1 as its Cry (§6.2): the card played just before this one this turn (R1081), casts counted
-- (R70), found wherever it is (R98).
local function critikal(ctx: J.EffectContext, radiant: boolean)
	local me = ctx:controller()
	-- One or more cards played earlier this turn, read at play time.
	local at = ctx:played_earlier(me, ctx:live_self())
	if at < 1 then
		return {}
	end
	local prev = ctx:played_ids_this_turn(me)[at]
	if prev == nil then
		return {}
	end
	local card = ctx:find_instance(prev)
	if card == nil then
		return {}
	end
	local where = card.zone.z
	-- From your graveyard or exile it moves to your hand ("Return … to hand", R746).
	if (where == "graveyard" or where == "exile") and card.zone.player == me then
		local instance: J.TargetSpec = { of = "instance", instanceId = card.id }
		return {
			J.add_to_hand({ instance = instance, radiant = radiant }),
		}
	end
	-- A permanent you control is Bounced (R692, R747); the Radiant face makes it Radiant first, so
	-- the Bounce carries it home. Anywhere else nothing happens.
	if where == "field" and card.controller == me then
		local target: J.TargetSpec = { of = "instance", instanceId = card.id }
		if radiant then
			return {
				J.set_radiant({ instanceId = card.id }),
				J.bounce({ target = target }),
			}
		end
		return {
			J.bounce({ target = target }),
		}
	end
	return {}
end

-- R195: hand only, the same read as the Cry, so the glow and the return cannot disagree.
local function combo_ready(ctx: J.ConditionContext): boolean
	return ctx:zone() == "hand" and ctx:played_earlier(ctx:controller(), ctx:self_()) >= 1
end

local base = {
	cry = function(ctx: J.EffectContext)
		return critikal(ctx, false)
	end,
	conditionMet = combo_ready,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return critikal(ctx, true)
	end,
	conditionMet = combo_ready,
}

return {
	base = base,
	radiant = radiant,
}
