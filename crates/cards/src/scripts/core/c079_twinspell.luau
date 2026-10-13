--!strict
local J = require("@jackioh")

-- "Echo +1", radiant "Echo +2": how many extra resolutions the next spell gets.
local BASE_ECHO = 1
local RADIANT_ECHO = 2

-- The two faces differ only in how many extra resolutions the next spell gets. The engine reads the
-- amount off the face the card wears when a Spell takes it, so a Twinspell made Radiant on the field
-- (#49 radiant) grants "Echo +2" from then on (§5.2, R209). It is declared as `echoGain` (R386), read
-- through `params::declared_or`, so a Degrade or an Upgrade moves it.
local function twinspell(amount: number)
	return {
		staticFlags = { echoGrant = amount },
	}
end

return {
	base = twinspell(BASE_ECHO),
	radiant = twinspell(RADIANT_ECHO),
}
