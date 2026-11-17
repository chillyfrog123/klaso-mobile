package pl.libruslite.mobile;

import android.content.Intent;
import android.content.pm.PackageInfo;
import android.content.pm.PackageManager;
import android.content.pm.Signature;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;
import androidx.activity.result.ActivityResult;
import androidx.core.content.FileProvider;
import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.ActivityCallback;
import com.getcapacitor.annotation.CapacitorPlugin;
import org.json.JSONArray;
import org.json.JSONObject;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.Arrays;
import java.util.HashSet;
import java.util.Locale;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;
import javax.net.ssl.HttpsURLConnection;

/** Public release checks do not use Librus sessions, account storage or GitHub tokens. */
@CapacitorPlugin(name = "AppUpdater")
public class AppUpdaterPlugin extends Plugin {
    private static final String LATEST = "https://api.github.com/repos/" + UpdatePolicy.REPOSITORY + "/releases/latest";
    private static final long MAX_APK_BYTES = 150L * 1024 * 1024;
    private static final ExecutorService worker = Executors.newSingleThreadExecutor();
    private final AtomicBoolean installing = new AtomicBoolean();
    private Release prepared;
    private boolean foreground;
    private PluginCall waitingForForeground;

    @Override protected void handleOnResume() {
        foreground = true;
        if (waitingForForeground != null) {
            PluginCall call = waitingForForeground;
            waitingForForeground = null;
            requestInstall(call);
        }
    }

    @Override protected void handleOnPause() { foreground = false; }

    private static class UpdateError extends Exception {
        UpdateError(String code) { super(code); }
    }

    private static class Release {
        final String version, url, digest, notes;
        final long size;
        Release(String version, String url, String digest, String notes, long size) {
            this.version = version; this.url = url; this.digest = digest;
            this.notes = notes; this.size = size;
        }
        JSObject json() { return new JSObject().put("version", version).put("notes", notes); }
    }

    @SuppressWarnings("deprecation")
    private PackageInfo installed() throws Exception {
        return getContext().getPackageManager().getPackageInfo(getContext().getPackageName(), signatureFlags());
    }

    @SuppressWarnings("deprecation")
    private static int signatureFlags() {
        return Build.VERSION.SDK_INT >= Build.VERSION_CODES.P ? PackageManager.GET_SIGNING_CERTIFICATES : PackageManager.GET_SIGNATURES;
    }

    @SuppressWarnings("deprecation")
    private static Signature[] signers(PackageInfo info) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            return info.signingInfo == null ? null : info.signingInfo.getApkContentsSigners();
        }
        return info.signatures;
    }

    @PluginMethod
    public void check(PluginCall call) {
        worker.execute(() -> {
            try {
                String current = installed().versionName;
                Release release = latest(current);
                call.resolve(new JSObject().put("currentVersion", current)
                    .put("release", release == null ? JSONObject.NULL : release.json()));
            } catch (UpdateError error) { call.reject(error.getMessage()); }
            catch (Exception ignored) { call.reject("updateCheckFailed"); }
        });
    }

    private Release latest(String current) throws Exception {
        JSONObject release = new JSONObject(readJson());
        if (release.optBoolean("draft") || release.optBoolean("prerelease")) throw new UpdateError("updatesUnavailable");
        String tag = release.optString("tag_name");
        String version;
        try { version = UpdatePolicy.version(tag); }
        catch (IllegalArgumentException ignored) { throw new UpdateError("updatesUnavailable"); }
        if (!UpdatePolicy.newer(version, current)) return null;
        JSONArray assets = release.getJSONArray("assets");
        for (int index = 0; index < assets.length(); index++) {
            JSONObject asset = assets.getJSONObject(index);
            String name = asset.optString("name"), url = asset.optString("browser_download_url");
            String digest = asset.optString("digest");
            long size = asset.optLong("size");
            if (UpdatePolicy.assetName(name, version) && UpdatePolicy.releaseUrl(url, tag, name) &&
                UpdatePolicy.digest(digest) && size > 0 && size <= MAX_APK_BYTES && "uploaded".equals(asset.optString("state"))) {
                return new Release(version, url, digest.substring(7).toLowerCase(Locale.ROOT), release.optString("body"), size);
            }
        }
        throw new UpdateError("updatesUnavailable");
    }

    private HttpsURLConnection connect(String address, boolean json) throws Exception {
        URL url = new URL(address);
        for (int redirects = 0; redirects <= 5; redirects++) {
            if (!UpdatePolicy.transportUrl(url.toString())) throw new UpdateError("updateVerificationFailed");
            HttpsURLConnection connection = (HttpsURLConnection) url.openConnection();
            connection.setInstanceFollowRedirects(false);
            connection.setConnectTimeout(15_000);
            connection.setReadTimeout(30_000);
            connection.setRequestProperty("User-Agent", "Klaso-Mobile-Updater");
            connection.setRequestProperty("Accept", json ? "application/vnd.github+json" : "application/octet-stream");
            if (json) connection.setRequestProperty("X-GitHub-Api-Version", "2022-11-28");
            int status;
            try { status = connection.getResponseCode(); }
            catch (Exception error) { connection.disconnect(); throw error; }
            if (status == 301 || status == 302 || status == 303 || status == 307 || status == 308) {
                String location = connection.getHeaderField("Location");
                connection.disconnect();
                if (location == null) throw new UpdateError("updateDownloadFailed");
                url = new URL(url, location);
                continue;
            }
            if (status != 200) {
                connection.disconnect();
                throw new UpdateError(status == 404 ? "updatesUnavailable" : "updateCheckFailed");
            }
            return connection;
        }
        throw new UpdateError("updateDownloadFailed");
    }

    private String readJson() throws Exception {
        HttpsURLConnection connection = connect(LATEST, true);
        try (InputStream input = connection.getInputStream(); ByteArrayOutputStream output = new ByteArrayOutputStream()) {
            byte[] buffer = new byte[8192];
            int count;
            while ((count = input.read(buffer)) != -1) {
                if (output.size() + count > 1024 * 1024) throw new UpdateError("updatesUnavailable");
                output.write(buffer, 0, count);
            }
            return output.toString(StandardCharsets.UTF_8.name());
        } finally { connection.disconnect(); }
    }

    @PluginMethod
    public void install(PluginCall call) {
        if (!installing.compareAndSet(false, true)) { call.reject("updateDownloadFailed"); return; }
        worker.execute(() -> {
            try {
                // Re-fetch trusted metadata. JavaScript never supplies a download URL or checksum.
                Release release = latest(installed().versionName);
                if (release == null || !release.version.equals(call.getString("version"))) throw new UpdateError("updateChanged");
                download(release);
                prepared = release;
                getActivity().runOnUiThread(() -> requestInstall(call));
            } catch (UpdateError error) { installing.set(false); call.reject(error.getMessage()); }
            catch (Exception ignored) { installing.set(false); call.reject("updateDownloadFailed"); }
        });
    }

    private File apkFile() throws Exception {
        File directory = new File(getContext().getCacheDir(), "app-updates");
        if (!directory.isDirectory() && !directory.mkdirs()) throw new UpdateError("updateDownloadFailed");
        return new File(directory, "update.apk");
    }

    private void download(Release release) throws Exception {
        File apk = apkFile();
        File partial = new File(apk.getParentFile(), "update.part");
        // Reuse only a complete APK that still matches the freshly fetched release.
        if (apk.isFile()) {
            try { verify(apk, release); return; }
            catch (Exception ignored) { if (!apk.delete()) throw new UpdateError("updateDownloadFailed"); }
        }
        HttpsURLConnection connection = connect(release.url, false);
        try {
            try (InputStream input = connection.getInputStream(); FileOutputStream output = new FileOutputStream(partial)) {
                byte[] buffer = new byte[64 * 1024];
                long total = 0;
                int count;
                while ((count = input.read(buffer)) != -1) {
                    total += count;
                    if (total > release.size || total > MAX_APK_BYTES) throw new UpdateError("updateVerificationFailed");
                    output.write(buffer, 0, count);
                }
                output.getFD().sync();
            }
            verify(partial, release);
            if (!partial.renameTo(apk)) throw new UpdateError("updateDownloadFailed");
        } finally {
            connection.disconnect();
            if (partial.exists() && !partial.delete()) partial.deleteOnExit();
        }
    }

    @SuppressWarnings("deprecation")
    private void verify(File apk, Release release) throws Exception {
        if (apk.length() != release.size) throw new UpdateError("updateVerificationFailed");
        MessageDigest digest = MessageDigest.getInstance("SHA-256");
        try (InputStream input = new FileInputStream(apk)) {
            byte[] buffer = new byte[64 * 1024];
            int count;
            while ((count = input.read(buffer)) != -1) digest.update(buffer, 0, count);
        }
        StringBuilder hex = new StringBuilder();
        for (byte value : digest.digest()) hex.append(String.format(Locale.ROOT, "%02x", value & 0xff));
        if (!release.digest.contentEquals(hex)) throw new UpdateError("updateVerificationFailed");
        PackageManager manager = getContext().getPackageManager();
        PackageInfo next = manager.getPackageArchiveInfo(apk.getAbsolutePath(), signatureFlags());
        PackageInfo current = installed();
        if (next == null || next.applicationInfo == null || !current.packageName.equals(next.packageName) ||
            !release.version.equals(next.versionName) || next.versionCode <= current.versionCode ||
            next.applicationInfo.minSdkVersion > Build.VERSION.SDK_INT ||
            !UpdatePolicy.newer(next.versionName, current.versionName) ||
            !sameSignatures(signers(current), signers(next))) throw new UpdateError("updateVerificationFailed");
    }

    private static boolean sameSignatures(Signature[] current, Signature[] next) {
        return current != null && next != null && current.length > 0 &&
            new HashSet<>(Arrays.asList(current)).equals(new HashSet<>(Arrays.asList(next)));
    }

    private void requestInstall(PluginCall call) {
        if (!foreground) { waitingForForeground = call; return; }
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !getContext().getPackageManager().canRequestPackageInstalls()) {
                Intent permission = new Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                    Uri.parse("package:" + getContext().getPackageName()));
                startActivityForResult(call, permission, "installPermissionResult");
            } else { openInstaller(call); }
        } catch (Exception ignored) { installing.set(false); call.reject("updatePermissionRequired"); }
    }

    @ActivityCallback
    private void installPermissionResult(PluginCall call, ActivityResult result) {
        if (call == null) { installing.set(false); return; }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !getContext().getPackageManager().canRequestPackageInstalls()) {
            installing.set(false); call.reject("updatePermissionRequired"); return;
        }
        openInstaller(call);
    }

    private void openInstaller(PluginCall call) {
        worker.execute(() -> {
            try {
                // Check again after returning from settings. A process restart requires a retry.
                if (prepared == null) throw new UpdateError("updateVerificationFailed");
                File apk = apkFile();
                verify(apk, prepared);
                Uri uri = FileProvider.getUriForFile(getContext(), getContext().getPackageName() + ".fileprovider", apk);
                Intent intent = new Intent(Intent.ACTION_VIEW).setDataAndType(uri, "application/vnd.android.package-archive")
                    .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                getActivity().runOnUiThread(() -> {
                    if (!foreground) { waitingForForeground = call; return; }
                    try { getActivity().startActivity(intent); call.resolve(); }
                    catch (Exception ignored) { call.reject("updateDownloadFailed"); }
                    finally { installing.set(false); }
                });
            } catch (Exception ignored) { installing.set(false); call.reject("updateVerificationFailed"); }
        });
    }
}
