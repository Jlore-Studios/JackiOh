--!strict
-- Mock: a Luau stand-in for the C module's memory interface, over a real
-- `buffer`. Implements `jackioh_alloc`/`jackioh_free` (tracking live
-- allocations and raising on a bad free), `jackioh_result_ptr`/
-- `jackioh_result_len`, `init`, and a few fake bindings. `grow` replaces the
-- buffer the way `memory.grow` would, so the host must re-fetch it.

local Mock: { [string]: any } = {}
Mock.__index = Mock

export type Mock = {
	_buf: buffer,
	_next: number,
	_live: { [number]: number },
	_resultPtr: number,
	_resultLen: number,
	initCalls: number,
	allocCalls: number,
	ptrCalls: number,
	bindingCalls: { [string]: number },
	growOnAlloc: boolean,
	growOnCall: boolean,
	exports: { [string]: any },
}

function Mock.new(): Mock
	local self: Mock = setmetatable({
		_buf = buffer.create(64),
		_next = 1,
		_live = {},
		_resultPtr = 0,
		_resultLen = 0,
		initCalls = 0,
		allocCalls = 0,
		ptrCalls = 0,
		bindingCalls = {},
		growOnAlloc = false,
		growOnCall = false,
		exports = {},
	}, Mock)
	self.exports.init = function()
		self.initCalls += 1
	end
	self.exports.jackioh_alloc = function(len: number): number
		self.allocCalls += 1
		if self.growOnAlloc then
			self.growOnAlloc = false
			self:grow()
		end
		self:ensure(self._next + len)
		local ptr = self._next
		self._next += math.max(len, 1)
		self._live[ptr] = len
		return ptr
	end
	self.exports.jackioh_free = function(ptr: number, len: number)
		if ptr == 0 then
			return
		end
		if self._live[ptr] ~= len then
			error("bad free: " .. tostring(ptr) .. "/" .. tostring(len), 0)
		end
		self._live[ptr] = nil
	end
	self.exports.jackioh_result_ptr = function(): number
		self.ptrCalls += 1
		return self._resultPtr
	end
	self.exports.jackioh_result_len = function(): number
		return self._resultLen
	end
	self.exports.echo = function(ptr: number, len: number): number
		self:count("echo")
		self:maybeGrow()
		self:setAnswer(buffer.readstring(self._buf, ptr, len))
		return 0
	end
	self.exports.concat = function(aPtr: number, aLen: number, bPtr: number, bLen: number): number
		self:count("concat")
		self:maybeGrow()
		local a = buffer.readstring(self._buf, aPtr, aLen)
		local b = buffer.readstring(self._buf, bPtr, bLen)
		self:setAnswer(a .. b)
		return 0
	end
	self.exports.addone = function(n: number): number
		self:count("addone")
		self:maybeGrow()
		self:setAnswer(tostring(n + 1))
		return 0
	end
	self.exports.ping = function(): number
		self:count("ping")
		self:maybeGrow()
		self:setAnswer("pong")
		return 0
	end
	self.exports.empty = function(): number
		self:count("empty")
		self:maybeGrow()
		self:setAnswer("")
		return 0
	end
	self.exports.refuse = function(): number
		self:count("refuse")
		self:maybeGrow()
		self:setAnswer("nope: bad deck")
		return 1
	end
	self.exports.panic = function(): number
		self:count("panic")
		self:maybeGrow()
		self:setAnswer("panicked at 'boom', src/lib.rs:1:1")
		error("unreachable", 0)
		return 0
	end
	self.exports.silent = function(): number
		self:count("silent")
		self:maybeGrow()
		error("boom", 0)
		return 0
	end
	return self
end

function Mock.count(self: Mock, name: string): ()
	self.bindingCalls[name] = (self.bindingCalls[name] or 0) + 1
end

function Mock.grow(self: Mock): ()
	local old = self._buf
	local new = buffer.create(buffer.len(old) * 2)
	buffer.copy(new, 0, old, 0, buffer.len(old))
	self._buf = new
end

function Mock.ensure(self: Mock, need: number): ()
	while buffer.len(self._buf) < need do
		self:grow()
	end
end

function Mock.setAnswer(self: Mock, text: string): ()
	if #text == 0 then
		self._resultPtr = 0
		self._resultLen = 0
		return
	end
	self:ensure(self._next + #text)
	local ptr = self._next
	self._next += #text
	buffer.writestring(self._buf, ptr, text)
	self._resultPtr = ptr
	self._resultLen = #text
end

function Mock.maybeGrow(self: Mock): ()
	if self.growOnCall then
		self.growOnCall = false
		self:grow()
	end
end

function Mock.liveCount(self: Mock): number
	local n = 0
	for _ in self._live do
		n += 1
	end
	return n
end

function Mock.instance(self: Mock): { [string]: any }
	return {
		exports = self.exports,
		memory = function(): buffer
			return self._buf
		end,
	}
end

return Mock
