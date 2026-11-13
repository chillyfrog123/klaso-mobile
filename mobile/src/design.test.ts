import { describe, it, expect } from "vitest";
import { selectDesign } from "./design";
describe("platform design", () => {
  it("uses Material You for Android and web by default", () => {
    expect(selectDesign("android", null)).toBe("material");
    expect(selectDesign("web", null)).toBe("material");
  });
  it("uses Liquid Glass on iOS", () =>
    expect(selectDesign("ios", null)).toBe("glass"));
  it("allows an iOS browser preview without overriding native Android", () => {
    expect(selectDesign("web", "ios")).toBe("glass");
    expect(selectDesign("android", "ios")).toBe("material");
  });
});
