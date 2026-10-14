--!strict
local J = require("@jackioh")

-- A spell's script is its `cry` hook (§10.9). §8: base draws 1, radiant draws 2 — the declared
-- number `draw` (R386), read off the face that is running.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.draw({ count = ctx:param("draw") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
