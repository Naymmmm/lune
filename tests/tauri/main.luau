local tauri = require("@lune/tauri")

print("Tauri module loaded successfully")
print("Version:", tauri.version)

assert(type(tauri.run) == "function", "tauri.run should be a function")
assert(type(tauri.listen) == "function", "tauri.listen should be a function")
assert(type(tauri.new) == "function", "tauri.new should be a function")

-- Test tauri.new()
local app = tauri.new({
    name = "Test App",
    identifier = "org.test.app",
})
assert(app ~= nil, "tauri.new should return an app")
assert(type(app.listen) == "function", "app:listen should be a function")
assert(type(app.run) == "function", "app:run should be a function")

print("All assertions passed.")
