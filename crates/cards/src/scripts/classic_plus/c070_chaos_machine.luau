--!strict
local J = require("@jackioh")

-- Buff {cards} random other cards in your hand or on your side of the field, then Nerf {cards}
-- random cards in your opponent's (R386: one draw each, different cards, R60; R585: never Chaos
-- Machine itself). The Radiant face's two is its declared `cards`.
local function tick(ctx: J.EffectContext)
	return {
		J.upgrade({
			scope = { side = "self", zones = { "hand", "field" }, excludeSelf = true },
			random = ctx:param("cards"),
		}),
		J.degrade({ scope = { side = "enemy", zones = { "hand", "field" } }, random = ctx:param("cards") }),
	}
end

-- One function under both hooks, so one adapter (#464 gap 10), as the Rust clones one hook; both
-- faces run this one script.
local base = {
	startOfTurn = tick,
	endOfTurn = tick,
}

return {
	base = base,
	radiant = "base",
}
