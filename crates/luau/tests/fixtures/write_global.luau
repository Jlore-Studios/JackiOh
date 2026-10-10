-- A hook that writes a global, which the sandbox's read-only environment refuses (L5).
local J = require("@jackioh")

return {
	base = {
		cry = function(ctx)
			leaked = 1
			return {}
		end,
	},
}
