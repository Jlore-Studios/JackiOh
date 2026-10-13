--!strict
local J = require("@jackioh")

-- The radiant face differs in data only — 20/20, Charge for Rush — which layer 1 reads off
-- `def.radiant`; neither face has text a script would carry, so the radiant face is the base one.
return {
	base = {},
	radiant = "base",
}
