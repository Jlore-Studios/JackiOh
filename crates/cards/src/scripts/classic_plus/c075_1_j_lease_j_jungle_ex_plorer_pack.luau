--!strict
local J = require("@jackioh")

-- One face: whether the cards it adds cost (0), the Radiant face's declared number `setCost` (R386).
-- The pool names its two sets, so Core never comes up (R380), and each card is made Radiant before it
-- reaches the hand (R74).
local function pack(free: boolean)
	return {
		staticFlags = { castOnDraw = true },
		cry = function(ctx: J.EffectContext)
			local args: J.AddRandomFromCatalogArgs = {
				query = { set = { "Classic", "Classic+" } },
				count = ctx:param("cards"),
				radiant = true,
			}
			if free then
				args.costOverride = ctx:param("setCost")
			end
			return {
				J.add_random_from_catalog(args),
			}
		end,
	}
end

return {
	base = pack(false),
	radiant = pack(true),
}
