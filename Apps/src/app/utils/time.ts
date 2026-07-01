export function relativeTime(date?: Date) {
  if (!date) return "刚刚";

  const diff = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000));
  if (diff < 60) return `${diff}秒前`;
  if (diff < 3_600) return `${Math.floor(diff / 60)}分钟前`;
  if (diff < 86_400) return `${Math.floor(diff / 3_600)}小时前`;
  return `${Math.floor(diff / 86_400)}天前`;
}

export function formatTime() {
  return new Date().toLocaleTimeString();
}
