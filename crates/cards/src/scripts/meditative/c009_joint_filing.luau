--!strict
local J = require("@jackioh")

-- The engine's turn-hook multiplier: while this acts on its controller's field, each of their
-- start-of-turn and end-of-turn hooks is queued 1 + `extra` times (R821); several multipliers do not
-- add, the highest holds (R820). Both faces run one script; `param` reads the face's `extra`.
local base = {
	turnHookExtra = function(args: J.HookArgs): number
		return args:param("extra")
	end,
}

return {
	base = base,
	radiant = "base",
}
