import type { Label } from "./i18n";
import type { AppRelease, UpdateSource } from "./updateBridge";
import { version } from "../package.json";

export const appVersion = version;
export const UPDATE_CHECK_INTERVAL = 6 * 60 * 60 * 1000;
const PREFERENCE_KEY = "app-updates-v1";
type Preferences = { autoCheck: boolean; autoUpdate: boolean };
type StorageAccess = Pick<Storage, "getItem" | "setItem">;
type Status =
  | "idle"
  | "checking"
  | "available"
  | "current"
  | "downloading"
  | "installerOpened";
export interface UpdateState {
  preferences: Preferences;
  currentVersion: string;
  release: AppRelease | null;
  status: Status;
  error: Label | null;
  prompt: boolean;
  lastChecked: string | null;
}

function readPreferences(storage?: StorageAccess): Preferences {
  try {
    const saved = JSON.parse(storage?.getItem(PREFERENCE_KEY) ?? "null");
    const autoCheck = saved?.autoCheck !== false;
    return { autoCheck, autoUpdate: autoCheck && saved?.autoUpdate === true };
  } catch {
    return { autoCheck: true, autoUpdate: false };
  }
}

// Separate from school-account requests: update failures never change school data.
export class UpdateController {
  private state: UpdateState;
  private listeners = new Set<() => void>();
  private lastAttempt: number | null = null;
  private attempted = new Set<string>();
  private dismissed = new Set<string>();
  private active = true;

  constructor(
    private source: UpdateSource,
    readonly supported: boolean,
    private storage?: StorageAccess,
  ) {
    this.state = {
      preferences: readPreferences(storage),
      currentVersion: appVersion,
      release: null,
      status: "idle",
      error: null,
      prompt: false,
      lastChecked: null,
    };
  }

  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private change(patch: Partial<UpdateState>) {
    this.state = { ...this.state, ...patch };
    this.listeners.forEach((listener) => listener());
  }
  private get busy() {
    return (
      this.state.status === "checking" || this.state.status === "downloading"
    );
  }

  setPreference(key: keyof Preferences, value: boolean) {
    const preferences = { ...this.state.preferences, [key]: value };
    if (key === "autoCheck" && !value) preferences.autoUpdate = false;
    if (key === "autoUpdate" && value) preferences.autoCheck = true;
    try {
      this.storage?.setItem(PREFERENCE_KEY, JSON.stringify(preferences));
    } catch {
      /* session preferences still work */
    }
    this.change({ preferences });
    if (value) void this.check();
  }

  async resume() {
    this.active = true;
    if (
      this.state.preferences.autoCheck &&
      (this.lastAttempt === null ||
        Date.now() - this.lastAttempt >= UPDATE_CHECK_INTERVAL)
    ) {
      await this.check();
    } else {
      await this.maybeInstall();
    }
  }
  suspend() {
    this.active = false;
  }
  dismiss() {
    if (this.state.release) this.dismissed.add(this.state.release.version);
    this.change({ prompt: false });
  }

  async check(manual = false) {
    if (!this.supported || this.busy) return;
    this.lastAttempt = Date.now();
    this.change({ status: "checking", error: null });
    try {
      const result = await this.source.check();
      const release = result.release;
      this.change({
        currentVersion: result.currentVersion,
        release,
        status: release ? "available" : "current",
        lastChecked: new Date().toISOString(),
        prompt:
          !!release &&
          (manual || !this.dismissed.has(release.version)) &&
          !this.state.preferences.autoUpdate,
      });
      await this.maybeInstall();
    } catch (error) {
      const code = (error as Error)?.message;
      this.change({
        status: this.state.release ? "available" : "idle",
        error:
          code === "updatesUnavailable"
            ? "updatesUnavailable"
            : "updateCheckFailed",
      });
    }
  }

  private async maybeInstall() {
    const { release, preferences } = this.state;
    if (
      this.active &&
      preferences.autoUpdate &&
      release &&
      !this.busy &&
      !this.attempted.has(release.version)
    ) {
      this.attempted.add(release.version);
      await this.install();
    }
  }

  async install() {
    const release = this.state.release;
    if (!this.supported || this.busy || !release) return;
    this.change({ status: "downloading", error: null, prompt: false });
    try {
      await this.source.install({ version: release.version });
      this.change({ status: "installerOpened" });
    } catch (error) {
      const code = (error as Error)?.message;
      const known: Label[] = [
        "updatePermissionRequired",
        "updateVerificationFailed",
        "updateChanged",
      ];
      this.change({
        status: "available",
        error: known.includes(code as Label)
          ? (code as Label)
          : "updateDownloadFailed",
      });
    }
  }
}
