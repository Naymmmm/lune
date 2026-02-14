local fs = require("@lune/fs")
local task = require("@lune/task")
local tauri = require("@lune/tauri")

print("Starting File Browser App...")

-- Create app with config
local app = tauri.new({
    name = "Lune File Browser",
    identifier = "org.lune.filebrowser",
    version = "1.0.0",
    html = "./index.html",
    window = {
        title = "Lune File Browser",
        width = 900,
        height = 700
    }
})

-- Helper to safely get file info
local function getFileItems(dirPath)
    if dirPath == "" then dirPath = "." end
    
    -- Normalize path separators
    dirPath = dirPath:gsub("\\", "/")
    if dirPath:sub(-1) == "/" then
        dirPath = dirPath:sub(1, -2)
    end

    local entries = fs.readDir(dirPath)
    local items = {}

    for _, name in ipairs(entries) do
        local fullPath = dirPath .. "/" .. name
        local isDir = false
        local success, meta = pcall(fs.metadata, fullPath)
        
        if success then
            isDir = meta.kind == "dir"
        end

        table.insert(items, {
            name = name,
            isDir = isDir,
            path = fullPath
        })
    end

    -- Sort: directories first, then alphabetical
    table.sort(items, function(a, b)
        if a.isDir and not b.isDir then return true end
        if not a.isDir and b.isDir then return false end
        return a.name:lower() < b.name:lower()
    end)

    return items
end

app:listen("read-dir", function(path, handle)
    task.spawn(function()
        print("Request for directory:", path)
        
        local success, result = pcall(function()
            local items = getFileItems(path)
            return items
        end)

        if success then
            handle:emit("read-dir-response", {
                currentPath = path == "" and "." or path,
                items = result
            })
        else
            print("Error reading dir:", result)
            handle:emit("error", "Failed to read directory: " .. tostring(result))
        end
    end)
end)

print("Backend ready. Waiting for requests.")

app:run()
