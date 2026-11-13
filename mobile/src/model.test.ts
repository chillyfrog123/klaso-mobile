import { describe, it, expect } from "vitest";
import { band, attendanceDays, RequestGate, keyFor } from "./model";
import { en, pl } from "./i18n";
describe("grade boundaries", () => {
  it.each([
    ["2+", "low"],
    ["3−", "middle"],
    ["4+", "middle"],
    ["5−", "middle"],
    ["5", "high"],
    ["6", "excellent"],
    ["0", "neutral"],
    ["np", "neutral"],
  ])("%s is %s", (value, color) => expect(band(value)).toBe(color));
});
it("matches any attendance type but retains the entire day", () => {
  const items = [
    {
      date: "2026-11-09",
      lesson: "1",
      subject: "Fiction",
      kind: "Nieobecność usprawiedliwiona",
      present: false,
    },
    {
      date: "2026-11-09",
      lesson: "2",
      subject: "Fiction",
      kind: "Obecność",
      present: true,
    },
  ];
  expect(attendanceDays(items, [2])[0][1]).toHaveLength(2);
  expect(attendanceDays(items, [])).toEqual([]);
});
it("rejects stale responses across requests and accounts", () => {
  const g = new RequestGate();
  const old = g.begin("Grades");
  const current = g.begin("Grades");
  expect(old()).toBe(false);
  expect(current()).toBe(true);
  g.reset();
  expect(current()).toBe(false);
  expect(g.begin("Grades")()).toBe(true);
});
it("partitions week and page caches", () => {
  expect(keyFor("Timetable", "2026-11-09", 1)).not.toBe(
    keyFor("Timetable", "2026-11-16", 1),
  );
  expect(keyFor("Messages", "", 1)).not.toBe(keyFor("Messages", "", 2));
});
it("has identical translation keys", () =>
  expect(Object.keys(en).sort()).toEqual(Object.keys(pl).sort()));
