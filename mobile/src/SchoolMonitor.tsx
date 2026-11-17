import { useEffect, useRef, useState } from "react";
import { App as NativeApp } from "@capacitor/app";
import {
  bridge,
  native,
  errorCode,
  type Bootstrap,
  type Monitoring,
} from "./bridge";
import { RequestGate, type Account, type Tab } from "./model";
import type { Label } from "./i18n";

type Translate = (key: Label) => string;
export function useSchoolMonitor(options: {
  ready: boolean;
  account: Account | null;
  demo: boolean;
  gate: RequestGate;
  onSync: (data: Bootstrap) => void;
  onOpen: (tab: Tab) => void;
}) {
  const callbacks = useRef(options);
  callbacks.current = options;
  const [monitoring, setMonitoring] = useState<Monitoring | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<Label | null>(null);
  const [settingsBusy, setSettingsBusy] = useState(false);
  const checkRef = useRef<() => Promise<void>>(async () => {});

  useEffect(() => {
    if (!native || !options.ready) return;
    let alive = true;
    void bridge
      .monitoring()
      .then((state) => {
        if (alive) setMonitoring(state);
      })
      .catch(() => {
        if (alive) setError("requestFailed");
      });
    return () => {
      alive = false;
    };
  }, [options.ready, options.account?.owner, options.demo]);

  useEffect(() => {
    setError(null);
    setBusy(false);
    if (!native || !options.ready) return;
    let alive = true;
    let active = !document.hidden;
    let running = false;
    let lastCheck = 0;
    const valid = options.gate.begin("school-monitor");
    const owner = options.account?.owner;
    const canSync =
      !!options.account?.session &&
      !options.demo &&
      monitoring?.enabled === true;
    const current = () => alive && valid();
    async function check(force = false) {
      if (!current() || !active || running || !canSync || !owner) return;
      if (!force && Date.now() - lastCheck < 30_000) return;
      lastCheck = Date.now();
      running = true;
      setBusy(true);
      setError(null);
      try {
        const data = await bridge.syncSchool({ owner });
        if (current() && data.account?.owner === owner) {
          callbacks.current.onSync(data);
          setError(
            data.syncErrors?.includes("verification")
              ? "verification"
              : data.syncErrors?.length
                ? "schoolSyncPartial"
                : null,
          );
          const state = await bridge.monitoring();
          if (current()) setMonitoring(state);
        }
      } catch (failure) {
        if (current()) setError(errorCode(failure));
      } finally {
        running = false;
        if (current()) setBusy(false);
      }
    }
    async function resume() {
      try {
        await bridge.setSchoolPaused({ paused: options.demo });
        if (!current() || options.demo) return;
        const state = await bridge.monitoring();
        if (!current()) return;
        setMonitoring(state);
        const target = await bridge.consumeNotification();
        if (current() && target.section && owner)
          callbacks.current.onOpen(target.section);
        await check();
      } catch (failure) {
        if (current()) setError(errorCode(failure));
      }
    }
    checkRef.current = () => check(true);
    void resume();
    const listener = NativeApp.addListener("appStateChange", ({ isActive }) => {
      active = isActive;
      if (isActive) void resume();
    });
    // Visibility also covers a WebView being hidden without a Capacitor event.
    const visibility = () => {
      active = !document.hidden;
    };
    document.addEventListener("visibilitychange", visibility);
    const timer = window.setInterval(() => {
      void check();
    }, 5 * 60_000);
    return () => {
      alive = false;
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", visibility);
      void listener.then((handle) => handle.remove());
      checkRef.current = async () => {};
    };
  }, [
    options.ready,
    options.account?.owner,
    options.account?.session,
    options.demo,
    options.gate,
    monitoring?.enabled,
  ]);

  async function configure(enabled: boolean, requestPermission = false) {
    setSettingsBusy(true);
    setError(null);
    try {
      setMonitoring(await bridge.setMonitoring({ enabled, requestPermission }));
    } catch (failure) {
      setError(errorCode(failure));
    } finally {
      setSettingsBusy(false);
    }
  }
  return {
    monitoring,
    busy,
    error,
    settingsBusy,
    configure,
    check: () => checkRef.current(),
  };
}

export function SchoolSettings({
  school,
  t,
  demo,
  account,
}: {
  school: ReturnType<typeof useSchoolMonitor>;
  t: Translate;
  demo: boolean;
  account: Account | null;
}) {
  return (
    <section className="card settings" aria-label={t("schoolNotifications")}>
      <h2>{t("schoolNotifications")}</h2>
      {!native || demo ? (
        <p className="muted">
          {t(demo ? "schoolDemoHint" : "schoolAndroidOnly")}
        </p>
      ) : (
        <>
          <label className="setting-row update-setting">
            <span>
              {t("schoolAutomatic")}
              <small>{t("schoolAutomaticHint")}</small>
            </span>
            <input
              type="checkbox"
              role="switch"
              checked={school.monitoring?.enabled ?? true}
              disabled={school.settingsBusy || !school.monitoring}
              onChange={(event) =>
                void school.configure(
                  event.target.checked,
                  event.target.checked,
                )
              }
            />
          </label>
          <p className="muted">{t("schoolCategories")}</p>
          <p className="muted">
            {t(
              account?.remembered
                ? "schoolBackgroundHint"
                : "schoolRememberHint",
            )}
          </p>
          {school.monitoring?.enabled &&
            school.monitoring.permission !== "granted" && (
              <>
                <p>{t("schoolPermissionHint")}</p>
                <button
                  className="wide secondary"
                  disabled={school.settingsBusy}
                  onClick={() => void school.configure(true, true)}
                >
                  {t("allowNotifications")}
                </button>
              </>
            )}
          {account?.session && school.monitoring?.enabled && (
            <button
              className="wide secondary"
              disabled={school.busy}
              onClick={() => void school.check()}
            >
              {t(school.busy ? "refreshing" : "schoolCheckNow")}
            </button>
          )}
          {school.error && <p role="alert">{t(school.error)}</p>}
        </>
      )}
    </section>
  );
}
