-- Everything the lint allows, and its refused names only where they are not code: in this comment
-- (pairs, next, table.sort, math.random, print, os), in strings and as a field.
local J = require("@jackioh")

local words = { "pairs", 'next', [[table.sort]], [==[ a / b ^ c ]==] }
local holder = { ["next"] = 3 }

--[[ A block comment: for k, v in pairs(t) do end, n / 2, math.sqrt(n). ]]

local function total(list: { number }): number
	local sum = 0
	for _, n in ipairs(list) do
		sum += n
	end
	for i = 1, #list do
		sum += i % 2
	end
	return sum
end

return {
	base = {
		cry = function(ctx)
			local sum = total({ 7, 5, holder.next, #words })
			local amount = math.clamp(math.floor(sum // 3), 0, math.max(1, math.min(10, math.abs(-9))))
			local note = `{sum // 2} of {sum}`
			return { J.damage({ to = { of = "enemyHero" }, amount = amount + #note - #note }) }
		end,
	},
}
