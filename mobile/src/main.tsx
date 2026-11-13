import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { App as NativeApp } from "@capacitor/app";
import {
  CalendarDays,
  GraduationCap,
  CheckCheck,
  Mail,
  UserRound,
  ChevronLeft,
  ChevronRight,
  RefreshCw,
  ArrowUpRight,
  ShieldCheck,
  Search,
  X,
  Paperclip,
  ArrowLeft,
  BookOpen,
  SlidersHorizontal,
  LogOut,
  Sparkles,
  Check,
  WifiOff,
} from "lucide-react";
import { en, pl, type Label } from "./i18n";
import {
  bridge,
  native,
  official,
  openGithub,
  errorCode,
  demoData,
} from "./bridge";
import {
  type Tab,
  type Section,
  type Grade,
  type Lesson,
  type Message,
  type Archive,
  type Account,
  type Pair,
  emptyArchive,
  monday,
  addDays,
  keyFor,
  band,
  attendanceDays,
  attendanceType,
  RequestGate,
} from "./model";
import { Choice } from "./Choice";
import "./styles.css";
import "./material.css";
import "./updates.css";
import { applyPlatformDesign } from "./design";
applyPlatformDesign();
const tabs: Tab[] = [
  "Timetable",
  "Grades",
  "Attendance",
  "Messages",
  "Profile",
];
const icons = {
  Timetable: CalendarDays,
  Grades: GraduationCap,
  Attendance: CheckCheck,
  Messages: Mail,
  Profile: UserRound,
};
const accents = ["Cobalt", "Emerald", "Violet", "Amber"] as const;
function preference(key: string, fallback: string) {
  try {
    return localStorage.getItem(key) ?? fallback;
  } catch {
    return fallback;
  }
}
function App() {
  const [lang, setLang] = useState(preference("language", "en"));
  const [mode, setMode] = useState(preference("theme", "system"));
  const [savedAccounts, setSavedAccounts] = useState<string[]>([]);
  const [addAccount, setAddAccount] = useState(false);
  const [about, setAbout] = useState(false);
  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");
    const apply = () =>
      (document.documentElement.dataset.mode =
        mode === "system" ? (media.matches ? "dark" : "light") : mode);
    apply();
    localStorage.setItem("theme", mode);
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [mode]);
  const [accent, setAccent] = useState(preference("accent", "Cobalt"));
  const t = (key: Label) => (lang === "pl" ? pl : en)[key];
  const [tab, setTab] = useState<Tab>("Timetable");
  const [inbox, setInbox] = useState(true);
  const [account, setAccount] = useState<Account | null>(null);
  const [archive, setArchive] = useState<Archive>(emptyArchive);
  const [demo, setDemo] = useState(false);
  const [ready, setReady] = useState(false);
  const [week, setWeek] = useState(monday());
  const [day, setDay] = useState(Math.min((new Date().getDay() + 6) % 7, 6));
  const [viewWeek, setViewWeek] = useState(false);
  const [semester, setSemester] = useState(0);
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState("");
  const [filters, setFilters] = useState([0, 1, 2, 3, 4, 5]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<Label | null>(null);
  const [detailError, setDetailError] = useState<Label | null>(null);
  const [reading, setReading] = useState<Message | null>(null);
  const [detailBusy, setDetailBusy] = useState(false);
  const [sheet, setSheet] = useState<{ grade?: Grade; lesson?: Lesson } | null>(
    null,
  );
  const gate = useRef(new RequestGate());
  const restore = useRef<{ account: Account | null; archive: Archive } | null>(
    null,
  );
  const dialog = useRef<HTMLDialogElement>(null);
  const section: Section =
    tab === "Profile"
      ? "Grades"
      : tab === "Messages" && !inbox
        ? "Notices"
        : tab;
  const cacheKey = keyFor(section, week, page);
  const visibleView = useRef("");
  visibleView.current = `${tab}:${cacheKey}`;
  const snapshot = archive.snapshots[cacheKey];
  const payload = snapshot?.payload;
  const date = (value: string, full = false) => {
    if (!value) return "—";
    const d = new Date(
      value.length === 10 ? value + "T12:00:00" : value.replace(" ", "T"),
    );
    return isNaN(+d)
      ? value
      : new Intl.DateTimeFormat(
          lang === "pl" ? "pl-PL" : "en-GB",
          full
            ? { day: "numeric", month: "long", year: "numeric" }
            : { day: "numeric", month: "short" },
        ).format(d);
  };
  const pair = (p: Pair | undefined) =>
    `[${p?.[0]?.toFixed(2) ?? "—"}]/[${p?.[1]?.toFixed(2) ?? "—"}]`;
  useEffect(() => {
    document.documentElement.lang = lang;
    document.documentElement.dataset.accent = accent;
    try {
      localStorage.setItem("language", lang);
      localStorage.setItem("accent", accent);
    } catch {
      /* preferences are non-sensitive */
    }
    if (native) void bridge.language({ language: lang }).catch(() => {});
  }, [lang, accent]);
  useEffect(() => {
    let active = true;
    if (native)
      bridge
        .bootstrap()
        .then((data) => {
          if (active) {
            setSavedAccounts(data.accounts ?? []);
            setAccount(data.account);
            setArchive(data.archive);
            // A cached previous week is useful offline; show its explicit dates
            // instead of an empty current week on the first launch of a new week.
            if (!data.archive.snapshots[keyFor("Timetable", monday(), 1)]) {
              const lastWeek = Object.entries(data.archive.snapshots)
                .filter(([key]) => key.startsWith("Timetable:"))
                .sort(([, a], [, b]) => b.updated.localeCompare(a.updated))[0];
              if (lastWeek) setWeek(lastWeek[0].slice("Timetable:".length));
            }
          }
        })
        .catch(() => {
          if (active) setError("storageFailed");
        })
        .finally(() => {
          if (active) setReady(true);
        });
    else setReady(true);
    return () => {
      active = false;
    };
  }, []);
  async function refresh() {
    if (!account?.session && !demo) {
      setError("signedOut");
      return;
    }
    const valid = gate.current.begin("load:" + cacheKey);
    const view = `${tab}:${cacheKey}`;
    setBusy(true);
    setError(null);
    try {
      const result = demo
        ? {
            payload: (await demoData(week))[section],
            updated: new Date().toISOString(),
          }
        : await bridge.load({ owner: account!.owner, section, week, page });
      if (valid()) {
        if (!demo)
          setAccount(
            (previous) => previous && { ...previous, authenticated: true },
          );
        setArchive((prev) => ({
          ...prev,
          snapshots: { ...prev.snapshots, [cacheKey]: result },
        }));
      }
    } catch (e) {
      if (valid() && visibleView.current === view) setError(errorCode(e));
    } finally {
      if (valid() && visibleView.current === view) setBusy(false);
    }
  }
  useEffect(() => {
    setError(null);
    setBusy(false);
    setSearch("");
    setReading(null);
    setDetailError(null);
    setDetailBusy(false);
    gate.current.begin("detail");
  }, [cacheKey, tab, account?.owner, demo]);
  useEffect(() => {
    if (
      ready &&
      tab !== "Profile" &&
      (demo || account?.session) &&
      !archive.snapshots[cacheKey]
    )
      void refresh();
  }, [cacheKey, tab, ready, account?.owner, demo]);
  useEffect(() => {
    if (sheet && !dialog.current?.open) dialog.current?.showModal();
    if (!sheet && dialog.current?.open) dialog.current.close();
  }, [sheet]);
  const backRef = useRef<() => boolean>(() => false);
  backRef.current = () => {
    const choice = document.querySelector<HTMLDialogElement>(
      "dialog.choice[open]",
    );
    if (choice) {
      choice.close();
      return true;
    }
    if (about) {
      setAbout(false);
      return true;
    }
    if (sheet) {
      setSheet(null);
      return true;
    }
    if (reading) {
      setReading(null);
      gate.current.begin("detail");
      return true;
    }
    if (tab !== "Timetable") {
      setTab("Timetable");
      return true;
    }
    return false;
  };
  useEffect(() => {
    const escape = (e: KeyboardEvent) => {
      if (e.key === "Escape") backRef.current();
    };
    window.addEventListener("keydown", escape);
    const listener = native
      ? NativeApp.addListener("backButton", () => {
          if (!backRef.current()) void NativeApp.minimizeApp();
        })
      : null;
    return () => {
      window.removeEventListener("keydown", escape);
      void listener?.then((l) => l.remove());
    };
  }, []);
  async function startDemo() {
    restore.current = { account, archive };
    gate.current.reset();
    setError(null);
    setDemo(true);
    setAccount(null);
    setArchive(emptyArchive());
    setTab("Grades");
  }
  function exitDemo() {
    gate.current.reset();
    setDemo(false);
    setAccount(restore.current?.account ?? null);
    setArchive(restore.current?.archive ?? emptyArchive());
    restore.current = null;
    setTab("Profile");
    setReading(null);
    setSheet(null);
    setError(null);
  }
  async function login(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!native) {
      setError("nativeOnly");
      return;
    }
    const form = event.currentTarget;
    const data = new FormData(form);
    gate.current.reset();
    const valid = gate.current.begin("auth");
    setBusy(true);
    setError(null);
    setAccount(null);
    setArchive(emptyArchive());
    try {
      const result = await bridge.login({
        login: String(data.get("login")).trim(),
        password: String(data.get("password")),
        remember: data.get("remember") === "on",
      });
      if (valid()) {
        setSavedAccounts(result.accounts ?? []);
        setAddAccount(false);
        setAccount(result.account);
        setArchive(result.archive);
        form.reset();
        setTab("Grades");
      }
    } catch (e) {
      if (valid()) setError(errorCode(e));
    } finally {
      if (valid()) setBusy(false);
    }
  }
  async function logout() {
    gate.current.reset();
    setAccount(null);
    setArchive(emptyArchive());
    setReading(null);
    setSheet(null);
    setError(null);
    setBusy(false);
    if (native)
      try {
        await bridge.logout();
        const restored = await bridge.bootstrap();
        setSavedAccounts(restored.accounts ?? []);
      } catch {
        setError("storageFailed");
      }
  }
  async function openMessage(message: Message) {
    const valid = gate.current.begin("detail");
    setReading(archive.details[message.id] ?? message);
    setDetailError(null);
    if (demo) {
      setArchive((prev) => ({
        ...prev,
        snapshots: {
          ...prev.snapshots,
          [cacheKey]: {
            ...prev.snapshots[cacheKey],
            payload: {
              Messages: {
                ...payload!.Messages!,
                items: payload!.Messages!.items.map((m) =>
                  m.id === message.id ? { ...m, unread: false } : m,
                ),
              },
            },
          },
        },
      }));
      return;
    }
    if (!account?.session) {
      setDetailError("signedOut");
      return;
    }
    setDetailBusy(true);
    try {
      const { message: full } = await bridge.detail({
        owner: account.owner,
        id: message.id,
      });
      if (valid()) {
        setReading(full);
        setArchive((prev) => ({
          ...prev,
          details: { ...prev.details, [full.id]: full },
          snapshots: {
            ...prev.snapshots,
            [cacheKey]: {
              ...prev.snapshots[cacheKey],
              payload: {
                Messages: {
                  ...prev.snapshots[cacheKey].payload.Messages!,
                  items: prev.snapshots[cacheKey].payload.Messages!.items.map(
                    (m) => (m.id === full.id ? { ...m, unread: false } : m),
                  ),
                },
              },
            },
          },
        }));
      }
    } catch (e) {
      if (valid()) setDetailError(errorCode(e));
    } finally {
      if (valid()) setDetailBusy(false);
    }
  }
  const empty = () => (
    <div className="empty">
      <BookOpen size={34} />
      <h3>{t("empty")}</h3>
      <p>{t(account || demo ? "emptyHint" : "signInHint")}</p>
      {!account && !demo && (
        <button className="primary" onClick={() => setTab("Profile")}>
          {t("signIn")}
        </button>
      )}
    </div>
  );
  const status = (code: Label | null) =>
    code && (
      <div className="error" role="alert">
        <p>{t(code)}</p>
        {code === "verification" && (
          <button onClick={() => void official()}>
            {t("official")} <ArrowUpRight size={16} />
          </button>
        )}
      </div>
    );
  const closeReader = () => {
    setReading(null);
    gate.current.begin("detail");
    setDetailBusy(false);
  };
  const reader = () => (
    <article className={"reader card " + (reading ? "has-message" : "")}>
      {reading ? (
        <>
          <button className="text-button" onClick={closeReader}>
            <ArrowLeft size={18} />
            {t("back")}
          </button>
          <span className="eyebrow">{t("reader")}</span>
          <h2>{reading.title}</h2>
          <p className="sender">{reading.sender}</p>
          <p className="muted">{date(reading.date, true)}</p>
          {detailBusy && <p role="status">{t("loading")}</p>}
          {status(detailError)}
          <div className="message-body">{reading.content}</div>
          <div className="attachment-box">
            <h3>
              <Paperclip size={18} />
              {t("attachments")}
            </h3>
            {reading.attachments.length ? (
              reading.attachments.map((a, i) => <p key={i}>{a}</p>)
            ) : (
              <p>
                {t(reading.has_files ? "attachmentUnknown" : "noAttachments")}
              </p>
            )}
            {(reading.has_files || reading.attachments.length > 0) && (
              <>
                <p className="muted">{t("attachmentHint")}</p>
                <button className="text-button" onClick={() => void official()}>
                  {t("official")}
                  <ArrowUpRight size={16} />
                </button>
              </>
            )}
          </div>
        </>
      ) : (
        <div className="empty">
          <Mail size={36} />
          <p>{t("chooseMessage")}</p>
        </div>
      )}
    </article>
  );
  function timetable() {
    const lessons = payload?.Timetable;
    if (!lessons) return empty();
    const days = Array.from({ length: 7 }, (_, i) => addDays(week, i));
    return (
      <>
        <div className="week-toolbar glass">
          <button
            aria-label={t("previousWeek")}
            onClick={() => setWeek(addDays(week, -7))}
          >
            <ChevronLeft />
          </button>
          <div>
            <strong>
              {date(week)} – {date(addDays(week, 6))}
            </strong>
            <button
              className="text-button"
              onClick={() => {
                setWeek(monday());
                setDay((new Date().getDay() + 6) % 7);
              }}
            >
              {t("today")}
            </button>
          </div>
          <button
            aria-label={t("nextWeek")}
            onClick={() => setWeek(addDays(week, 7))}
          >
            <ChevronRight />
          </button>
          <button
            className="week-toggle"
            aria-pressed={viewWeek}
            onClick={() => setViewWeek(!viewWeek)}
          >
            {t(viewWeek ? "day" : "week")}
          </button>
        </div>
        <div className="day-picker">
          {days.map((d, i) => (
            <button
              key={d}
              aria-pressed={day === i}
              className={day === i ? "active" : ""}
              onClick={() => setDay(i)}
            >
              <span>
                {new Intl.DateTimeFormat(lang === "pl" ? "pl-PL" : "en", {
                  weekday: "short",
                }).format(new Date(d + "T12:00:00"))}
              </span>
              <strong>{Number(d.slice(-2))}</strong>
              <i className={lessons.some((l) => l.date === d) ? "dot" : ""} />
            </button>
          ))}
        </div>
        <div className={viewWeek ? "days week-view" : "days"}>
          {days.map((d, i) => {
            const rows = lessons.filter((l) => l.date === d);
            return (
              <section
                key={d}
                className={"day-column " + (day !== i ? "inactive-day" : "")}
              >
                <h2>
                  {new Intl.DateTimeFormat(lang === "pl" ? "pl-PL" : "en", {
                    weekday: "long",
                  }).format(new Date(d + "T12:00:00"))}
                  <span>{date(d)}</span>
                </h2>
                {!rows.length ? (
                  <div className="day-off">
                    <CalendarDays size={24} />
                    <strong>{t("dayOff")}</strong>
                    <p>{t("noLessons")}</p>
                  </div>
                ) : (
                  rows.map((l, index) => (
                    <button
                      key={index}
                      className={
                        "lesson-card " +
                        (l.canceled
                          ? "canceled"
                          : l.substitution
                            ? "substitution"
                            : "")
                      }
                      onClick={() => setSheet({ lesson: l })}
                    >
                      <div className="lesson-time">
                        <strong>{l.from}</strong>
                        <span>{l.to}</span>
                      </div>
                      <div className="lesson-info">
                        <span className="lesson-number">
                          {t("lesson")} {l.number}
                        </span>
                        <h3>{l.subject}</h3>
                        <p>{l.teacher}</p>
                        <small>
                          {t("room")} {l.room}
                        </small>
                        {(l.canceled || l.substitution) && (
                          <b>{t(l.canceled ? "canceled" : "substitution")}</b>
                        )}
                      </div>
                      <ChevronRight size={18} />
                    </button>
                  ))
                )}
              </section>
            );
          })}
        </div>
      </>
    );
  }
  function grades() {
    const grades = payload?.Grades;
    if (!grades) return empty();
    const rows = grades.items.filter(
      (g) => !semester || g.semester === String(semester),
    );
    const subjects = [...new Set(rows.map((g) => g.subject))].sort();
    return (
      <>
        <article className="card behaviour">
          <div className="section-toolbar">
            <h2>{t("behaviour")}</h2>
            <ShieldCheck size={21} />
          </div>
          <p className="muted">{t("behaviourHint")}</p>
          {grades.behaviour
            .filter((b) => !semester || b.semester === String(semester))
            .map((b, i) => (
              <div className="behaviour-row" key={i}>
                <strong>{b.value}</strong>
                <span>{b.description}</span>
                <small>{date(b.date)}</small>
              </div>
            ))}
        </article>
        <div className="summary-card">
          <div>
            <span className="eyebrow">{t("overall")}</span>
            <h2>{pair(grades.summaries[semester]?.overall)}</h2>
            <p>{t("averageHint")}</p>
          </div>
          <div className="summary-icon">
            <GraduationCap size={34} />
          </div>
          <div className="summary-footer">
            <span>
              {subjects.length} {t("subjects").toLowerCase()}
            </span>
            <span>
              {rows.length} {t("Grades").toLowerCase()}
            </span>
          </div>
        </div>
        <div className="section-toolbar">
          <h2>{t("subjects")}</h2>
          <label>
            <span className="sr-only">{t("semester")}</span>
            <Choice
              label={t("semester")}
              value={semester}
              onChange={(v) => setSemester(Number(v))}
              close={t("close")}
              options={[
                { value: 0, label: t("allSemesters") },
                { value: 1, label: t("semester") + " 1" },
                { value: 2, label: t("semester") + " 2" },
              ]}
            />
          </label>
        </div>
        <div className="subjects">
          {subjects.map((subject, index) => (
            <article className="subject card" key={subject}>
              <div className="subject-heading">
                <span className="subject-icon">
                  {String(index + 1).padStart(2, "0")}
                </span>
                <h3>{subject}</h3>
                <span className="average" title={t("averageHint")}>
                  {pair(grades.summaries[semester]?.subjects[subject])}
                </span>
              </div>
              <div className="grade-row">
                {rows
                  .filter((g) => g.subject === subject)
                  .map((g, i) => (
                    <button
                      key={i}
                      className={"grade " + band(g.value)}
                      aria-label={`${subject} ${g.value} · ${t("details")}`}
                      onClick={() => setSheet({ grade: g })}
                    >
                      {g.value}
                    </button>
                  ))}
              </div>
            </article>
          ))}
        </div>
        {!subjects.length && empty()}
      </>
    );
  }
  function attendance() {
    const items = payload?.Attendance;
    if (!items) return empty();
    const labels: Label[] = [
      "present",
      "absent",
      "excused",
      "released",
      "late",
      "other",
    ];
    const codes =
      lang === "pl"
        ? ["O", "N", "NU", "Z", "S", "?"]
        : ["P", "A", "EA", "R", "L", "?"];
    const days = attendanceDays(items, filters);
    return (
      <>
        <div className="card filter-card">
          <h2>
            <SlidersHorizontal size={18} />
            {t("filters")}
          </h2>
          <div className="filter-chips">
            {labels.map((label, i) => (
              <button
                key={label}
                aria-pressed={filters.includes(i)}
                onClick={() =>
                  setFilters((prev) =>
                    prev.includes(i)
                      ? prev.filter((x) => x !== i)
                      : [...prev, i],
                  )
                }
                className={filters.includes(i) ? "active" : ""}
              >
                <b>{codes[i]}</b>
                {t(label)}
              </button>
            ))}
          </div>
          <div className="filter-actions">
            <button onClick={() => setFilters([0, 1, 2, 3, 4, 5])}>
              {t("selectAll")}
            </button>
            <button onClick={() => setFilters([])}>{t("clearAll")}</button>
          </div>
          <p className="muted">{t("filterHint")}</p>
        </div>
        {days.map(([dateKey, rows]) => (
          <section className="attendance-day" key={dateKey}>
            <div className="section-toolbar">
              <h2>{date(dateKey, true)}</h2>
              <small>
                {rows.length} {t("entries")}
              </small>
            </div>
            <div className="card">
              {[...rows]
                .sort((a, b) => Number(a.lesson) - Number(b.lesson))
                .map((a, i) => (
                  <div className="attendance-row" key={i}>
                    <span
                      className={"attendance-code type-" + attendanceType(a)}
                      title={t(labels[attendanceType(a)])}
                    >
                      {codes[attendanceType(a)]}
                    </span>
                    <div>
                      <h3>{a.subject}</h3>
                      <p>
                        {t("lesson")} {a.lesson} · {a.kind}
                      </p>
                    </div>
                  </div>
                ))}
            </div>
          </section>
        ))}
        {!days.length && empty()}
      </>
    );
  }
  function messages() {
    const list = payload?.Messages;
    const notices = payload?.Notices;
    const query = search.toLocaleLowerCase();
    return (
      <>
        <div className="segmented glass" aria-label={t("Messages")}>
          <button
            className={inbox ? "active" : ""}
            onClick={() => {
              setInbox(true);
              setSearch("");
            }}
          >
            {t("Inbox")}
          </button>
          <button
            className={!inbox ? "active" : ""}
            onClick={() => {
              setInbox(false);
              setSearch("");
            }}
          >
            {t("Notices")}
          </button>
        </div>
        <label className="search-box">
          <Search size={19} />
          <span className="sr-only">
            {t(inbox ? "searchInbox" : "searchNotices")}
          </span>
          <input
            type="search"
            placeholder={t(inbox ? "searchInbox" : "searchNotices")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          {search && (
            <button aria-label={t("clearAll")} onClick={() => setSearch("")}>
              <X size={18} />
            </button>
          )}
        </label>
        {inbox ? (
          <>
            <p className="muted read-hint">{t("readHint")}</p>
            <div className={"mail-layout " + (reading ? "reading" : "")}>
              <div className="mail-list">
                {list?.items
                  .filter((m) =>
                    `${m.title} ${m.sender} ${m.content}`
                      .toLocaleLowerCase()
                      .includes(query),
                  )
                  .map((m) => (
                    <button
                      key={m.id}
                      className={
                        "mail-card card " +
                        (m.unread ? "unread " : "") +
                        (reading?.id === m.id ? "selected" : "")
                      }
                      aria-label={`${m.title} · ${t(m.unread ? "unread" : "read")}`}
                      onClick={() => void openMessage(m)}
                    >
                      <div className="mail-top">
                        <span>{m.sender}</span>
                        <time>{date(m.date)}</time>
                      </div>
                      <h3>
                        {m.unread && <i className="dot" />}
                        {m.title}
                      </h3>
                      <p>{m.content || t(m.unread ? "unread" : "read")}</p>
                      <div className="mail-meta">
                        {m.has_files && <Paperclip size={16} />}
                        <span>
                          {t(
                            reading?.id === m.id
                              ? "selected"
                              : m.unread
                                ? "unread"
                                : "read",
                          )}
                        </span>
                        <ChevronRight size={16} />
                      </div>
                    </button>
                  ))}
                {(!list ||
                  !list.items.filter((m) =>
                    `${m.title} ${m.sender} ${m.content}`
                      .toLocaleLowerCase()
                      .includes(query),
                  ).length) &&
                  empty()}
                <div className="pagination glass">
                  <button
                    disabled={page === 1}
                    aria-label={t("previousPage")}
                    onClick={() => setPage(page - 1)}
                  >
                    <ChevronLeft />
                  </button>
                  <span>
                    {t("page")} {page} /{" "}
                    {Math.max(1, Math.ceil((list?.total ?? 0) / 20))}
                  </span>
                  <button
                    disabled={!list || page * 20 >= list.total || demo}
                    aria-label={t("nextPage")}
                    onClick={() => setPage(page + 1)}
                  >
                    <ChevronRight />
                  </button>
                </div>
              </div>
              {reader()}
            </div>
          </>
        ) : (
          <div className="notices">
            {notices
              ?.filter((n) =>
                `${n.title} ${n.author} ${n.content}`
                  .toLocaleLowerCase()
                  .includes(query),
              )
              .map((n) => (
                <details className="notice card" key={n.id}>
                  <summary>
                    <span className="eyebrow">
                      {n.author} · {date(n.date)}
                    </span>
                    <h3>{n.title}</h3>
                    <span className="text-link">
                      {t("details")} <ChevronRight size={15} />
                    </span>
                  </summary>
                  <div className="message-body">{n.content}</div>
                  {n.until && (
                    <p className="muted">
                      {t("until")}: {date(n.until, true)}
                    </p>
                  )}
                </details>
              ))}
            {(!notices ||
              !notices.filter((n) =>
                `${n.title} ${n.author} ${n.content}`
                  .toLocaleLowerCase()
                  .includes(query),
              ).length) &&
              empty()}
          </div>
        )}
      </>
    );
  }
  async function switchAccount(owner: string) {
    gate.current.reset();
    const valid = gate.current.begin("auth");
    setBusy(true);
    setError(null);
    setAccount(null);
    setArchive(emptyArchive());
    setReading(null);
    setSheet(null);
    try {
      const result = await bridge.switchAccount({ owner });
      if (valid()) {
        setAccount(result.account);
        setArchive(result.archive);
        setSavedAccounts(result.accounts ?? []);
        setTab("Grades");
      }
    } catch (e) {
      if (valid()) setError(errorCode(e));
    } finally {
      if (valid()) setBusy(false);
    }
  }
  function profile() {
    if (about)
      return (
        <section className="card about">
          <button onClick={() => setAbout(false)}>{t("back")}</button>
          <h2>Klaso Mobile</h2>
          <p>{t("version")} 0.1.2</p>
          <p>{t("noTelemetry")}</p>
          <p>React · Capacitor · Rust</p>
          <a
            href="https://github.com/chillyfrog123/klaso-mobile"
            target="_blank"
            rel="noopener noreferrer"
          >
            {t("repository")}
          </a>
          <button className="wide primary" onClick={() => void openGithub()}>
            {t("github")}
          </button>
        </section>
      );

    return (
      <>
        <div className="profile-banner">
          <div className="avatar">
            <UserRound size={30} />
          </div>
          <h2>
            {demo ? t("offlineDemo") : (account?.owner ?? "Klaso Mobile")}
          </h2>
          <p>
            {demo
              ? t("demoHint")
              : t(
                  account?.session
                    ? account.authenticated
                      ? "connected"
                      : "savedSession"
                    : account
                      ? "cachedAccount"
                      : "signedOut",
                )}
          </p>
          {account && (
            <span className="status-pill">
              <ShieldCheck size={14} />
              {t(account.remembered ? "remembered" : "sessionOnly")}
            </span>
          )}
        </div>
        {!demo && (
          <section className="card accounts">
            <h2>{t("accounts")}</h2>
            {savedAccounts.map((owner) => (
              <div className="account-row" key={owner}>
                <button
                  disabled={busy || owner === account?.owner}
                  onClick={() => void switchAccount(owner)}
                >
                  {owner}
                  {owner === account?.owner ? " ✓" : ""}
                </button>
                <button
                  aria-label={t("forgetAccount") + " " + owner}
                  disabled={busy}
                  onClick={async () => {
                    try {
                      await bridge.forgetAccount({ owner });
                      if (account?.owner === owner)
                        setAccount(
                          (prev) => prev && { ...prev, remembered: false },
                        );
                      setSavedAccounts((prev) =>
                        prev.filter((x) => x !== owner),
                      );
                    } catch (e) {
                      setError(errorCode(e));
                    }
                  }}
                >
                  {t("forgetAccount")}
                </button>
              </div>
            ))}
            <button
              className="wide secondary"
              onClick={() => setAddAccount(!addAccount)}
            >
              {t("addAccount")}
            </button>
          </section>
        )}
        {!demo && (!account?.session || addAccount) && (
          <form className="card login-form" onSubmit={login}>
            <h2>{t("signIn")}</h2>
            <p className="muted">{t("loginHint")}</p>
            <label>
              {t("login")}
              <input
                name="login"
                autoComplete="username"
                autoCapitalize="none"
                spellCheck={false}
                required
                defaultValue={account?.owner}
              />
            </label>
            <label>
              {t("password")}
              <input
                name="password"
                type="password"
                autoComplete="current-password"
                required
              />
            </label>
            <label className="checkbox">
              <input name="remember" type="checkbox" />
              {t("remember")}
            </label>
            <button className="primary" disabled={busy} type="submit">
              {busy ? t("loading") : t("signIn")}
              <ArrowUpRight size={18} />
            </button>
            <p className="secure-hint">
              <ShieldCheck size={17} />
              {t("secureHint")}
            </p>
          </form>
        )}
        <section className="card settings">
          <h2>{t("appearance")}</h2>
          <label className="setting-row">
            <span>{t("language")}</span>
            <Choice
              label={t("language")}
              value={lang}
              onChange={setLang}
              close={t("close")}
              options={[
                { value: "en", label: "English" },
                { value: "pl", label: "Polski" },
              ]}
            />
          </label>
          <div className="setting-row">
            <span>{t("theme")}</span>
            <Choice
              label={t("theme")}
              value={mode}
              onChange={setMode}
              close={t("close")}
              options={[
                { value: "system", label: t("system") },
                { value: "light", label: t("light") },
                { value: "dark", label: t("dark") },
              ]}
            />
          </div>
          <p>{t("accent")}</p>
          <div className="accents">
            {accents.map((a) => (
              <button
                key={a}
                aria-pressed={accent === a}
                onClick={() => setAccent(a)}
              >
                <span className={"swatch " + a}>
                  {accent === a && <Check size={18} />}
                </span>
                {t(a)}
              </button>
            ))}
          </div>
        </section>
        <section className="card privacy">
          <ShieldCheck size={24} />
          <div>
            <h3>{t("privacy")}</h3>
            <p>{t("privacyHint")}</p>
          </div>
        </section>
        <button className="wide secondary" onClick={() => void official()}>
          {t("official")}
          <ArrowUpRight size={18} />
        </button>
        <button className="wide secondary" onClick={() => setAbout(true)}>
          {t("about")}
        </button>
        {demo ? (
          <button className="wide secondary" onClick={exitDemo}>
            <LogOut size={18} />
            {t("exitDemo")}
          </button>
        ) : (
          <>
            <button
              className="wide secondary"
              disabled={busy}
              onClick={() => void startDemo()}
            >
              <Sparkles size={18} />
              {t("demo")}
            </button>
            {account && (
              <button className="wide danger" onClick={() => void logout()}>
                <LogOut size={18} />
                {t("signOut")}
              </button>
            )}
          </>
        )}
      </>
    );
  }
  return (
    <div className="app">
      <header className="topbar glass">
        <a
          className="brand"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            setTab("Timetable");
          }}
        >
          <span className="brand-mark">
            <BookOpen size={21} />
          </span>
          Klaso<span>mobile</span>
        </a>
        <button
          className="profile-button"
          aria-label={t("Profile")}
          onClick={() => setTab("Profile")}
        >
          <UserRound size={20} />
        </button>
      </header>
      <main>
        {demo && (
          <div className="demo-banner">
            <WifiOff size={15} />
            <span>{t("offlineDemo")}</span>
            <button onClick={exitDemo} aria-label={t("exitDemo")}>
              <X size={16} />
            </button>
          </div>
        )}
        <div className="page-heading">
          <div>
            <span className="eyebrow">{t("greeting")}</span>
            <h1>{t(tab)}</h1>
          </div>
          {tab !== "Profile" && (account || demo) && (
            <button
              className={"refresh glass " + (busy ? "spinning" : "")}
              disabled={busy}
              aria-label={t("refresh")}
              onClick={() => void refresh()}
            >
              <RefreshCw size={20} />
            </button>
          )}
        </div>
        {tab !== "Profile" && (account || demo) && (
          <p className="freshness" role="status">
            <span className={"dot " + (busy ? "pending" : "")} />
            {busy
              ? t("refreshing")
              : snapshot
                ? `${t("updated")} · ${new Intl.DateTimeFormat(lang === "pl" ? "pl-PL" : "en-GB", { dateStyle: "short", timeStyle: "short" }).format(new Date(snapshot.updated))}`
                : t("notUpdated")}
          </p>
        )}
        {status(error)}
        {!ready ? (
          <div className="empty" role="status">
            {t("loading")}
          </div>
        ) : tab === "Profile" ? (
          profile()
        ) : tab === "Grades" ? (
          grades()
        ) : tab === "Timetable" ? (
          timetable()
        ) : tab === "Attendance" ? (
          attendance()
        ) : (
          messages()
        )}
      </main>
      <nav className="bottom-nav glass" aria-label="Klaso Mobile">
        {tabs.map((item) => {
          const Icon = icons[item];
          return (
            <button
              key={item}
              aria-current={tab === item ? "page" : undefined}
              className={tab === item ? "active" : ""}
              onClick={() => {
                setTab(item);
                setReading(null);
                setSheet(null);
              }}
            >
              <span>
                <Icon size={21} />
              </span>
              <small>{t(item)}</small>
            </button>
          );
        })}
      </nav>
      <dialog
        ref={dialog}
        className="detail-sheet"
        onCancel={() => setSheet(null)}
        onClick={(e) => {
          if (e.target === dialog.current) setSheet(null);
        }}
      >
        <div className="sheet-handle" />
        <button
          className="close-sheet"
          aria-label={t("close")}
          onClick={() => setSheet(null)}
        >
          <X />
        </button>
        {sheet?.grade && (
          <>
            <span className="eyebrow">{t("gradeDetails")}</span>
            <div className="sheet-title">
              <span className={"grade " + band(sheet.grade.value)}>
                {sheet.grade.value}
              </span>
              <h2>{sheet.grade.subject}</h2>
            </div>
            <dl>
              {(
                [
                  "category",
                  "date",
                  "teacher",
                  "weight",
                  "semester",
                  "counts",
                ] as const
              ).map((field) => (
                <div key={field}>
                  <dt>{t(field)}</dt>
                  <dd>
                    {field === "counts"
                      ? t(sheet.grade!.counts ? "yes" : "no")
                      : field === "date"
                        ? date(sheet.grade!.date, true)
                        : (sheet.grade![field] ?? t("unknown"))}
                  </dd>
                </div>
              ))}
            </dl>
          </>
        )}
        {sheet?.lesson && (
          <>
            <span className="eyebrow">{t("lessonDetails")}</span>
            <h2>{sheet.lesson.subject}</h2>
            <dl>
              {(["date", "teacher", "room", "time", "note"] as const).map(
                (field) => (
                  <div key={field}>
                    <dt>{t(field)}</dt>
                    <dd>
                      {field === "time"
                        ? `${sheet.lesson!.from} – ${sheet.lesson!.to}`
                        : field === "date"
                          ? date(sheet.lesson!.date, true)
                          : sheet.lesson![field] || "—"}
                    </dd>
                  </div>
                ),
              )}
            </dl>
            {(sheet.lesson.canceled || sheet.lesson.substitution) && (
              <p
                className={
                  sheet.lesson.canceled
                    ? "canceled status-pill"
                    : "substitution status-pill"
                }
              >
                {t(sheet.lesson.canceled ? "canceled" : "substitution")}
              </p>
            )}
          </>
        )}
      </dialog>
    </div>
  );
}
createRoot(document.getElementById("root")!).render(<App />);
