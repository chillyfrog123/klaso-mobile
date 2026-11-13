import { Capacitor } from "@capacitor/core";

export type PlatformDesign = "material" | "glass";
export function selectDesign(
  platform: string,
  preview: string | null,
): PlatformDesign {
  // Browser previews may demonstrate either platform. A native build always
  // follows its actual OS, so a URL cannot silently restyle an Android install.
  return platform === "ios" || (platform === "web" && preview === "ios")
    ? "glass"
    : "material";
}
export function applyPlatformDesign() {
  const design = selectDesign(
    Capacitor.getPlatform(),
    new URLSearchParams(location.search).get("platform"),
  );
  document.documentElement.dataset.design = design;
  return design;
}
