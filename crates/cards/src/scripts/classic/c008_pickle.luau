--!strict
local J = require("@jackioh")

local DISCARD = "discard"
local EXILE = "exile"
local DRAW = "draw"

-- The chain's data keys: the numbers, read once as the Spell resolves and carried through every
-- question (§10.6), and which question is being asked (1-based).
type Chain = { choices: number, discard: number, exile: number, draw: number, n: number }

-- TS `numberIn(data, key)`: a finite integer number, else nil. §10.6's data is JSON, so what
-- comes back is narrowed, never cast.
local function number_in(data: any, key: string): number?
	if type(data) ~= "table" then
		return nil
	end
	local value = data[key]
	if type(value) ~= "number" then
		return nil
	end
	if value ~= math.floor(value) then
		return nil
	end
	return value
end

local function chain_of(ctx: J.EffectContext): Chain?
	local data = ctx:data()
	local n = number_in(data, "n")
	local choices = number_in(data, "choices")
	local discard = number_in(data, "discard")
	local exile = number_in(data, "exile")
	local draw = number_in(data, "draw")
	if n == nil or choices == nil or discard == nil or exile == nil or draw == nil then
		return nil
	end
	return { choices = choices, discard = discard, exile = exile, draw = draw, n = n }
end

local function cards(count: number): string
	if count == 1 then
		return "1 card"
	end
	return tostring(count) .. " cards"
end

-- Question `chain.n`, offering only what would do something; nothing once every question is asked.
local function ask(ctx: J.EffectContext, chain: Chain, hand_loss: number, deck_loss: number): { J.EffectCall }
	if chain.n > chain.choices then
		return {}
	end
	local enemy = ctx:opponent_of(ctx:controller())
	local options: { string } = {}
	if ctx:zone_count(enemy, "hand") - hand_loss > 0 and not ctx:discard_guarded(enemy) then
		options[#options + 1] = DISCARD
	end
	if ctx:zone_count(enemy, "library") - deck_loss > 0 then
		options[#options + 1] = EXILE
	end
	options[#options + 1] = DRAW
	local data: { [string]: J.JsonValue } = {
		choices = chain.choices,
		discard = chain.discard,
		exile = chain.exile,
		draw = chain.draw,
		n = chain.n,
	}
	return {
		J.choose_mode({
			by = "enemy",
			options = options,
			labels = {
				[DISCARD] = "Discard " .. cards(chain.discard),
				[EXILE] = "Exile the bottom " .. cards(chain.exile) .. " of your deck",
				[DRAW] = "Your opponent draws " .. cards(chain.draw),
			},
			step = "chosen",
			prompt = "Pickle: your choice (" .. tostring(chain.n) .. " of " .. tostring(chain.choices) .. ")",
			data = data,
		}),
	}
end

-- The `chosen` step: the opponent's answer to question `n`, then question `n + 1`.
local function chosen(ctx: J.EffectContext): { J.EffectCall }
	local chain = chain_of(ctx)
	if chain == nil then
		return {}
	end
	local follow: Chain = {
		choices = chain.choices,
		discard = chain.discard,
		exile = chain.exile,
		draw = chain.draw,
		n = chain.n + 1,
	}
	local enemy = ctx:opponent_of(ctx:controller())
	local answer = ctx:chosen_options()[1]
	if answer == DISCARD then
		-- A hand emptied since the question was asked has nothing to discard: on to the next one.
		-- The random discard lands first in the list, so the next question subtracts it.
		local hand_loss = math.min(chain.discard, ctx:zone_count(enemy, "hand"))
		if hand_loss == 0 then
			return ask(ctx, follow, 0, 0)
		end
		local effects = {
			J.discard_random({ count = chain.discard, player = "enemy" }),
		}
		for _, effect in ipairs(ask(ctx, follow, hand_loss, 0)) do
			effects[#effects + 1] = effect
		end
		return effects
	end
	if answer == EXILE then
		local deck_loss = math.min(chain.exile, ctx:zone_count(enemy, "library"))
		local effects = {
			J.exile_bottom_of_library({ player = "enemy", count = chain.exile }),
		}
		for _, effect in ipairs(ask(ctx, follow, 0, deck_loss)) do
			effects[#effects + 1] = effect
		end
		return effects
	end
	if answer == DRAW then
		local effects = {
			J.draw({ count = chain.draw }),
		}
		for _, effect in ipairs(ask(ctx, follow, 0, 0)) do
			effects[#effects + 1] = effect
		end
		return effects
	end
	return {}
end

local pickle = {
	cry = function(ctx: J.EffectContext)
		local chain: Chain = {
			choices = ctx:param("choices"),
			discard = ctx:param("discard"),
			exile = ctx:param("exile"),
			draw = ctx:param("draw"),
			n = 1,
		}
		return ask(ctx, chain, 0, 0)
	end,
	resume = {
		chosen = chosen,
	},
}

-- The same script: the Radiant face differs only in its declared numbers, which `param` reads
-- off the running face.
return {
	base = pickle,
	radiant = "base",
}
