--!strict
local J = require("@jackioh")

-- The printed "Draw 1-2": the fewest and most cards one Cry draws.
local MIN_DRAWS = 1
local MAX_DRAWS = 2

-- The printed "Add 1-4" (base) and "Add 1-10" (Radiant): the least and most one Cry adds.
local MIN_JADE = 1
local MAX_JADE = 4
local MAX_JADE_RADIANT = 10

-- R1438, R987: the Lucky on the card as it stands, and its controller's Luck.
local function lucky_of(ctx: J.EffectContext): number
	local card = ctx:live_self()
	local lucky = 0
	if card ~= nil then
		lucky = ctx:lucky_on(card.id)
	end
	return lucky + ctx:luck_of(ctx:controller())
end

-- One uniform roll over `low..=high`, made Lucky X: X more rolls, keeping the highest (the
-- engine's `Rng::lucky`).
local function lucky_roll(ctx: J.EffectContext, lucky: number, low: number, high: number): number
	local best = ctx:rng_next_int(low, high)
	for _ = 1, lucky do
		best = math.max(best, ctx:rng_next_int(low, high))
	end
	return best
end

local function catboy_maid(max_jade: number)
	return {
		cry = function(ctx: J.EffectContext)
			local lucky = lucky_of(ctx)
			local draws = lucky_roll(ctx, lucky, MIN_DRAWS, MAX_DRAWS)
			local jade = lucky_roll(ctx, lucky, MIN_JADE, max_jade)
			return {
				J.draw({ count = draws }),
				J.add_jade({ amount = jade }),
			}
		end,
	}
end

return {
	base = catboy_maid(MAX_JADE),
	radiant = catboy_maid(MAX_JADE_RADIANT),
}
