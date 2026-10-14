--!strict
local J = require("@jackioh")

-- §7, R353: the token is the catalog's Ghoul Token, named by id.
local GHOUL_TOKEN = "core-t-ghoul"

-- The four multipliers are the whole of the radiant text: the declared numbers `damage`, `heal`,
-- `draw` and `ghoul` (R386), 2, 3, 2 and 3 on the Radiant face; the base face prints a bare X.
local base = {
	-- R81: "a target" travels in the play action; a hero is always on the board (R90).
	targets = {
		{ kind = "target", min = 1, max = 1, filter = { side = "any", of = { "unit", "hero" } } },
	},
	cry = function(ctx: J.EffectContext)
		-- The declared X off the context: the play validator refuses X < 0, and the clamp is belt
		-- and braces so no clause can go backwards.
		local x = math.max(ctx:x(), 0)
		-- R348 keeps a play from choosing X = 0; a run with none makes nothing, and no 0/0 token.
		if x == 0 then
			return {}
		end
		local stats = ctx:param("ghoul") * x
		local amount = ctx:param("damage") * x
		local healed = ctx:param("heal") * x
		local drawn = ctx:param("draw") * x
		local hero: J.TargetSpec = { of = "selfHero" }
		return {
			J.damage({ to = { of = "chosen" }, amount = amount }),
			J.heal({ target = hero, amount = healed }),
			J.draw({ count = drawn }),
			J.summon({ defId = GHOUL_TOKEN, statsOverride = { attack = stats, health = stats } }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
