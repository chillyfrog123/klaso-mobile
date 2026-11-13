import type { CapacitorConfig } from "@capacitor/cli";
const config: CapacitorConfig = {
  appId: "pl.libruslite.mobile",
  appName: "Klaso Mobile",
  webDir: "dist",
  loggingBehavior: "none",
  android: { allowMixedContent: false },
  server: { androidScheme: "https" },
  plugins: {
    SystemBars: {
      style: "LIGHT",
      insetsHandling: "css",
      initialViewportFitValueHint: "cover",
    },
  },
};
export default config;
