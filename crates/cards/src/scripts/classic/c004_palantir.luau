--!strict
local J = require("@jackioh")

-- Where the Radiant face keeps the cost it has stolen so far.
local STOLEN_COST = "stolenCost"

-- Whether `tags` holds the Book tag.
local function has_book(tags: { string }): boolean
	for _, tag in ipairs(tags) do
		if tag == "Book" then
			return true
		end
	end
	return false
end

-- The opponent's announced Spell (a Book only, on the base face), or nil: the announced card's
-- instance id, which is what this answers.
local function answered(ctx: J.EffectContext, event: J.GameEvent, books_only: boolean): string?
	if event.type ~= "cardAnnounced" then
		return nil
	end
	if event.player == ctx:controller() or event.cardType ~= "Spell" then
		return nil
	end
	-- MD-B15, R923: a granted Book tag counts — read the announced card itself when it still exists,
	-- else its definition's tags.
	if books_only then
		local card = ctx:find_instance(event.instanceId)
		local tags = if card ~= nil then ctx:tags_of(event.instanceId) else ctx:def_of(event.defId).tags
		if not has_book(tags) then
			return nil
		end
	end
	return event.instanceId
end

-- The opponent may draw `drawLimit` cards a turn.
local function draw_limit(ctx: J.HookArgs)
	local limit: J.DrawLimit = { player = "enemy", count = ctx:param("drawLimit") }
	return { limit }
end

-- Base: "Tribute this to steal it" — no question, the price is paid to steal.
local steal_book = {
	id = "palantir-book",
	on = { "cardAnnounced" },
	when = function(ctx: J.EffectContext, event: J.GameEvent)
		return answered(ctx, event, true) ~= nil
	end,
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		local book = answered(ctx, event, true)
		if book == nil then
			return {}
		end
		return {
			J.sacrifice({ target = { of = "self" } }),
			J.counter_play({ to = "thief", target = { of = "instance", instanceId = book } }),
		}
	end,
}

-- Radiant: steal every Spell, count what it cost, and Tribute this at the threshold.
local steal_spells = {
	id = "palantir-spell",
	on = { "cardAnnounced" },
	when = function(ctx: J.EffectContext, event: J.GameEvent)
		return answered(ctx, event, false) ~= nil
	end,
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		local spell = answered(ctx, event, false)
		if spell == nil then
			return {}
		end
		-- R65: its own cost as it stands out of play, an X Spell 0 (R396).
		local cost = ctx:own_cost(spell) or 0
		local before = ctx:recalled(STOLEN_COST)
		local total = (if type(before) == "number" then before else 0) + math.max(cost, 0)
		local steal = J.counter_play({ to = "thief", target = { of = "instance", instanceId = spell } })
		local kept = J.remember({ key = STOLEN_COST, value = total })
		if total >= ctx:param("threshold") then
			return { steal, kept, J.sacrifice({ target = { of = "self" } }) }
		end
		return { steal, kept }
	end,
}

return {
	base = {
		drawLimit = draw_limit,
		triggers = { steal_book },
	},
	radiant = {
		drawLimit = draw_limit,
		triggers = { steal_spells },
	},
}
