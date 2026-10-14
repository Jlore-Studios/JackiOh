--!strict
local J = require("@jackioh")

-- The step the delayed steal re-enters at the start of this player's next turn (§10.6, R62).
local STEAL_STEP = "steal"
-- `prompts::RESUME_HOOK`: the delay re-enters this card's `resume` table.
local RESUME_HOOK = "resume"
-- `effects::delay::HAND_WATCH_KEY`: the cards the watch still holds, in the entry's data.
local HAND_WATCH_KEY = "handWatch"

-- R437, R1141: the pending steal's mark, purple as #50 K-Pop Fanatic's.
local function steal_mark(): J.CardMark
	return { mark = "steal", color = "purple" }
end

-- The cards the watch still holds as it comes due, in the order they were picked.
local function still_watched(ctx: J.EffectContext): { string }
	local out: { string } = {}
	local held = ctx:data()[HAND_WATCH_KEY]
	if type(held) ~= "table" then
		return out
	end
	for _, id in ipairs(held) do
		if type(id) == "string" then
			out[#out + 1] = id
		end
	end
	return out
end

-- R12, R1140: each watched card still in their hand becomes this player's, in hand order.
local function steal_step(ctx: J.EffectContext)
	local ids = still_watched(ctx)
	if #ids == 0 then
		return {}
	end
	return {
		J.give_from_hand({ from = "enemy", ids = ids }),
	}
end

-- `every` is the Radiant face: every card of their hand, not `marks` of them.
local function do_or_die(every: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local hand: { string } = {}
			for _, card in ipairs(ctx:zone_cards(ctx:opponent_of(ctx:controller()), "hand")) do
				hand[#hand + 1] = card.id
			end
			if #hand == 0 then
				return {}
			end
			local picked: { string } = hand
			if not every then
				local marks = math.max(ctx:param("marks"), 0)
				picked = {}
				for i, id in ipairs(ctx:rng_shuffle(hand)) do
					if i > marks then
						break
					end
					picked[#picked + 1] = id
				end
			end
			return {
				J.delay({
					at = { phase = "start", player = "self" },
					step = STEAL_STEP,
					hook = RESUME_HOOK,
					handWatch = picked,
					mark = steal_mark(),
				}),
			}
		end,
		resume = {
			[STEAL_STEP] = steal_step,
		},
	}
end

return {
	base = do_or_die(false),
	radiant = do_or_die(true),
}
