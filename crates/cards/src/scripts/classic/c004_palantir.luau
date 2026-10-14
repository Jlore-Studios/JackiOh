--!strict
local J = require("@jackioh")

-- Where the Radiant face counts the Spells it has stolen so far.
local STOLEN = "stolen"

-- Patch v0.3.9 (#831): the Spells it answers are the (1) Cost ones (R65: a card's own cost as it
-- stands out of play, an X Spell 0), Books or not.
local STOLEN_COST = 1

-- The opponent's announced (1) Cost Spell, or nil: the announced card's instance id, which is what
-- this answers.
local function answered(ctx: J.EffectContext, event: J.GameEvent): string?
	if event.type ~= "cardAnnounced" then
		return nil
	end
	if event.player == ctx:controller() or event.cardType ~= "Spell" then
		return nil
	end
	if ctx:own_cost(event.instanceId) ~= STOLEN_COST then
		return nil
	end
	return event.instanceId
end

-- The opponent may draw `drawLimit` cards a turn.
local function draw_limit(ctx: J.HookArgs)
	local limit: J.DrawLimit = { player = "enemy", count = ctx:param("drawLimit") }
	return { limit }
end

-- "Steal it and cast it yourself": countered into your hand, then cast by you, every choice at
-- random but aimed in your favour (R831001, R452, R656).
local function steal_and_cast(spell: string): { J.EffectCall }
	local card: J.TargetSpec = { of = "instance", instanceId = spell }
	return {
		J.counter_play({ to = "thief", target = card }),
		J.cast({ target = card, random = true, targetEnemies = true }),
	}
end

-- Base: "Tribute this to steal it" — no question, the price is paid to steal.
local steal_one = {
	id = "palantir-steal",
	on = { "cardAnnounced" },
	when = function(ctx: J.EffectContext, event: J.GameEvent)
		return answered(ctx, event) ~= nil
	end,
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		local spell = answered(ctx, event)
		if spell == nil then
			return {}
		end
		local effects = { J.sacrifice({ target = { of = "self" } }) }
		for _, effect in ipairs(steal_and_cast(spell)) do
			effects[#effects + 1] = effect
		end
		return effects
	end,
}

-- Radiant: steal each one, count them, and Tribute this once it has stolen {threshold}.
local steal_some = {
	id = "palantir-spells",
	on = { "cardAnnounced" },
	when = function(ctx: J.EffectContext, event: J.GameEvent)
		return answered(ctx, event) ~= nil
	end,
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		local spell = answered(ctx, event)
		if spell == nil then
			return {}
		end
		local before = ctx:recalled(STOLEN)
		local total = (if type(before) == "number" then before else 0) + 1
		local effects = steal_and_cast(spell)
		effects[#effects + 1] = J.remember({ key = STOLEN, value = total })
		if total >= ctx:param("threshold") then
			effects[#effects + 1] = J.sacrifice({ target = { of = "self" } })
		end
		return effects
	end,
}

return {
	base = {
		drawLimit = draw_limit,
		triggers = { steal_one },
	},
	radiant = {
		drawLimit = draw_limit,
		triggers = { steal_some },
	},
}
