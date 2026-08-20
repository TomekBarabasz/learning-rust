local function untildot(s)
  return (string.gsub(tostring(s), "%..*$", ""))
end

local function taskPriority(t)
  local p = t.priority and string.lower(tostring(t.priority)) or "normal"
  if p == "high" or p == "h" or p == "1" then return 1
  elseif p == "low" or p == "l" or p == "3" then return 3
  else return 2 end
end

local q = query[[from t = index.tasks() where not t.done]]
if #q == 0 then return { empty = true } end
table.sort(q, function(a, b) return taskPriority(a) < taskPriority(b) end)
local t = q[1]
return { name = untildot(t.name), ref = t.ref }
