export const formatDate = (date: Date): string => {
    let hours = date.getHours() % 12;
    if (hours == 0) {
        hours = 1;
    }
    let minutes = date.getMinutes()
    let current = new Date();
    if (current.getTime() - date.getTime() <= 60_000) {
        return "less than a minute ago"
    } else {
        return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}`;
    }
}
