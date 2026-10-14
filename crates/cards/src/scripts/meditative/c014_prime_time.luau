--!strict
local J = require("@jackioh")

-- The smallest prime: 0 and 1 are not prime.
local SMALLEST_PRIME = 2
-- The largest index Rust's `parse::<u32>()` accepts.
local U32_MAX = 4294967295

-- Trial division: the indices that can occur stay under a few thousand. Both operands of `%` are
-- positive, where Luau's floored remainder is Rust's.
local function is_prime(n: number): boolean
	if n < SMALLEST_PRIME then
		return false
	end
	local divisor = SMALLEST_PRIME
	while divisor * divisor <= n do
		if n % divisor == 0 then
			return false
		end
		divisor += 1
	end
	return true
end

-- R804: the card's catalog index is a prime number. The index parses as Rust's `parse::<u32>()`
-- does (an optional `+`, then decimal digits only, at most `u32::MAX`), so a token's "N.k" or
-- T-name, and a transient card's id, fail the parse and never match.
local function prime_indexed(ctx: J.EffectContext, defId: string): boolean
	local digits = string.match(ctx:def_of(defId).index, "^%+?(%d+)$")
	if digits == nil then
		return false
	end
	local n = tonumber(digits)
	return n ~= nil and n <= U32_MAX and is_prime(n)
end

-- The library's prime-indexed ids, top-down, at most `take` of them (`nil` takes every match).
local function prime_ids(ctx: J.EffectContext, take: number?): { string }
	local ids: { string } = {}
	for _, card in ipairs(ctx:zone_cards(ctx:controller(), "library")) do
		if take ~= nil and #ids >= math.max(take, 0) then
			break
		end
		if prime_indexed(ctx, card.defId) then
			ids[#ids + 1] = card.id
		end
	end
	return ids
end

local function draw_each(id: string)
	return { J.draw_from_library({ instanceId = id }) }
end

-- One memoized `for_each_card` over the prime-indexed ids: read once as the list reaches it, so a
-- cast-on-draw pause resumes over the same set (R113).
local base = {
	-- "Draw 3 …": the declared number `draws` (R386, R1431).
	cry = function(ctx: J.EffectContext)
		return {
			J.for_each_card({ cards = "each_cards", each = "each_card", data = { take = ctx:param("draws") } }),
		}
	end,
	each_cards = function(ctx: J.EffectContext, data: any): { string }
		return prime_ids(ctx, data.take)
	end,
	each_card = draw_each,
}

-- "Draw every card with a prime index": every match in the deck as it stood when the effect began.
local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.for_each_card({ cards = "each_cards", each = "each_card" }),
		}
	end,
	each_cards = function(ctx: J.EffectContext): { string }
		return prime_ids(ctx, nil)
	end,
	each_card = draw_each,
}

return {
	base = base,
	radiant = radiant,
}
