--!strict
local J = require("@jackioh")

-- §8: "+1 Plague Counter" — one per damage instance.
local TOKENS_PER_DAMAGE = 1

-- §8: base "+1 mana per Plague Counter"; radiant "+2 mana per token" — the declared number `mana`
-- (R386), read off the face that is up.

-- "Whenever THIS takes damage": the event carries the target, so a hit this card DEALT — its own
-- strike-back, its Trample overflow — is not a hit it took.
local function is_hit_on_self(ctx: J.EffectContext, event: J.GameEvent): boolean
	local self_ = ctx:self_()
	if self_ == nil then
		return false
	end
	if event.type == "damage" then
		return event.targetId == self_.id
	end
	return false
end

-- R280: the formula as the face prints it, with its rate as it stands, which the preview labels its
-- number with.
local function formula(per_token: number): string
	return "+" .. tostring(per_token) .. " mana per Plague Counter"
end

-- The mana the start-of-turn hook gains now: its own Plague Counters times the face's rate.
local function mana_now(self_: any, per_token: number): number
	if self_ == nil then
		return 0
	end
	return math.max(self_.counters.plague or 0, 0) * per_token
end

local base = {
	triggers = {
		{
			id = "fed-fauci-plague",
			on = { "damage" },
			when = is_hit_on_self,
			-- "+1 Plague Counter". `plague` defaults its target to `{ of: "self" }`, which is this card.
			run = function(ctx: J.EffectContext, event: J.GameEvent)
				if is_hit_on_self(ctx, event) then
					return {
						J.plague({ amount = TOKENS_PER_DAMAGE }),
					}
				end
				return {}
			end,
		},
	},
	-- "Start of turn: +N mana per Plague Counter" (§2.2, R62: after the refresh, before the draw). With
	-- no tokens the card gains nothing and returns no effect at all, so it emits no `manaChanged` for a
	-- change of zero.
	startOfTurn = function(ctx: J.EffectContext)
		local amount = mana_now(ctx:self_(), ctx:param("mana"))
		if amount <= 0 then
			return {}
		end
		return {
			J.gain_mana({ amount = amount }),
		}
	end,
	preview = function(ctx: J.ConditionContext)
		local per_token = ctx:param("mana")
		return {
			{
				label = formula(per_token),
				value = mana_now(ctx:self_(), per_token),
			},
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
