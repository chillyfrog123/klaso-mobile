package pl.libruslite.mobile;

import java.net.URI;
import java.util.regex.Pattern;

/** Validation shared by release discovery and installation. Stable releases only. */
final class UpdatePolicy {
    static final String REPOSITORY = "chillyfrog123/klaso-mobile";
    private static final Pattern VERSION = Pattern.compile("v?(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)(?:\\.(0|[1-9][0-9]*))?");
    private UpdatePolicy() {}

    static String version(String value) {
        if (value == null || !VERSION.matcher(value).matches()) throw new IllegalArgumentException("version");
        String normalized = value.startsWith("v") ? value.substring(1) : value;
        for (String part : normalized.split("\\.")) Long.parseLong(part);
        return normalized;
    }

    static boolean newer(String candidate, String current) {
        String[] next = version(candidate).split("\\.");
        String[] installed = version(current).split("\\.");
        for (int index = 0; index < Math.max(next.length, installed.length); index++) {
            long nextPart = index < next.length ? Long.parseLong(next[index]) : 0;
            long installedPart = index < installed.length ? Long.parseLong(installed[index]) : 0;
            int comparison = Long.compare(nextPart, installedPart);
            if (comparison != 0) return comparison > 0;
        }
        return false;
    }

    static boolean assetName(String name, String version) {
        String prefix = "klaso-mobile-" + version(version) + "-arm64";
        return name.equals(prefix + ".apk") || name.equals(prefix + "-debug.apk") || name.equals(prefix + "-release.apk");
    }

    static boolean digest(String value) {
        return value != null && value.matches("sha256:[0-9a-fA-F]{64}");
    }

    static boolean releaseUrl(String value, String tag, String name) {
        try {
            URI uri = new URI(value);
            return safeHttps(uri) && "github.com".equals(uri.getHost()) &&
                uri.getPath().equals("/" + REPOSITORY + "/releases/download/" + tag + "/" + name) &&
                uri.getQuery() == null && uri.getFragment() == null;
        } catch (Exception ignored) { return false; }
    }

    static boolean transportUrl(String value) {
        try {
            URI uri = new URI(value);
            if (!safeHttps(uri)) return false;
            String host = uri.getHost();
            return ("api.github.com".equals(host) && uri.getPath().equals("/repos/" + REPOSITORY + "/releases/latest")) ||
                ("github.com".equals(host) && uri.getPath().startsWith("/" + REPOSITORY + "/releases/download/")) ||
                "release-assets.githubusercontent.com".equals(host) || "objects.githubusercontent.com".equals(host);
        } catch (Exception ignored) { return false; }
    }

    private static boolean safeHttps(URI uri) {
        return "https".equals(uri.getScheme()) && uri.getUserInfo() == null &&
            (uri.getPort() == -1 || uri.getPort() == 443);
    }
}
