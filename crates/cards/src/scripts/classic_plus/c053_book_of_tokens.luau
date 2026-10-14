--!strict
local J = require("@jackioh")

-- TS `cardDef("core-t-rush").id`.
local RUSH_TOKEN = "core-t-rush"

-- The printed "1-2": the fewest and most tokens one cast summons.
local MIN_TOKENS = 1
local MAX_TOKENS = 2

-- §6.1: the Lucky X the running card has now, printed plus given (R1438; its Radiant face prints
-- Lucky 1).
local function lucky_of(ctx: J.EffectContext): number
	local card = ctx:live_self()
	-- No running card: the reader's `None` crosses as a null, never a table.
	if type(card) ~= "table" then
		return 0
	end
	return ctx:lucky_on(card.id)
end

local function book_of_tokens(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local function roll(): number
				return ctx:rng_next_int(MIN_TOKENS, MAX_TOKENS)
			end
			-- R987: the controller's Luck rolls extra times beside the card's own Lucky. Lucky X rolls
			-- X more times and keeps the most (the engine's `Rng::lucky`).
			local lucky = lucky_of(ctx) + ctx:luck_of(ctx:controller())
			local count = roll()
			for _ = 1, lucky do
				count = math.max(count, roll())
			end
			local effects = {}
			for _ = 1, count do
				effects[#effects + 1] = J.summon({ defId = RUSH_TOKEN, radiant = radiant })
			end
			return effects
		end,
	}
end

return {
	base = book_of_tokens(false),
	radiant = book_of_tokens(true),
}
