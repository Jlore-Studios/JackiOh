--!strict
local J = require("@jackioh")

-- "When this enters your hand: Replace it with a random Radiant card." The pool is every open set's
-- non-token cards but this one (R380, R387); the new card is Chinese (ME-CN) and runs its own
-- arrival hooks (R151). No cry on either face: a cast resolves with no effect (MD-B19, R926).
local function bottle(radiant: boolean)
	return {
		entersHand = function(ctx: J.EffectContext)
			local args: J.TransformRandomArgs = {
				target = { of = "self" },
				radiant = true,
				chinese = true,
				arrives = true,
			}
			-- The Radiant face's new card costs (0).
			if radiant then
				args.costOverride = 0
			end
			return {
				J.transform_random(args),
			}
		end,
	}
end

return {
	base = bottle(false),
	radiant = bottle(true),
}
