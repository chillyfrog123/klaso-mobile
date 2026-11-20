import { describe, it, expect } from "vitest";
import {
  band,
  attendanceDays,
  RequestGate,
  keyFor,
  gradeKeys,
  mergeArchive,
  gradesThisWeek,
  type Grade,
} from "./model";
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
it("keeps duplicate grade identities aligned with Android, including mixed stable IDs", () => {
  const grade: Grade = {
    subject: "Math / A",
    value: "5",
    category: "Test",
    date: "2026-11-17",
    teacher: "Teacher",
    semester: "1",
    numeric: 5,
    weight: 1,
    counts: true,
  };
  const base = JSON.stringify([
    grade.subject,
    grade.value,
    grade.category,
    grade.date,
    grade.teacher,
    grade.semester,
  ]);
  expect(gradeKeys([{ ...grade, id: "1" }, grade, { ...grade }])).toEqual([
    "id:1",
    base + "#1",
    base + "#2",
  ]);
});
it("never rolls back newer cached grades or new-grade markers after an older sync", () => {
  const latest = {
    snapshots: { Grades: { payload: {}, updated: "2026-11-17T12:00:00Z" } },
    details: {},
    activity: {
      newGradeIds: ["id:2"],
      baselineReady: true,
      updated: "2026-11-17T12:00:00Z",
    },
  };
  const stale = {
    snapshots: { Grades: { payload: {}, updated: "2026-11-17T11:00:00Z" } },
    details: {},
    activity: {
      newGradeIds: [],
      baselineReady: true,
      updated: "2026-11-17T11:00:00Z",
    },
  };
  expect(mergeArchive(latest, stale)).toEqual(latest);
});
describe("grades dated in the current school week", () => {
  const grade = (date: string): Grade => ({
    subject: "Math",
    value: "5",
    category: "Test",
    date,
    teacher: "Teacher",
    semester: "1",
    numeric: 5,
    counts: true,
    weight: 1,
  });
  it("includes Monday through Sunday, rather than the last seven days", () => {
    const items = [
      "2026-11-15",
      "2026-11-16",
      "2026-11-20",
      "2026-11-22",
      "2026-11-23",
    ].map(grade);
    expect(
      gradesThisWeek(items, new Date("2026-11-20T12:00:00Z")).map(
        (g) => g.date,
      ),
    ).toEqual(["2026-11-16", "2026-11-20", "2026-11-22"]);
  });
  it.each([
    ["2026-11-15T22:59:59Z", ["2026-11-15"]],
    ["2026-11-15T23:00:00Z", ["2026-11-16"]],
    ["2026-07-05T21:59:59Z", ["2026-07-05"]],
    ["2026-07-05T22:00:00Z", ["2026-07-06"]],
  ])("uses Warsaw's Monday boundary at %s", (now, expected) => {
    const items = ["2026-11-15", "2026-11-16", "2026-07-05", "2026-07-06"].map(
      grade,
    );
    expect(gradesThisWeek(items, new Date(now)).map((g) => g.date)).toEqual(
      expected,
    );
  });
  it("keeps one school week across the year boundary", () => {
    const items = [
      "2025-12-28",
      "2025-12-29",
      "2026-01-01",
      "2026-01-04",
      "2026-01-05",
    ].map(grade);
    expect(
      gradesThisWeek(items, new Date("2026-01-01T12:00:00Z")).map(
        (g) => g.date,
      ),
    ).toEqual(["2025-12-29", "2026-01-01", "2026-01-04"]);
  });
  it("excludes missing, malformed, and impossible grade dates", () => {
    const items = [
      "",
      "not a date",
      "2026-02-30",
      "2026-02-29",
      "2026-2-23",
      "2026-02-23junk",
      "2026-02-23",
      "2026-03-01",
    ].map(grade);
    expect(
      gradesThisWeek(items, new Date("2026-02-25T12:00:00Z")).map(
        (g) => g.date,
      ),
    ).toEqual(["2026-02-23", "2026-03-01"]);
  });
});
