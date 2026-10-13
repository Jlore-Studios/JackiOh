--!strict
local J = require("@jackioh")

-- No bonus: what the hook answers when the attack does not qualify.
local NONE = { attack = 0, poisonous = false }

-- Whether two printed tag lists share a tag (granted tags never count, M10).
local function share_a_tag(ours: { string }, theirs: { string }): boolean
	for _, tag in ipairs(ours) do
		for _, other in ipairs(theirs) do
			if tag == other then
				return true
			end
		end
	end
	return false
end

-- +`bonus` Attack while attacking a Unit that shares none of the attacker's printed tags, and
-- Poisonous with it when `poisonous`; `yours_only` covers its controller's attackers alone.
local function anxiety(args: any, yours_only: boolean, poisonous: boolean)
	local attacker = args:attacker()
	local defender = args:defender()
	if yours_only and attacker.controller ~= args:self_().controller then
		return NONE
	end
	-- MD-B1, R940: M #25 is immune to it, an untagged attacker included; the view's
	-- `conditionTargets` reads this same hook, so the yellow skips it too.
	if args:immune_to_tribal_hate(defender.id) then
		return NONE
	end
	if share_a_tag(args:def_of(attacker.defId).tags, args:def_of(defender.defId).tags) then
		return NONE
	end
	return { attack = args:param("bonus"), poisonous = poisonous }
end

local base = {
	attackMods = function(args)
		return anxiety(args, false, false)
	end,
}

-- "Your Units have +{bonus} Attack and Poisonous while doing so."
local radiant = {
	attackMods = function(args)
		return anxiety(args, true, true)
	end,
}

return {
	base = base,
	radiant = radiant,
}
