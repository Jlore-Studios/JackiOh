--!strict
local J = require("@jackioh")

-- Gap 8: a generic aura, declared as data. A stat `mod` with a data `applies`
-- filter (the first entry), and one with a named predicate over the
-- candidate unit (the second, `aura_filter`).
local base = {
	aura = function(_ctx: J.HookArgs)
		return {
			J.aura({
				mod = { attack = 2, maxHealth = 2 },
				applies = { side = "self" },
			}),
			J.aura({
				mod = { maxHealth = 1 },
				applies = "aura_filter",
			}),
		}
	end,
	aura_filter = function(unit: any)
		return unit.defId ~= "lx-fx-002"
	end,
}

return {
	base = base,
	radiant = "base",
}
