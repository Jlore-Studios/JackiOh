--!strict
local J = require("@jackioh")

-- One face exercising every direct hook adapter: each hook answers a canned
-- value of its own type so tests can call the adapter and read it back.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.damage({ to = { of = "enemyHero" }, amount = 1 }),
		}
	end,
	death = function(ctx: J.EffectContext)
		return {
			J.gain_mana({ amount = 1 }),
		}
	end,
	delayed = function(ctx: J.EffectContext)
		return {
			J.draw({ count = 1 }),
		}
	end,
	startOfTurn = function(ctx: J.EffectContext)
		local target: J.TargetSpec = { of = "selfHero" }
		return {
			J.heal({ target = target, amount = 1 }),
		}
	end,
	endOfTurn = function(ctx: J.EffectContext)
		return {
			J.damage({ to = { of = "enemyHero" }, amount = 1 }),
		}
	end,
	startOfOpponentTurn = function(ctx: J.EffectContext)
		return {
			J.draw({ count = 1 }),
		}
	end,
	afterAttack = function(ctx: J.EffectContext)
		return {
			J.damage({ to = { of = "enemyHero" }, amount = 1 }),
		}
	end,
	afterAttacked = function(ctx: J.EffectContext)
		local target: J.TargetSpec = { of = "selfHero" }
		return {
			J.heal({ target = target, amount = 1 }),
		}
	end,
	entersHand = function(ctx: J.EffectContext)
		return {
			J.gain_mana({ amount = 1 }),
		}
	end,
	onPlayHook = function(ctx: J.EffectContext)
		return {
			J.draw({ count = 1 }),
		}
	end,
	cost = function(args)
		return 3
	end,
	conditionMet = function(ctx: J.ConditionContext)
		return ctx:unspent_mana_of(ctx:controller()) >= 0
	end,
	preview = function(ctx: J.ConditionContext)
		return {}
	end,
	setStat = function(args)
		return { attack = 3 }
	end,
	aura = function(args)
		return {
			J.defense_armor_aura({ armor = 1 }),
		}
	end,
	costAura = function(args)
		return {}
	end,
	graveyardPlay = function(args)
		return {}
	end,
	faceDownPlay = function(args)
		return {}
	end,
	targetingDiscards = function(args)
		return 0
	end,
	drawLimit = function(args)
		return {}
	end,
	discardGuard = function(args)
		return {}
	end,
	heroGuard = function(args)
		return {}
	end,
	attackMods = function(args)
		return { attack = 1, poisonous = false }
	end,
	conditionalKeywords = function(args)
		return {}
	end,
	plagueMultiplier = function(args)
		return 1
	end,
	echoX = function(args)
		return 0
	end,
	turnHookExtra = function(args)
		return 1
	end,
	cryDeathExtra = function(args)
		return 1
	end,
	tributeWhen = function(args)
		return false
	end,
	wouldCounter = function(args)
		return false
	end,
}

return {
	base = base,
	radiant = "base",
}
