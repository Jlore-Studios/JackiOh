--!strict
local J = require("@jackioh")

-- "It costs (0)": the Radiant face's price.
local FREE = 0

-- Start of turn: a random Radiant card with the KY tag, the CN tag or both (`anyTags`, R1422), never
-- the Printer itself (R387), not Chinese (⚠ designer); the Radiant face's costs (0).
local function printer(radiant: boolean)
	return {
		startOfTurn = function(_ctx: J.EffectContext)
			local args: J.AddRandomFromCatalogArgs = { query = { anyTags = { "KY", "CN" } }, radiant = true }
			if radiant then
				args.costOverride = FREE
			end
			return { J.add_random_from_catalog(args) }
		end,
	}
end

return {
	base = printer(false),
	radiant = printer(true),
}
