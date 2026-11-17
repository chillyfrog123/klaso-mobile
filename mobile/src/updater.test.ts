import { describe, expect, it, vi } from "vitest";
import { UpdateController, UPDATE_CHECK_INTERVAL } from "./updater";

const release = { version: "0.1.3", notes: "A new version" };
function setup(saved?: string) {
  const storage = { getItem: vi.fn(() => saved ?? null), setItem: vi.fn() };
  const source = {
    check: vi.fn(async () => ({
      currentVersion: "0.1.2",
      release: release as typeof release | null,
    })),
    install: vi.fn(async (_options: { version: string }) => {}),
  };
  const controller = new UpdateController(source, true, storage);
  return { controller, source, storage };
}

describe("app updates", () => {
  it("checks automatically, prompts, and never downloads by default", async () => {
    const { controller, source } = setup();
    await controller.resume();
    expect(controller.getSnapshot().preferences).toEqual({
      autoCheck: true,
      autoUpdate: false,
    });
    expect(controller.getSnapshot()).toMatchObject({
      prompt: true,
      status: "available",
      release,
    });
    expect(source.install).not.toHaveBeenCalled();
    controller.dismiss();
    await controller.check();
    expect(controller.getSnapshot().prompt).toBe(false);
    await controller.check(true);
    expect(controller.getSnapshot().prompt).toBe(true);
  });

  it("opens the installer only after a manual update action", async () => {
    const { controller, source } = setup();
    await controller.check();
    await controller.install();
    expect(source.install).toHaveBeenCalledExactlyOnceWith({
      version: release.version,
    });
    expect(controller.getSnapshot()).toMatchObject({
      status: "installerOpened",
      prompt: false,
    });
  });

  it("persists opt-in, downloads automatically once per version, and allows manual retry", async () => {
    const { controller, source, storage } = setup();
    controller.setPreference("autoUpdate", true);
    await vi.waitFor(() =>
      expect(controller.getSnapshot().status).toBe("installerOpened"),
    );
    expect(JSON.parse(storage.setItem.mock.calls[0][1])).toEqual({
      autoCheck: true,
      autoUpdate: true,
    });
    await controller.check();
    await controller.resume();
    expect(source.install).toHaveBeenCalledTimes(1);
    await controller.install();
    expect(source.install).toHaveBeenCalledTimes(2);
  });

  it("supports disabling checks, keeps manual checks available, and normalizes preferences", async () => {
    const { controller, source } = setup(
      '{"autoCheck":false,"autoUpdate":true}',
    );
    await controller.resume();
    expect(controller.getSnapshot().preferences).toEqual({
      autoCheck: false,
      autoUpdate: false,
    });
    expect(source.check).not.toHaveBeenCalled();
    await controller.check(true);
    expect(source.check).toHaveBeenCalledTimes(1);
    expect(source.install).not.toHaveBeenCalled();
    controller.setPreference("autoCheck", false);
    expect(controller.getSnapshot().preferences.autoUpdate).toBe(false);
  });

  it("throttles resume checks, including failures, but not manual retries", async () => {
    vi.useFakeTimers();
    try {
      const { controller, source } = setup();
      source.check.mockRejectedValue(Error("network"));
      await controller.resume();
      await controller.resume();
      expect(source.check).toHaveBeenCalledTimes(1);
      vi.advanceTimersByTime(UPDATE_CHECK_INTERVAL);
      await controller.resume();
      expect(source.check).toHaveBeenCalledTimes(2);
      await controller.check(true);
      expect(source.check).toHaveBeenCalledTimes(3);
      expect(controller.getSnapshot()).toMatchObject({
        error: "updateCheckFailed",
        lastChecked: null,
      });
    } finally {
      vi.useRealTimers();
    }
  });

  it("preserves the available version after an offline check or rejected package", async () => {
    const { controller, source } = setup();
    await controller.check();
    const lastChecked = controller.getSnapshot().lastChecked;
    source.check.mockRejectedValue(Error("network"));
    await controller.check();
    expect(controller.getSnapshot()).toMatchObject({
      release,
      lastChecked,
      error: "updateCheckFailed",
    });
    source.install.mockRejectedValue(Error("updateVerificationFailed"));
    await controller.install();
    expect(controller.getSnapshot()).toMatchObject({
      release,
      status: "available",
      error: "updateVerificationFailed",
    });
  });

  it("does not retry denied installation automatically", async () => {
    const { controller, source } = setup(
      '{"autoCheck":true,"autoUpdate":true}',
    );
    source.install.mockRejectedValue(Error("updatePermissionRequired"));
    await controller.resume();
    await controller.resume();
    await controller.check();
    expect(source.install).toHaveBeenCalledTimes(1);
    expect(controller.getSnapshot().preferences.autoUpdate).toBe(true);
  });

  it("waits for foreground before starting an automatic download", async () => {
    const { controller, source } = setup(
      '{"autoCheck":true,"autoUpdate":true}',
    );
    let finish!: (value: {
      currentVersion: string;
      release: typeof release;
    }) => void;
    source.check.mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const check = controller.resume();
    controller.suspend();
    finish({ currentVersion: "0.1.2", release });
    await check;
    expect(source.install).not.toHaveBeenCalled();
    await controller.resume();
    expect(source.install).toHaveBeenCalledTimes(1);
  });

  it("never contacts GitHub in browser previews or on unsupported platforms", async () => {
    const { source } = setup();
    const controller = new UpdateController(source, false);
    await controller.resume();
    await controller.check(true);
    controller.setPreference("autoUpdate", true);
    await controller.install();
    expect(source.check).not.toHaveBeenCalled();
    expect(source.install).not.toHaveBeenCalled();
  });

  it("handles corrupt preferences and reports an unavailable release feed", async () => {
    const { controller, source } = setup("bad json");
    source.check.mockRejectedValue(Error("updatesUnavailable"));
    await controller.resume();
    expect(controller.getSnapshot()).toMatchObject({
      error: "updatesUnavailable",
      preferences: { autoCheck: true, autoUpdate: false },
    });
  });

  it("reports current versions and prevents duplicate concurrent operations", async () => {
    const { controller, source } = setup();
    source.check.mockResolvedValue({ currentVersion: "0.1.2", release: null });
    await Promise.all([
      controller.check(),
      controller.check(),
      controller.install(),
    ]);
    expect(source.check).toHaveBeenCalledTimes(1);
    expect(source.install).not.toHaveBeenCalled();
    expect(controller.getSnapshot()).toMatchObject({
      status: "current",
      release: null,
      prompt: false,
    });
  });
});
