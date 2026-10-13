--!strict
local J = require("@jackioh")

-- §8.7: the Radiant face's "Draw 1". An AI card declares no params (B8).
local RADIANT_DRAW = 1

-- "One of your Units": whether any of the announced ids is a Unit of yours on the field.
local function your_unit(ctx: J.EffectContext, you: J.PlayerId, ids: { string }): boolean
	local units = ctx:active_units_of(you)
	for _, id in ipairs(ids) do
		for _, unit in ipairs(units) do
			if unit.id == id then
				return true
			end
		end
	end
	return false
end

-- "You or one of your cards": whether any of the announced ids is your hero, or a card you control on
-- the field or hold in your hand.
local function you_or_yours(ctx: J.EffectContext, you: J.PlayerId, ids: { string }): boolean
	for _, id in ipairs(ids) do
		if id == "hero-" .. you then
			return true
		end
		local card = ctx:find_instance(id)
		if type(card) == "table" and (card.zone.z == "field" or card.zone.z == "hand") and card.controller == you then
			return true
		end
	end
	return false
end

-- The opponent's announced Spell whose declared targets `reaches` one of yours, or nil.
local function refused(
	ctx: J.EffectContext,
	event: J.GameEvent,
	reaches: (J.EffectContext, J.PlayerId, { string }) -> boolean
): string?
	if event.type ~= "cardAnnounced" then
		return nil
	end
	local you = ctx:controller()
	if event.player == you or event.cardType ~= "Spell" then
		return nil
	end
	if reaches(ctx, you, event.targets) then
		return event.instanceId
	end
	return nil
end

-- The trap's trigger: its `when` is the condition (R99).
local function refusal(reaches: (J.EffectContext, J.PlayerId, { string }) -> boolean, drawn: number)
	return {
		id = "refusal",
		on = { "cardAnnounced" },
		when = function(ctx: J.EffectContext, event: J.GameEvent)
			return refused(ctx, event, reaches) ~= nil
		end,
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			local target: J.TargetSpec = { of = "instance", instanceId = refused(ctx, event, reaches) or "" }
			local effects = {
				J.counter_play({ target = target }),
			}
			if drawn > 0 then
				effects[#effects + 1] = J.draw({ count = drawn })
			end
			return effects
		end,
	}
end

return {
	base = {
		triggers = { refusal(your_unit, 0) },
	},
	radiant = {
		triggers = { refusal(you_or_yours, RADIANT_DRAW) },
	},
}
