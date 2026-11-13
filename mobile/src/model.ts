export type Tab =
  "Timetable" | "Grades" | "Attendance" | "Messages" | "Profile";
export type Section = Exclude<Tab, "Profile"> | "Notices";
export type Grade = {
  subject: string;
  value: string;
  category: string;
  date: string;
  teacher: string;
  semester: string;
  numeric: number | null;
  counts: boolean;
  weight: number | null;
};
export type Behaviour = {
  value: string;
  description: string;
  semester: string;
  date: string;
};
export type Pair = [number | null, number | null];
export type Grades = {
  items: Grade[];
  behaviour: Behaviour[];
  behaviour_error: string | null;
  summaries: { overall: Pair; subjects: Record<string, Pair> }[];
};
export type Attendance = {
  date: string;
  lesson: string;
  subject: string;
  kind: string;
  present: boolean | null;
};
export type Lesson = {
  date: string;
  number: string;
  subject: string;
  teacher: string;
  room: string;
  from: string;
  to: string;
  canceled: boolean;
  substitution: boolean;
  note: string;
};
export type Message = {
  id: string;
  title: string;
  sender: string;
  content: string;
  date: string;
  unread: boolean;
  attachments: string[];
  has_files: boolean;
};
export type Notice = {
  id: string;
  title: string;
  content: string;
  date: string;
  until: string;
  author: string;
  unread: boolean;
};
export type Payload = {
  Grades?: Grades;
  Attendance?: Attendance[];
  Timetable?: Lesson[];
  Messages?: { items: Message[]; total: number };
  Notices?: Notice[];
};
export type Snapshot = { payload: Payload; updated: string };
export type Archive = {
  snapshots: Record<string, Snapshot>;
  details: Record<string, Message>;
};
export type Account = {
  owner: string;
  session: boolean;
  remembered: boolean;
  authenticated?: boolean;
};
export const emptyArchive = (): Archive => ({ snapshots: {}, details: {} });
export function keyFor(section: Section, week: string, page: number) {
  return section === "Timetable"
    ? `${section}:${week}`
    : section === "Messages"
      ? `${section}:${page}`
      : section;
}
export function monday(date = new Date()) {
  const d = new Date(
    date.toLocaleString("en-US", { timeZone: "Europe/Warsaw" }),
  );
  d.setHours(12, 0, 0, 0);
  d.setDate(d.getDate() - ((d.getDay() + 6) % 7));
  return localDate(d);
}
export function localDate(d: Date) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}
export function addDays(date: string, n: number) {
  const d = new Date(date + "T12:00:00");
  d.setDate(d.getDate() + n);
  return localDate(d);
}
export function band(value: string) {
  const match = value
    .trim()
    .replace("−", "-")
    .match(/^([1-6])([+-]?)$/);
  if (!match) return "neutral";
  const n =
    Number(match[1]) + (match[2] === "+" ? 0.5 : match[2] === "-" ? -0.25 : 0);
  return n >= 6 ? "excellent" : n >= 5 ? "high" : n >= 2.75 ? "middle" : "low";
}
export function attendanceType(a: Attendance) {
  const s = a.kind
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replaceAll("ł", "l");
  if (s.includes("spozn") || s === "s" || s === "late") return 4;
  if (s === "z" || s.includes("zwoln") || s === "released") return 3;
  if (s.includes("nieuspraw")) return 1;
  if (s === "nu" || s.includes("uspraw") || s === "excused absence") return 2;
  if (s === "n" || s.includes("nieobecn") || s === "absent") return 1;
  if (s === "o" || s.includes("obecn") || a.present === true) return 0;
  return 5;
}
export function attendanceDays(items: Attendance[], selected: number[]) {
  const days: Record<string, Attendance[]> = {};
  for (const a of items) (days[a.date.slice(0, 10)] ??= []).push(a);
  return Object.entries(days)
    .filter(([, rows]) =>
      rows.some((a) => selected.includes(attendanceType(a))),
    )
    .sort(([a], [b]) => b.localeCompare(a));
}
export class RequestGate {
  private generation = 0;
  private tickets = new Map<string, number>();
  reset() {
    this.generation++;
    this.tickets.clear();
  }
  begin(key: string) {
    const epoch = this.generation;
    const ticket = (this.tickets.get(key) ?? 0) + 1;
    this.tickets.set(key, ticket);
    return () => epoch === this.generation && this.tickets.get(key) === ticket;
  }
}
