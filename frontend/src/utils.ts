import {toast as t, type ToastT} from "svelte-sonner";

/**
 * Formats a go date string to a human-readable date string.
 */
export function formatToLocalTime(dateString: string): string {
    const date = new Date(dateString);
    const options: Intl.DateTimeFormatOptions = {
        hour: '2-digit',
        minute: '2-digit',
        day: '2-digit',
        month: 'long',
        year: 'numeric'
    };
    return date.toLocaleString(undefined, options).replace(',', '');
}

/**
 * Takes a go date and determines how long ago it was in days.
 */
export function timeAgo(dateString: string): number {
    const date = new Date(dateString);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    return Math.floor(diff / (1000 * 60 * 60 * 24));
}

type ToastType = {
    description?: string,
    duration?: number,
    dismissable?: boolean
}

export let toast = {
    success: (message: string, options?: ToastType) => {
        let opts: ToastType = {
            duration: options?.duration || 5000,
            dismissable: options?.dismissable || true
        };
        if (options?.description) {
            opts.description = options?.description;
        }
        t.success(message, opts);
    },

    error: (message: string, options?: ToastType) => {
        let opts: ToastType = {
            duration: options?.duration || 5000,
            dismissable: options?.dismissable || true
        };
        if (options?.description) {
            opts.description = options?.description;
        }
        t.error(message, opts);
    },

    message: (message: string, options?: ToastType) => {
        let opts: ToastType = {
            duration: options?.duration || 5000,
            dismissable: options?.dismissable || true
        };
        if (options?.description) {
            opts.description = options?.description;
        }
        t.message(message, opts);
    },

    info: (message: string, options?: ToastType) => {
        let opts: ToastType = {
            duration: options?.duration || 5000,
            dismissable: options?.dismissable || true
        }
        if (options?.description) {
            opts.description = options?.description;
        }
        t.info(message, opts);
    }
}