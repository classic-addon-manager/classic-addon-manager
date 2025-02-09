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