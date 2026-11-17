import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { App as NativeApp } from "@capacitor/app";
import type { Label } from "./i18n";
import { updateSource, updatesSupported } from "./updateBridge";
import { UpdateController } from "./updater";

export function useUpdates() {
  const [controller] = useState(() => {
    let storage: Storage | undefined;
    try {
      storage = localStorage;
    } catch {
      /* unavailable preferences */
    }
    return new UpdateController(updateSource, updatesSupported, storage);
  });
  const state = useSyncExternalStore(
    controller.subscribe,
    controller.getSnapshot,
  );
  useEffect(() => {
    if (!controller.supported) return;
    void controller.resume();
    const listener = NativeApp.addListener("appStateChange", ({ isActive }) => {
      if (isActive) void controller.resume();
      else controller.suspend();
    });
    return () => {
      controller.suspend();
      void listener.then((handle) => handle.remove());
    };
  }, [controller]);
  return { controller, state };
}

type Updates = ReturnType<typeof useUpdates>;
type Translate = (key: Label) => string;

export function UpdateSettings({
  updates: { controller, state },
  t,
}: {
  updates: Updates;
  t: Translate;
}) {
  const busy = state.status === "checking" || state.status === "downloading";
  return (
    <section className="card settings app-updates" aria-label={t("appUpdates")}>
      <h2>{t("appUpdates")}</h2>
      <p className="muted">
        {t("version")} {state.currentVersion}
      </p>
      {!controller.supported ? (
        <p>{t("updatesAndroidOnly")}</p>
      ) : (
        <>
          <label className="setting-row update-setting">
            <span>
              {t("autoCheckUpdates")}
              <small>{t("autoCheckHint")}</small>
            </span>
            <input
              type="checkbox"
              role="switch"
              checked={state.preferences.autoCheck}
              onChange={(event) =>
                controller.setPreference("autoCheck", event.target.checked)
              }
            />
          </label>
          <label className="setting-row update-setting">
            <span>
              {t("autoUpdate")}
              <small>{t("autoUpdateHint")}</small>
            </span>
            <input
              type="checkbox"
              role="switch"
              checked={state.preferences.autoUpdate}
              onChange={(event) =>
                controller.setPreference("autoUpdate", event.target.checked)
              }
            />
          </label>
          <p className="muted">{t("updateNetworkHint")}</p>
          <button
            className="wide secondary"
            disabled={busy}
            onClick={() => void controller.check(true)}
          >
            {t(
              state.status === "checking" ? "checkingUpdates" : "checkUpdates",
            )}
          </button>
          <UpdateStatus state={state} t={t} />
          {state.release && (
            <>
              <p>
                {t("newVersion")} {state.release.version}
              </p>
              {state.release.notes && (
                <details>
                  <summary>{t("releaseNotes")}</summary>
                  <p className="release-notes">{state.release.notes}</p>
                </details>
              )}
              <button
                className="wide primary"
                disabled={busy}
                onClick={() => void controller.install()}
              >
                {t(
                  state.status === "downloading"
                    ? "downloadingUpdate"
                    : "installUpdate",
                )}
              </button>
            </>
          )}
        </>
      )}
    </section>
  );
}

function UpdateStatus({ state, t }: { state: Updates["state"]; t: Translate }) {
  if (state.error) return <p role="alert">{t(state.error)}</p>;
  const key =
    state.status === "current"
      ? "appUpToDate"
      : state.status === "downloading"
        ? "downloadingUpdate"
        : state.status === "installerOpened"
          ? "updateInstallerOpened"
          : null;
  return key ? <p role="status">{t(key)}</p> : null;
}

export function UpdateNotice({
  updates: { state },
  t,
  onManage,
}: {
  updates: Updates;
  t: Translate;
  onManage: () => void;
}) {
  const installErrors: Label[] = [
    "updatePermissionRequired",
    "updateVerificationFailed",
    "updateChanged",
    "updateDownloadFailed",
  ];
  if (
    state.status !== "downloading" &&
    state.status !== "installerOpened" &&
    !(state.error && installErrors.includes(state.error))
  )
    return null;
  return (
    <section className="card update-notice" aria-label={t("appUpdates")}>
      <UpdateStatus state={state} t={t} />
      <button className="secondary" onClick={onManage}>
        {t("appUpdates")}
      </button>
    </section>
  );
}

export function UpdatePrompt({
  updates: { controller, state },
  t,
}: {
  updates: Updates;
  t: Translate;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    if (state.prompt && state.release && !dialog.current?.open)
      dialog.current?.showModal();
    else if (!state.prompt && dialog.current?.open) dialog.current.close();
  }, [state.prompt, state.release]);
  return (
    <dialog
      ref={dialog}
      className="choice update-prompt"
      aria-labelledby="update-title"
      onCancel={() => controller.dismiss()}
      onClose={() => {
        if (state.prompt) controller.dismiss();
      }}
    >
      <h2 id="update-title">{t("updateAvailable")}</h2>
      <p>
        {t("newVersion")} {state.release?.version}
      </p>
      <p>{t("updatePromptHint")}</p>
      {state.release?.notes && (
        <details>
          <summary>{t("releaseNotes")}</summary>
          <p className="release-notes">{state.release.notes}</p>
        </details>
      )}
      <div className="update-actions">
        <button className="secondary" onClick={() => controller.dismiss()}>
          {t("updateLater")}
        </button>
        <button className="primary" onClick={() => void controller.install()}>
          {t("installUpdate")}
        </button>
      </div>
    </dialog>
  );
}
