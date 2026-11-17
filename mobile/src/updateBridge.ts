import { Capacitor, registerPlugin } from "@capacitor/core";

export interface AppRelease {
  version: string;
  notes: string;
}

export interface UpdateSource {
  check(): Promise<{ currentVersion: string; release: AppRelease | null }>;
  install(options: { version: string }): Promise<void>;
}

export const updatesSupported = Capacitor.getPlatform() === "android";
export const updateSource = registerPlugin<UpdateSource>("AppUpdater");
