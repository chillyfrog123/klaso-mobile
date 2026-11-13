import { Capacitor, registerPlugin } from "@capacitor/core";
import type {
  Account,
  Archive,
  Payload,
  Section,
  Snapshot,
  Message,
} from "./model";
export const native = Capacitor.isNativePlatform();
export interface Bootstrap {
  accounts?: string[];
  account: Account | null;
  archive: Archive;
}
interface Bridge {
  bootstrap(): Promise<Bootstrap>;
  login(options: {
    login: string;
    password: string;
    remember: boolean;
  }): Promise<Bootstrap>;
  switchAccount(options: { owner: string }): Promise<Bootstrap>;
  forgetAccount(options: { owner: string }): Promise<void>;
  github(): Promise<void>;
  logout(): Promise<void>;
  load(options: {
    owner: string;
    section: Section;
    week: string;
    page: number;
  }): Promise<Snapshot>;
  detail(options: { owner: string; id: string }): Promise<{ message: Message }>;
  language(options: { language: string }): Promise<void>;
  official(): Promise<void>;
}
export const bridge = registerPlugin<Bridge>("Librus");
export async function official() {
  if (native) await bridge.official();
  else
    window.open("https://synergia.librus.pl/", "_blank", "noopener,noreferrer");
}
export function errorCode(error: unknown) {
  const code = (error as { message?: string })?.message;
  return code === "verification" ||
    code === "storageFailed" ||
    code === "signedOut"
    ? code
    : "requestFailed";
}
export async function demoData(
  week: string,
): Promise<Record<Section, Payload>> {
  const data = (await import("./demo.json")).default as unknown as Record<
    Section,
    Payload
  >;
  const copy = structuredClone(data);
  const base = new Date("2026-11-09T12:00:00");
  const shift = new Date(week + "T12:00:00").getTime() - base.getTime();
  for (const lesson of copy.Timetable.Timetable ?? [])
    lesson.date = new Date(
      new Date(lesson.date + "T12:00:00").getTime() + shift,
    ).toLocaleDateString("en-CA");
  return copy;
}

export async function openGithub() {
  if (native) await bridge.github();
  else
    window.open(
      "https://github.com/chillyfrog123/klaso-mobile",
      "_blank",
      "noopener,noreferrer",
    );
}
