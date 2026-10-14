--!strict
local J = require("@jackioh")

local ID = "meditative-049-2"

-- Both faces share the one script: the declared `discards` and `damage` (1 and 2, Radiant 2 and 4)
-- are what differ. The amount is read as the hook resolves and carried into each hit, so a Nerf or
-- Buff on the numbers moves every hit of the turn (R386).
local base = {
	-- "Can't be in Defense Position": #65.1 Spikey Pillow's flag, so the token never has Defense's Taunt.
	staticFlags = { neverDefense = true },
	-- Activate (R384): destroys itself, once a turn in its controller's main phase.
	activations = {
		{
			id = "destroy",
			label = "Destroy this",
			uses = 1,
			run = function(ctx: J.EffectContext)
				return { J.destroy({ target = { of = "self" } }) }
			end,
		},
	},
	-- Discards at random from its controller's hand (R682), firing though the token vanishes (R11).
	death = function(ctx: J.EffectContext)
		return { J.discard_random({ count = ctx:param("discards") }) }
	end,
	-- One hit each on the Units beside it on its side and row (§3.1), those of Yile's Virus's
	-- definition skipped, then one on its controller's hero.
	startOfTurn = function(ctx: J.EffectContext)
		local amount = ctx:param("damage")
		local hero: J.TargetSpec = { of = "selfHero" }
		return {
			J.for_each_card({ cards = "each_cards", each = "each_card", data = { amount = amount } }),
			J.damage({ to = hero, amount = amount }),
		}
	end,
	each_cards = function(ctx: J.EffectContext): { string }
		local ids: { string } = {}
		for _, card in ipairs(ctx:adjacent_to({ of = "self" }, {})) do
			if card.defId ~= ID then
				ids[#ids + 1] = card.id
			end
		end
		return ids
	end,
	each_card = function(id: string, data: any)
		return { J.damage({ to = { of = "instance", instanceId = id }, amount = data.amount }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
