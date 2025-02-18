local api = require("api")
local classic_addon_manager_addon = {
	name = "Classic Addon Manager",
	author = "Sami",
	version = "1.0",
	desc = "Notifies whenever new addon updates are available!"
}

local camWindow

local function OnUpdate(dt)

end 

local function OnLoad()
	local settings = api.GetSettings("AddonUpdateNotification")

    local addonsToUpdate = api.File:Read("AddonUpdateNotification/updates.lua")
    if addonsToUpdate ~= nil then 
        api.Log:Info("[Classic Addon Manager] The following addons have updates available:")
        for addonId, updateInfo in pairs(addonsToUpdate) do
            api.Log:Info("[Classic Addon Manager] - " .. updateInfo.name .. " version: " .. updateInfo.version)
        end 
    end 

	camWindow = api.Interface:CreateWindow("camWindow", "Classic Addon Manager")
    camWindow:AddAnchor("CENTER", "UIParent", 0, 0)
    camWindow:Show(false)

    api.On("UPDATE", OnUpdate)
	api.SaveSettings()
end

local function OnUnload()
	api.On("UPDATE", function() return end)
    if camWindow ~= nil then 
        camWindow:Show(false)
    end 
	
    camWindow = nil
end

classic_addon_manager_addon.OnLoad = OnLoad
classic_addon_manager_addon.OnUnload = OnUnload

return classic_addon_manager_addon