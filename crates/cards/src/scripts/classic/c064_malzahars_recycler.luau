--!strict
local J = require("@jackioh")

-- "End of turn: Discard 2 random cards." — random (R682).
local END_OF_TURN_DISCARDS = 2

local function end_of_turn(ctx: J.EffectContext)
	return {
		J.discard_random({ count = END_OF_TURN_DISCARDS }),
	}
end

-- Whether this event is a card its controller discarded.
local function your_discard(ctx: J.EffectContext, event: J.GameEvent): boolean
	return event.type == "discarded" and event.owner == ctx:controller()
end

-- "Draw that many": one draw for each card discarded.
local function draw_one(ctx: J.EffectContext)
	return {
		J.draw({ count = 1 }),
	}
end

-- "Draw your deck": the deck's size as the draw starts (R58).
local function draw_deck(ctx: J.EffectContext)
	return {
		J.draw({ count = ctx:zone_count(ctx:controller(), "library") }),
	}
end

local function answer(draws: (J.EffectContext) -> { J.EffectCall })
	return {
		id = "recycle",
		on = { "discarded" },
		when = your_discard,
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			if your_discard(ctx, event) then
				return draws(ctx)
			end
			return {}
		end,
	}
end

return {
	base = {
		endOfTurn = end_of_turn,
		triggers = { answer(draw_one) },
	},
	radiant = {
		endOfTurn = end_of_turn,
		triggers = { answer(draw_deck) },
	},
}
