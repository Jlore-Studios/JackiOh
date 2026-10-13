--!strict
local J = require("@jackioh")

-- The printed thresholds: "1 or less", and "2 or less" on the radiant face. The card declares them as
-- `giftLimit` (R386), and the engine reads the flag through `params::declared_or`, so a Degrade or an
-- Upgrade moves the threshold step 3 uses.
local BASE_THRESHOLD = 1
local RADIANT_THRESHOLD = 2

local function gifted(threshold: number)
	return {
		staticFlags = { giftedProgram = threshold },
	}
end

return {
	base = gifted(BASE_THRESHOLD),
	radiant = gifted(RADIANT_THRESHOLD),
}
