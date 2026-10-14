--!strict
local J = require("@jackioh")

-- "When your opponent plays a card that costs ({threshold}) or less": the cost paid (R56, R70).
local function cheap_play(ctx: J.EffectContext, event: J.GameEvent): boolean
	if event.type ~= "cardAnnounced" then
		return false
	end
	if event.player == ctx:controller() then
		return false
	end
	return event.costPaid <= ctx:param("threshold")
end

local function exile_trap(radiant_face: boolean): J.TriggerDef
	return {
		id = "exile",
		on = { "cardAnnounced" },
		when = cheap_play,
		-- The condition again: `when` is a trap's alone (R99), and a non-trap that holds this text
		-- (a fusion kept as another card) runs its trigger on every announce.
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			if event.type ~= "cardAnnounced" or not cheap_play(ctx, event) then
				return {}
			end
			local countered = J.counter_play({
				to = "exile",
				target = { of = "instance", instanceId = event.instanceId },
			})
			if not radiant_face then
				return { countered }
			end
			local budget = ctx:param("threshold") - event.costPaid
			return {
				countered,
				J.for_each_card({ cards = "each_cards", each = "each_card", data = { budget = budget } }),
			}
		end,
	}
end

-- The Radiant face's picks live on the face, where `for_each_card` resolves them: the trigger
-- names them, the face holds them.
local function exile_face(radiant_face: boolean)
	return {
		triggers = { exile_trap(radiant_face) },
		each_cards = function(ctx: J.EffectContext, data: any)
			if type(data) ~= "table" or type(data.budget) ~= "number" then
				return {}
			end
			local picked: { string } = {}
			local taken: { [string]: boolean } = {}
			local left = data.budget
			while left > 0 do
				local fits: { any } = {}
				for _, card in
					ipairs(ctx:cards_in_scope({ side = "enemy", rows = { "units", "backrow" } }))
				do
					if not taken[card.id] and ctx:cost_now(card.id) <= left then
						fits[#fits + 1] = card
					end
				end
				if #fits == 0 then
					break
				end
				local card = ctx:rng_pick(fits)
				if type(card) ~= "table" then
					break
				end
				left -= ctx:cost_now(card.id)
				taken[card.id] = true
				picked[#picked + 1] = card.id
			end
			return picked
		end,
		each_card = function(id: string)
			return { J.exile({ target = { of = "instance", instanceId = id } }) }
		end,
	}
end

return {
	base = exile_face(false),
	radiant = exile_face(true),
}
