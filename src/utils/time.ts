export function countdown(resetsAt: string | null, now = new Date()): string | null {
  if (!resetsAt) return null;
  const milliseconds = new Date(resetsAt).getTime() - now.getTime();
  if (!Number.isFinite(milliseconds)) return null;
  if (milliseconds <= 0) return "Refreshing";

  const totalMinutes = Math.ceil(milliseconds / 60_000);
  const days = Math.floor(totalMinutes / 1_440);
  const hours = Math.floor((totalMinutes % 1_440) / 60);
  const minutes = totalMinutes % 60;
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

export function displayTimestamp(iso: string | null): string {
  if (!iso) return "Not checked";
  const date = new Date(iso);
  return Number.isNaN(date.getTime())
    ? "Unknown"
    : date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}
