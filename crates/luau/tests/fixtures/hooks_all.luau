--!strict
local J = require("@jackioh")

-- One face exercising every direct hook adapter: each hook answers a canned
-- value of its own type so tests can call the adapter and read it back.
local base = {
	cry = function(ctx)
		return {
			J.damage({ to = { of = "enemyHero" }, amount = 1 }),
		}
	end,
	death = function(ctx)
		return {
			J.gain_mana({ amount = 1 }),
		}
	end,
	delayed = function(ctx)
		return {
			J.draw({ count = 1 }),
		}
	end,
	startOfTurn = function(ctx)
		return {
			J.heal({ target = { of = "selfHero" }, amount = 1 }),
		}
	end,
	endOfTurn = function(ctx)
		return {
			J.damage({ to = { of = "enemyHero" }, amount = 1 }),
		}
	end,
	startOfOpponentTurn = function(ctx)
		return {
			J.draw({ count = 1 }),
		}
	end,
	afterAttack = function(ctx)
		return {
			J.damage({ to = { of = "enemyHero" }, amount = 1 }),
		}
	end,
	afterAttacked = function(ctx)
		return {
			J.heal({ target = { of = "selfHero" }, amount = 1 }),
		}
	end,
	entersHand = function(ctx)
		return {
			J.gain_mana({ amount = 1 }),
		}
	end,
	onPlayHook = function(ctx)
		return {
			J.draw({ count = 1 }),
		}
	end,
	cost = function(args)
		return 3
	end,
	conditionMet = function(ctx)
		return ctx:unspent_mana_of(ctx:controller()) >= 0
	end,
	preview = function(ctx)
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
