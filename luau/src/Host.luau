--!strict
-- Host: the Luau side of the engine's C boundary.
--
-- Implements `crates/wasm/c/README.md` exactly: `Host.new` calls `init()` once,
-- `host:call` marshals each string argument as a `(ptr, len)` pair through
-- `jackioh_alloc` (an empty string is `(0, 0)` with no buffer), copies the answer
-- out of `jackioh_result_ptr`/`jackioh_result_len` before freeing the argument
-- buffers, and never caches the buffer `memory()` returns (`memory.grow` can
-- replace it at any point). JSON stays opaque text throughout.
--
-- `instance` is an adapter over whatever the translator produces; that adapter
-- arrives with the translation, in a later issue.

local Host: { [string]: any } = {}
Host.__index = Host

export type CInstance = {
	exports: { [string]: any },
	memory: () -> buffer,
}

export type Host = {
	_instance: CInstance,
	_dead: boolean,
}

local STATUS_FAILED = 1
local PANIC_PREFIX = "panicked at "

-- Copy the last binding's answer out of the module. The result buffer is the
-- module's: never free it, and never touch the pointer when the length is 0.
local function readResult(instance: CInstance): string
	local len: number = instance.exports.jackioh_result_len()
	if len == 0 then
		return ""
	end
	local ptr: number = instance.exports.jackioh_result_ptr()
	return buffer.readstring(instance.memory(), ptr, len)
end

local function freeAll(instance: CInstance, owned: { { number } }): ()
	for _, pair in owned do
		instance.exports.jackioh_free(pair[1], pair[2])
	end
end

function Host.new(instance: CInstance): Host
	instance.exports.init()
	return setmetatable({ _instance = instance, _dead = false }, Host)
end

function Host.call(self: Host, name: string, ...: any): string
	local instance: CInstance = self._instance
	if self._dead then
		error(name .. " refused: instance is dead; open a fresh one", 0)
	end
	local exports = instance.exports
	local binding = exports[name]
	if binding == nil then
		error(name .. " refused: unknown binding", 0)
	end
	local args = table.pack(...)
	for i = 1, args.n do
		local kind = type(args[i])
		if kind ~= "string" and kind ~= "number" then
			error(name .. " refused: argument " .. tostring(i) .. " is not a string or number", 0)
		end
	end
	local parts: { any } = {}
	local owned: { { number } } = {}
	for i = 1, args.n do
		local arg = args[i]
		if type(arg) == "string" then
			if #arg == 0 then
				table.insert(parts, 0)
				table.insert(parts, 0)
			else
				local ptr: number = exports.jackioh_alloc(#arg)
				buffer.writestring(instance.memory(), ptr, arg)
				table.insert(parts, ptr)
				table.insert(parts, #arg)
				table.insert(owned, { ptr, #arg })
			end
		else
			table.insert(parts, arg)
		end
	end
	local ok, status = pcall(binding, table.unpack(parts, 1, #parts))
	if not ok then
		freeAll(instance, owned)
		self._dead = true
		local text = readResult(instance)
		if string.sub(text, 1, #PANIC_PREFIX) ~= PANIC_PREFIX then
			text = "no text"
		end
		error(name .. " trapped: " .. text, 0)
	end
	local answer = readResult(instance)
	freeAll(instance, owned)
	if status == STATUS_FAILED then
		error(name .. " refused: " .. answer, 0)
	end
	return answer
end

local function skipSpace(s: string, pos: number): number
	while pos <= #s do
		local c = string.sub(s, pos, pos)
		if c ~= " " and c ~= "\t" and c ~= "\n" and c ~= "\r" then
			break
		end
		pos += 1
	end
	return pos
end

-- `s[pos]` is the opening quote. Returns the closing quote's index, skipping
-- `\X` escape pairs so an escaped quote never ends the string early.
local function scanString(s: string, pos: number): number
	local i = pos + 1
	while i <= #s do
		local byte = string.byte(s, i)
		if byte == 92 then
			i += 2
		elseif byte == 34 then
			return i
		else
			i += 1
		end
	end
	error("splitState: unterminated string", 0)
	return pos -- unreachable: error never returns
end

-- Returns the last index of the JSON value starting at `pos`. Objects and
-- arrays are walked with `scanString`/`scanValue` recursion so braces inside
-- strings never count; anything else (a number, `true`, `false`, `null`) runs
-- to the next `,`, `}` or `]`.
local function scanValue(s: string, pos: number): number
	pos = skipSpace(s, pos)
	if pos > #s then
		error("splitState: bad JSON", 0)
	end
	local c = string.sub(s, pos, pos)
	if c == '"' then
		return scanString(s, pos)
	elseif c == "{" then
		local i = skipSpace(s, pos + 1)
		if string.sub(s, i, i) == "}" then
			return i
		end
		while true do
			i = skipSpace(s, i)
			if string.sub(s, i, i) ~= '"' then
				error("splitState: bad JSON", 0)
			end
			i = scanString(s, i) + 1
			i = skipSpace(s, i)
			if string.sub(s, i, i) ~= ":" then
				error("splitState: bad JSON", 0)
			end
			i = scanValue(s, i + 1)
			i = skipSpace(s, i + 1)
			local d = string.sub(s, i, i)
			if d == "," then
				i += 1
			elseif d == "}" then
				return i
			else
				error("splitState: bad JSON", 0)
			end
		end
	elseif c == "[" then
		local i = skipSpace(s, pos + 1)
		if string.sub(s, i, i) == "]" then
			return i
		end
		while true do
			i = scanValue(s, i)
			i = skipSpace(s, i + 1)
			local d = string.sub(s, i, i)
			if d == "," then
				i += 1
			elseif d == "]" then
				return i
			else
				error("splitState: bad JSON", 0)
			end
		end
	else
		local stop = string.find(s, "[},%]]", pos)
		if stop == nil then
			error("splitState: bad JSON", 0)
		end
		return stop - 1
	end
end

-- Cut the `state` member out of a `{"state":…, …}` answer (`begin_game`,
-- `reduce`, `fold`) as its exact text. Never decode and re-encode it: a
-- re-encode can change the text, and the module caches states by exact text.
function Host.splitState(answer: string): string
	local i = skipSpace(answer, 1)
	if string.sub(answer, i, i) ~= "{" then
		error("splitState: answer is not an object", 0)
	end
	i = skipSpace(answer, i + 1)
	if string.sub(answer, i, i) == "}" then
		error("splitState: no state member", 0)
	end
	while true do
		i = skipSpace(answer, i)
		if string.sub(answer, i, i) ~= '"' then
			error("splitState: bad JSON", 0)
		end
		local keyEnd = scanString(answer, i)
		local key = string.sub(answer, i, keyEnd)
		i = skipSpace(answer, keyEnd + 1)
		if string.sub(answer, i, i) ~= ":" then
			error("splitState: bad JSON", 0)
		end
		local valueStart = skipSpace(answer, i + 1)
		local valueEnd = scanValue(answer, valueStart)
		if key == '"state"' then
			return string.sub(answer, valueStart, valueEnd)
		end
		i = skipSpace(answer, valueEnd + 1)
		local d = string.sub(answer, i, i)
		if d == "," then
			i += 1
		elseif d == "}" then
			error("splitState: no state member", 0)
		else
			error("splitState: bad JSON", 0)
		end
	end
end

return Host
