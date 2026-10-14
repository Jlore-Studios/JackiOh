--!strict
local J = require("@jackioh")

-- A unit that cries itself into another fixture unit.
local base = {
	cry = function(ctx)
		return {
			J.transform({ target = { of = "self" }, defId = "lx-fx-002" }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
