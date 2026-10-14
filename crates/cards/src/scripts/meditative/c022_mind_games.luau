--!strict
local J = require("@jackioh")

-- The token this hands the opponent (M #22.1).
local FORTIFY_MIND = "meditative-022-1"

-- The `resume` step the reward re-enters.
local REWARD = "reward"

-- `secrets::SECRET_KEY`: the secret's id in the reward entry's data.
local SECRET_KEY = "secret"

-- R860: the three choices, in `SecretChoice::ALL`'s order.
local GREED = "greed"
local ATTACK = "attack"
local DEFEND = "defend"
local CHOICES = { GREED, ATTACK, DEFEND }

-- The declared mode as a choice, or nil when none was declared (a redacted copy, R865).
local function choice_of(mode: string?): J.SecretChoice?
	for _, choice in ipairs(CHOICES) do
		if mode == choice then
			return choice :: J.SecretChoice
		end
	end
	return nil
end

-- The caster's active Units as the clause reaches them.
local function own_units(ctx: J.EffectContext): { string }
	local ids: { string } = {}
	for _, unit in ipairs(ctx:active_units_of(ctx:controller())) do
		ids[#ids + 1] = unit.id
	end
	return ids
end

-- The reward's Defend: heal the caster's hero and each of their Units, then grant the hero Armor
-- and each Unit Armor `n` (R59: one hit each, closed by one state check).
local function defend_side(heal_n: number, armor_n: number): { J.EffectCall }
	local hero: J.TargetSpec = { of = "selfHero" }
	return {
		J.heal({ target = hero, amount = heal_n }),
		J.for_each_card({ cards = "each_cards", each = "each_card", data = { amount = heal_n } }),
		J.gain_hero_armor({ amount = armor_n }),
		J.for_each_card({ cards = "each_cards_2", each = "each_card_2", data = { n = armor_n } }),
	}
end

local function heal_each(id: string, data: any)
	local target: J.TargetSpec = { of = "instance", instanceId = id }
	return {
		J.heal({ target = target, amount = data.amount }),
	}
end

local function armor_each(id: string, data: any)
	return {
		J.grant_keyword({
			target = { of = "instance", instanceId = id },
			keyword = { kind = "Armor", n = data.n },
		}),
	}
end

-- The reward at the start of the caster's next turn (R861): the secret resolves, then its reward.
local function reward(ctx: J.EffectContext)
	local secret_id = ctx:data()[SECRET_KEY]
	if type(secret_id) ~= "string" then
		return {}
	end
	local secret = ctx:secret_of(secret_id)
	if secret == nil or secret.choice == nil then
		return {}
	end
	local choice = secret.choice
	local out: { J.EffectCall } = {
		J.resolve_secret({ secretId = secret_id }),
	}
	if choice == GREED then
		out[#out + 1] = J.gain_mana({ amount = ctx:param("mana") })
		out[#out + 1] = J.draw({ count = ctx:param("draw") })
	elseif choice == ATTACK then
		out[#out + 1] = J.damage_all({
			amount = ctx:param("damage"),
			side = "enemy",
			heroes = true,
		})
	elseif choice == DEFEND then
		for _, effect in ipairs(defend_side(ctx:param("heal"), ctx:param("armor"))) do
			out[#out + 1] = effect
		end
	end
	return out
end

-- The mode is declared with the play (R81) and kept secret (R860, R865). The Radiant face prices
-- the Fortify Mind it hands over; the reward matches the base face.
local function mind_games(radiant: boolean)
	return {
		modes = {
			{ kind = "mode", options = CHOICES },
		},
		secretModes = true,
		cry = function(ctx: J.EffectContext)
			local choice = choice_of(ctx:chosen_options()[1])
			if choice == nil then
				-- R865: a redacted copy has no mode to file, so no secret is kept — but the
				-- Fortify Mind it would have linked still reaches the enemy hand.
				return {
					J.add_to_hand({ defId = FORTIFY_MIND, player = "enemy" }),
				}
			end
			local link: J.SecretLink = { defId = FORTIFY_MIND, player = "enemy" }
			if radiant then
				link.costOverride = ctx:param("fortifyCost")
			end
			return {
				J.keep_secret({
					choice = choice,
					step = REWARD,
					link = link,
				}),
			}
		end,
		resume = {
			[REWARD] = reward,
		},
		each_cards = own_units,
		each_card = heal_each,
		each_cards_2 = own_units,
		each_card_2 = armor_each,
	}
end

return {
	base = mind_games(false),
	radiant = mind_games(true),
}
