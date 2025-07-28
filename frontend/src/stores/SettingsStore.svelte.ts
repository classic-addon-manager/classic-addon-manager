import {ApplicationService} from '$lib/wails'

let isInitialized: boolean = $state(false)
let pathDetection: boolean = $state(false)
let aacPath: string = $state('');

interface Settings {
    "general"?: {
        "aacpath"?: string,
        "autodetectpath"?: boolean,
    },

    [key: string]: any
}

async function initializeDefaults() {
    if (!isInitialized) {
        try {
            const defaults: Settings = await ApplicationService.GetConfig();
            pathDetection = defaults.general?.autodetectpath ?? true;
            aacPath = defaults.general?.aacpath ?? '';

            isInitialized = true;
        } catch (error) {
            // Keep the fallback default (false) if async call fails
            console.warn('Failed to load default path detection setting:', error);
            isInitialized = true;
        }
    }
}

export function setAutoPathDetection(value: boolean) {
    pathDetection = value;
}

export function autoPathDetection() {
    return pathDetection;
}

export function getAACPath(): string {
    return aacPath;
}

initializeDefaults().catch(console.error);