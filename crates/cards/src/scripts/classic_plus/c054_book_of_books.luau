--!strict
local J = require("@jackioh")

local function book_of_books(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.add_random_from_catalog({
					query = { tags = { "Book" } },
					count = ctx:param("books"),
					-- "Each costs (0)": the declared number `setCost` (R386).
					costOverride = ctx:param("setCost"),
					radiant = radiant,
					temporary = true,
				}),
			}
		end,
	}
end

return {
	base = book_of_books(false),
	radiant = book_of_books(true),
}
