--!strict
local J = require("@jackioh")

-- "Draw {cards} from the bottom of your opponent's deck": that many separate draws (§2.4).
local function draw_bottoms(ctx: J.EffectContext)
	local effects = {}
	for _ = 1, ctx:param("cards") do
		effects[#effects + 1] = J.draw_from_opponent({ ["end"] = "bottom" })
	end
	return effects
end

return {
	base = {
		startOfTurn = draw_bottoms,
	},
	radiant = {
		startOfTurn = draw_bottoms,
		endOfTurn = draw_bottoms,
	},
}
