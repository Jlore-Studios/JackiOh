--!strict
local J = require("@jackioh")

-- `secrets::SECRET_KEY`: the key the Mind Games that made this card links its secret under (R862).
local SECRET_KEY = "secret"

-- R860: the three choices, in `SecretChoice::ALL`'s order.
local GREED = "greed"
local ATTACK = "attack"
local DEFEND = "defend"
local CHOICES = { GREED, ATTACK, DEFEND }

-- The declared guess as a choice, or nil when none was declared.
local function guess_of(mode: string?): J.SecretChoice?
	for _, choice in ipairs(CHOICES) do
		if mode == choice then
			return choice :: J.SecretChoice
		end
	end
	return nil
end

-- The opponent's active Units as the clause reaches them.
local function enemy_units(ctx: J.EffectContext): { string }
	local ids: { string } = {}
	for _, unit in ipairs(ctx:active_units_of(ctx:opponent_of(ctx:controller()))) do
		ids[#ids + 1] = unit.id
	end
	return ids
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

-- One choice's penalty, on the player who plays or discards this card: Greed taxes their own next
-- refresh and discards at random; Attack hits their own hero and each of their Units; Defend heals
-- and armors the opponent's side instead (R863).
local function penalty(ctx: J.EffectContext, guess: J.SecretChoice): { J.EffectCall }
	if guess == GREED then
		return {
			J.next_turn_mana({ amount = -ctx:param("mana") }),
			J.discard_random({ count = ctx:param("discards") }),
		}
	elseif guess == ATTACK then
		return {
			J.damage_all({
				amount = ctx:param("damage"),
				side = "self",
				heroes = true,
			}),
		}
	end
	local heal_n = ctx:param("heal")
	local armor_n = ctx:param("armor")
	local hero: J.TargetSpec = { of = "enemyHero" }
	return {
		J.heal({ target = hero, amount = heal_n }),
		J.for_each_card({ cards = "each_cards", each = "each_card", data = { amount = heal_n } }),
		J.gain_hero_armor({ amount = armor_n, player = "enemy" }),
		J.for_each_card({ cards = "each_cards_2", each = "each_card_2", data = { n = armor_n } }),
	}
end

-- Every penalty at once: the base face's discard clause (R862).
local function all_penalties(ctx: J.EffectContext): { J.EffectCall }
	local out: { J.EffectCall } = {}
	for _, guess in ipairs({ GREED, ATTACK, DEFEND } :: { J.SecretChoice }) do
		for _, effect in ipairs(penalty(ctx, guess)) do
			out[#out + 1] = effect
		end
	end
	return out
end

-- R862: the guess is judged against the secret this card's own Mind Games linked in its memory. No
-- guess, no link (a copy, or one another card made) or no live secret — a resolved one included —
-- and the card does nothing.
local function cry(ctx: J.EffectContext)
	local guess = guess_of(ctx:chosen_options()[1])
	local link = ctx:recalled(SECRET_KEY)
	if guess == nil or type(link) ~= "string" then
		return {}
	end
	local secret = ctx:secret_of(link)
	if secret == nil or secret.choice == nil then
		return {}
	end
	local out: { J.EffectCall } = {
		J.guess_secret({ secretId = link, guess = guess }),
	}
	if ctx:judge(guess, secret.choice) == "lost" then
		for _, effect in ipairs(penalty(ctx, guess)) do
			out[#out + 1] = effect
		end
	end
	return out
end

local MODES = {
	{ kind = "mode", options = CHOICES },
}

local base = {
	modes = MODES,
	cry = cry,
	graveyardTriggers = {
		{
			id = "discardPenalty",
			on = { "discarded" },
			run = function(ctx: J.EffectContext, event: J.GameEvent)
				-- R862: the clause answers its own discard — Temporary's included — whatever the
				-- secret's state, even after it resolved or when none was ever kept.
				if event.type ~= "discarded" then
					return {}
				end
				local me = ctx:self_()
				if me == nil or me.id ~= event.instanceId then
					return {}
				end
				return all_penalties(ctx)
			end,
		},
	},
	each_cards = enemy_units,
	each_card = heal_each,
	each_cards_2 = enemy_units,
	each_card_2 = armor_each,
}

-- The Radiant face: the same without the discard clause.
local radiant = {
	modes = MODES,
	cry = cry,
	each_cards = enemy_units,
	each_card = heal_each,
	each_cards_2 = enemy_units,
	each_card_2 = armor_each,
}

return {
	base = base,
	radiant = radiant,
}
