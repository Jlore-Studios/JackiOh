-- A module that asks `require` for something other than the host's module (L5).
local J = require("@jackioh")
local other = require("other")

return {
	base = {
		cry = function(ctx)
			return {}
		end,
	},
}
