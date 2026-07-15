function pad2(value: number) {
  return String(value).padStart(2, "0");
}

export function localMonth(date = new Date()) {
  return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}`;
}

export function localDate(date = new Date()) {
  return `${localMonth(date)}-${pad2(date.getDate())}`;
}

export function localDateTime(date = new Date()) {
  return `${localDate(date)}T${pad2(date.getHours())}:${pad2(date.getMinutes())}:${pad2(date.getSeconds())}`;
}
