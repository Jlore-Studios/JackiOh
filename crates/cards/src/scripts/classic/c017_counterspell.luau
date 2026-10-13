--!strict
local J = require("@jackioh")

-- "When your opponent plays a Spell": the announce of an opponent's Spell-type play or cast, as the
-- announced card's instance and definition ids, or nil.
local function opponents_spell(ctx: J.EffectContext, event: J.GameEvent): (string?, string?)
	if event.type ~= "cardAnnounced" then
		return nil, nil
	end
	if event.player == ctx:controller() then
		return nil, nil
	end
	if event.cardType == "Spell" then
		return event.instanceId, event.defId
	end
	return nil, nil
end

local function counter_it(ctx: J.EffectContext, instanceId: string, defId: string, copy: boolean)
	local target: J.TargetSpec = { of = "instance", instanceId = instanceId }
	local countered = J.counter_play({ target = target })
	if not copy then
		return { countered }
	end
	-- Read before the counter moves it: the copy keeps the countered card's face (R57) and, a copy of a
	-- Chinese card being Chinese, its language (R1300).
	local card = ctx:find_instance(instanceId)
	local found = type(card) == "table"
	local args: J.AddToHandArgs = {
		defId = defId,
		radiant = found and card.radiant == true,
		costOverride = ctx:param("setCost"),
	}
	if found and card.chinese == true then
		args.chinese = true
	end
	return { countered, J.add_to_hand(args) }
end

local function counterspell(copy: boolean)
	return {
		id = "counterspell",
		on = { "cardAnnounced" },
		when = function(ctx: J.EffectContext, event: J.GameEvent)
			return opponents_spell(ctx, event) ~= nil
		end,
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			local instanceId, defId = opponents_spell(ctx, event)
			if instanceId == nil or defId == nil then
				return {}
			end
			return counter_it(ctx, instanceId, defId, copy)
		end,
	}
end

return {
	base = {
		triggers = { counterspell(false) },
	},
	radiant = {
		triggers = { counterspell(true) },
	},
}
