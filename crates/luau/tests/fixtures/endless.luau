-- A hook whose loop never ends, which the interrupt stops at its cap (L5).
local J = require("@jackioh")

return {
	base = {
		cry = function(ctx)
			while true do
			end
		end,
	},
}
