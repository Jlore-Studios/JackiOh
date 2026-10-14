--!strict
local J = require("@jackioh")

-- Where the instance keeps the Spell it exiled.
local SCEPTER = "scepter"
-- The name of the Cry's hand-pick rule in `targetChecks`.
local WITHIN_LIMIT = "spellWithinLimit"

-- What the instance remembers under `scepter`: an object naming a definition, with its Radiant
-- flag. The memory is JSON, so it is narrowed, never cast.
local function held_of(value: any): { defId: string, radiant: boolean }?
	if type(value) ~= "table" or type(value.defId) ~= "string" then
		return nil
	end
	return { defId = value.defId, radiant = value.radiant == true }
end

local base = {
	targets = {
		{
			kind = "hand",
			min = 1,
			max = 1,
			filter = { of = { "hand" }, type = "Spell", check = WITHIN_LIMIT },
		},
	},
	targetChecks = {
		-- "A ({costLimit}) Cost or less Spell from your hand", at the cost it would be played for
		-- now (R65).
		[WITHIN_LIMIT] = function(args: J.TargetCheckArgs): boolean
			local candidate = args:candidate()
			if candidate == nil then
				return false
			end
			return candidate.zone.z == "hand"
				and args:def_of(candidate.defId).type == "Spell"
				and args:effective_cost(candidate.id) <= args:param("costLimit")
		end,
	},
	-- Exile the picked Spell and remember its definition and Radiant flag.
	cry = function(ctx: J.EffectContext)
		local picked = ctx:instance_of({ of = "chosen" })
		if picked == nil then
			return {}
		end
		return {
			J.remember({ key = SCEPTER, value = { defId = picked.defId, radiant = picked.radiant } }),
			J.exile({ target = { of = "chosen" } }),
		}
	end,
	activations = {
		{
			id = "cast-copy",
			label = "Cast a copy of that Spell",
			uses = 1,
			canActivate = function(ctx: J.ConditionContext)
				return held_of(ctx:self_().memory[SCEPTER]) ~= nil
			end,
			-- A fresh copy, free and counted as a play (R70), its Radiant flag kept. The memory is
			-- read as `recalled` reads it: the running ingredient's own key on a fused card (R102).
			run = function(ctx: J.EffectContext)
				local held = held_of(ctx:recalled(SCEPTER))
				if held == nil then
					return {}
				end
				return {
					J.cast_new({ def = { defId = held.defId, radiant = held.radiant } }),
				}
			end,
		},
	},
}

-- The same script: the Radiant face's (2) is its declared costLimit, which `param` reads off the face.
return {
	base = base,
	radiant = "base",
}
