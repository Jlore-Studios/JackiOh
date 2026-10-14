--!strict
local J = require("@jackioh")

-- "Tribute 1" (§6.3): one of your units, or one Sheep Token.
local TRIBUTE_COST = 1

-- "Draw {draw} and heal your hero {heal}", the text the Cry and the attack trigger share.
local function titan(ctx: J.EffectContext)
	local hero: J.TargetSpec = { of = "selfHero" }
	return {
		J.draw({ count = ctx:param("draw") }),
		J.heal({ target = hero, amount = ctx:param("heal") }),
	}
end

-- The same script on both faces: the Radiant face's draw 2 and heal 6 are its declared numbers,
-- which `param` reads off the running face.
local base = {
	staticFlags = { tribute = TRIBUTE_COST },
	cry = titan,
	triggers = {
		-- "Whenever this attacks": every `attackDeclared` whose attacker is this card, forced ones
		-- included. The test is in `run`: a `when` is read for traps only (R99).
		{
			id = "45-whenever-this-attacks",
			on = { "attackDeclared" },
			run = function(ctx: J.EffectContext, event: J.GameEvent)
				if event.type ~= "attackDeclared" then
					return {}
				end
				local me = ctx:self_()
				if type(me) ~= "table" or me.id ~= event.attackerId then
					return {}
				end
				return titan(ctx)
			end,
		},
	},
}

return {
	base = base,
	radiant = "base",
}
