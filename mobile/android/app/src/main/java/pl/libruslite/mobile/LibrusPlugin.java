package pl.libruslite.mobile;

import android.Manifest;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;
import com.getcapacitor.JSObject;
import com.getcapacitor.PermissionState;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;
import com.getcapacitor.annotation.Permission;
import com.getcapacitor.annotation.PermissionCallback;
import org.json.JSONArray;
import org.json.JSONObject;
import java.text.SimpleDateFormat;
import java.util.Calendar;
import java.util.Date;
import java.util.Iterator;
import java.util.Locale;
import java.util.TimeZone;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.function.BooleanSupplier;

@CapacitorPlugin(name="Librus", permissions={@Permission(alias="notifications", strings={Manifest.permission.POST_NOTIFICATIONS})})
public class LibrusPlugin extends Plugin {
    static { System.loadLibrary("librus_mobile_core"); }
    private static native String nativeRequest(String request);
    // UI, periodic workers, all account changes, Rust's process-wide session and encrypted
    // writes share one queue. Nothing can write data using another account's credentials.
    private static final ExecutorService worker = Executors.newSingleThreadExecutor();
    private static SecureStore storage;
    private static String owner = "";
    private static boolean session, remembered, authenticated, foregroundStarted, paused;
    private static JSONObject archive = emptyArchive();
    private String notificationSection = "", notificationOwner = "";
    interface Work { JSObject run() throws Exception; }
    private static final class BridgeError extends Exception { BridgeError(String code) { super(code); } }
    private static JSONObject emptyArchive() {
        try { return new JSONObject("{\"snapshots\":{},\"details\":{}}"); }
        catch (Exception impossible) { throw new AssertionError(); }
    }
    private static void ensureStorage(Context context) throws Exception { if (storage == null) storage = new SecureStore(context); }
    private void run(PluginCall call, Work action) {
        worker.execute(() -> {
            try { ensureStorage(getContext()); call.resolve(action.run()); }
            catch (BridgeError error) { call.reject(error.getMessage()); }
            catch (Exception ignored) { call.reject("storageFailed"); }
        });
    }
    private static JSONObject request(JSONObject input) throws Exception {
        JSONObject result = new JSONObject(nativeRequest(input.toString()));
        if (!result.optBoolean("ok")) throw new BridgeError(result.optString("code", "requestFailed"));
        return result.getJSONObject("value");
    }
    private static JSONObject command(String op) throws Exception { return new JSONObject().put("op", op); }
    private static String timestamp() {
        SimpleDateFormat format = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'", Locale.ROOT);
        format.setTimeZone(TimeZone.getTimeZone("UTC"));
        return format.format(new Date());
    }
    private static Calendar schoolDate() { return Calendar.getInstance(TimeZone.getTimeZone("Europe/Warsaw"), Locale.ROOT); }
    private static String date(Calendar calendar) {
        SimpleDateFormat format = new SimpleDateFormat("yyyy-MM-dd", Locale.ROOT);
        format.setTimeZone(TimeZone.getTimeZone("Europe/Warsaw"));
        return format.format(calendar.getTime());
    }
    private static JSONObject accounts() throws Exception {
        String raw = storage.read("accounts-v1"); return raw == null ? new JSONObject() : new JSONObject(raw);
    }
    private static JSObject state() throws Exception {
        JSObject out = new JSObject();
        out.put("account", owner.isEmpty() ? JSONObject.NULL : new JSONObject().put("owner", owner)
            .put("session", session).put("remembered", remembered).put("authenticated", authenticated));
        out.put("archive", archive);
        JSONArray names = accounts().names();
        out.put("accounts", names == null ? new JSONArray() : names);
        return out;
    }
    private static void reset() {
        owner = ""; session = remembered = authenticated = foregroundStarted = false; archive = emptyArchive();
    }
    private static void activate(String nextOwner, String password) throws Exception {
        String cached = storage.read("account:" + nextOwner);
        request(command("drop")); reset();
        try {
            request(command("restore").put("login", nextOwner).put("password", password));
            storage.write("active", new JSONObject().put("owner", nextOwner).toString());
            owner = nextOwner; session = remembered = true;
            archive = cached == null ? emptyArchive() : new JSONObject(cached);
        } catch (Exception error) { request(command("drop")); throw error; }
    }
    private static void foregroundLogin() throws Exception {
        if (owner.isEmpty() || foregroundStarted) return;
        JSONObject next = new JSONObject(archive.toString());
        SchoolChanges.beginLogin(next, timestamp());
        storage.write("account:" + owner, next.toString());
        archive = next; foregroundStarted = true;
    }
    private static boolean backgroundEligible(Context context) {
        return SchoolNotifications.enabled(context) && !paused && session && remembered && !owner.isEmpty();
    }
    private static void schedule(Context context) { SchoolNotifications.schedule(context, backgroundEligible(context)); }
    private static void restoreActive() throws Exception {
        if (!owner.isEmpty()) return;
        String saved = storage.read("active");
        if (saved == null) return;
        JSONObject active = new JSONObject(saved);
        String name = active.getString("owner");
        JSONObject list = accounts();
        // Migration from the original single-account encrypted marker.
        if (active.optBoolean("remembered") && active.has("password")) {
            list.put(name, active.getString("password"));
            storage.write("accounts-v1", list.toString());
            storage.write("active", new JSONObject().put("owner", name).toString());
        }
        String password = list.optString(name, "");
        if (!password.isEmpty()) activate(name, password);
        else {
            owner = name;
            String cached = storage.read("account:" + owner);
            archive = cached == null ? emptyArchive() : new JSONObject(cached);
        }
    }
    private static void requireOwner(PluginCall call) throws Exception {
        if (!session || owner.isEmpty() || !owner.equals(call.getString("owner"))) throw new BridgeError("signedOut");
    }
    @Override public void load() { captureNotification(getActivity().getIntent()); }
    @Override protected void handleOnNewIntent(Intent intent) { captureNotification(intent); }
    private synchronized void captureNotification(Intent intent) {
        if (intent == null) return;
        String section = intent.getStringExtra(SchoolNotifications.EXTRA_SECTION);
        if ("Grades".equals(section) || "Messages".equals(section) || "Timetable".equals(section)) {
            notificationSection = section;
            notificationOwner = intent.getStringExtra(SchoolNotifications.EXTRA_OWNER);
            intent.removeExtra(SchoolNotifications.EXTRA_SECTION);
            intent.removeExtra(SchoolNotifications.EXTRA_OWNER);
        }
    }
    @PluginMethod public void consumeNotification(PluginCall call) { run(call, () -> {
        JSObject result = new JSObject();
        synchronized (this) {
            if (owner.equals(notificationOwner)) result.put("section", notificationSection);
            notificationSection = notificationOwner = "";
        }
        return result;
    }); }
    @PluginMethod public void bootstrap(PluginCall call) { run(call, () -> {
        restoreActive(); foregroundLogin(); schedule(getContext()); return state();
    }); }
    @PluginMethod public void switchAccount(PluginCall call) { run(call, () -> {
        String next = call.getString("owner", ""), password = accounts().optString(next, "");
        if (password.isEmpty()) throw new BridgeError("signedOut");
        SchoolNotifications.clear(getContext());
        activate(next, password); foregroundLogin(); schedule(getContext()); return state();
    }); }
    @PluginMethod public void forgetAccount(PluginCall call) { run(call, () -> {
        String target = call.getString("owner", "");
        JSONObject saved = accounts(); saved.remove(target); storage.write("accounts-v1", saved.toString());
        if (target.equals(owner)) { remembered = false; storage.write("active", new JSONObject().put("owner", owner).toString()); }
        schedule(getContext()); return new JSObject();
    }); }
    @PluginMethod public void login(PluginCall call) { run(call, () -> {
        SchoolNotifications.clear(getContext()); SchoolNotifications.schedule(getContext(), false);
        storage.delete("active"); request(command("drop")); reset();
        String login = call.getString("login", "").trim(), password = call.getString("password", "");
        if (login.isEmpty() || password.isEmpty()) throw new BridgeError("requestFailed");
        try { request(command("login").put("login", login).put("password", password)); }
        finally { call.getData().remove("password"); }
        boolean keep = Boolean.TRUE.equals(call.getBoolean("remember", false));
        try {
            String cached = storage.read("account:" + login);
            JSONObject nextArchive = cached == null ? emptyArchive() : new JSONObject(cached);
            JSONObject list = accounts(); if (keep) list.put(login, password); else list.remove(login);
            storage.write("accounts-v1", list.toString());
            storage.write("active", new JSONObject().put("owner", login).toString());
            owner = login; remembered = keep; session = authenticated = true; archive = nextArchive;
            foregroundLogin(); schedule(getContext());
        } catch (Exception error) { request(command("drop")); reset(); throw error; }
        return state();
    }); }
    @PluginMethod public void logout(PluginCall call) { run(call, () -> {
        JSONObject list = accounts(); list.remove(owner); storage.write("accounts-v1", list.toString());
        storage.delete("active"); reset();
        SchoolNotifications.clear(getContext()); SchoolNotifications.schedule(getContext(), false);
        request(command("logout")); return new JSObject();
    }); }
    private static JSONObject loadSnapshot(Context context, String section, String week, int page, boolean foreground) throws Exception {
        return loadSnapshot(context, section, week, page, foreground, null, false);
    }
    private static JSONObject loadSnapshot(Context context, String section, String week, int page, boolean foreground,
                                           SchoolChanges.Delta batch, boolean monitoredInboxPage) throws Exception {
        JSONObject payload = request(command("load").put("section", section).put("week", week).put("page", page));
        String id = section.equals("Timetable") ? section + ":" + week : section.equals("Messages") ? section + ":" + page : section;
        String now = timestamp();
        JSONObject snapshot = new JSONObject().put("payload", payload).put("updated", now);
        JSONObject next = new JSONObject(archive.toString());
        // Keep the original boundary until a multi-page scan completes, so a
        // failed later page can be retried even after page one became known.
        if (batch != null && section.equals("Messages") && page == 1) SchoolChanges.startInboxScan(next);
        SchoolChanges.Delta changes = SchoolChanges.apply(next, section, id, payload, foreground, date(schoolDate()), now, monitoredInboxPage);
        next.getJSONObject("snapshots").put(id, snapshot);
        storage.write("account:" + owner, next.toString()); archive = next; authenticated = true;
        // Notify only after the encrypted snapshot and deduplication state were committed.
        if (batch != null) batch.add(changes);
        else if (!paused) SchoolNotifications.show(context, owner, changes);
        JSONObject result = new JSONObject(snapshot.toString());
        if (archive.has("activity")) result.put("activity", archive.getJSONObject("activity"));
        return result;
    }
    @PluginMethod public void load(PluginCall call) { run(call, () -> {
        requireOwner(call);
        return JSObject.fromJSONObject(loadSnapshot(getContext(), call.getString("section", ""),
            call.getString("week", ""), Math.max(1, call.getInt("page", 1)), true));
    }); }
    private static JSONArray sync(Context context, boolean foreground, BooleanSupplier stopped) throws Exception {
        JSONArray errors = new JSONArray();
        SchoolChanges.Delta batch = new SchoolChanges.Delta();
        Calendar monday = schoolDate();
        monday.add(Calendar.DAY_OF_MONTH, -((monday.get(Calendar.DAY_OF_WEEK) + 5) % 7));
        String thisWeek = date(monday); monday.add(Calendar.DAY_OF_MONTH, 7); String nextWeek = date(monday);
        String[][] requests = {{"Grades", thisWeek}, {"Messages", thisWeek}, {"Timetable", thisWeek}, {"Timetable", nextWeek}};
        for (String[] item : requests) {
            if (stopped.getAsBoolean() || paused || !SchoolNotifications.enabled(context)) break;
            try {
                SchoolChanges.InboxScan inbox = item[0].equals("Messages")
                    ? new SchoolChanges.InboxScan(SchoolChanges.inboxScanBaseline(archive)) : null;
                JSONObject snapshot = loadSnapshot(context, item[0], item[1], 1, foreground, batch, false);
                int page = 1;
                boolean more = inbox != null && inbox.continueAfter(snapshot.getJSONObject("payload"));
                while (more && !stopped.getAsBoolean() && !paused && SchoolNotifications.enabled(context)) {
                    snapshot = loadSnapshot(context, "Messages", item[1], ++page, foreground, batch, true);
                    more = inbox.continueAfter(snapshot.getJSONObject("payload"));
                }
                if (inbox != null && !more) {
                    JSONObject next = new JSONObject(archive.toString());
                    if (SchoolChanges.finishInboxScan(next)) {
                        storage.write("account:" + owner, next.toString()); archive = next;
                    }
                }
            }
            catch (BridgeError error) { errors.put(error.getMessage()); if (error.getMessage().equals("verification") || error.getMessage().equals("signedOut")) break; }
            catch (Exception ignored) { errors.put("storageFailed"); }
        }
        // A single notification per category includes both monitored weeks and
        // every newly discovered inbox page. Each contributing snapshot was saved.
        if (!paused) SchoolNotifications.show(context, owner, batch);
        return errors;
    }
    @PluginMethod public void syncSchool(PluginCall call) { run(call, () -> {
        requireOwner(call); foregroundLogin();
        JSONArray errors = sync(getContext(), true, () -> false);
        JSObject result = state(); result.put("syncErrors", errors); return result;
    }); }
    static Future<Boolean> backgroundSync(Context context, BooleanSupplier stopped) {
        return worker.submit(() -> {
            ensureStorage(context);
            if (stopped.getAsBoolean() || paused || !SchoolNotifications.enabled(context)) return true;
            restoreActive();
            if (!backgroundEligible(context)) return true;
            // A forgotten credential or signed-out active marker must stop future network work.
            String active = storage.read("active");
            if (active == null || !owner.equals(new JSONObject(active).optString("owner")) || accounts().optString(owner, "").isEmpty()) return true;
            return SchoolChanges.backgroundComplete(sync(context, false, stopped));
        });
    }
    private JSObject monitoringState() throws Exception {
        String permission = "granted";
        if (Build.VERSION.SDK_INT >= 33 && getPermissionState("notifications") != PermissionState.GRANTED)
            permission = getPermissionState("notifications") == PermissionState.DENIED ? "denied" : "prompt";
        else if (!SchoolNotifications.allowed(getContext())) permission = "denied";
        return new JSObject().put("enabled", SchoolNotifications.enabled(getContext())).put("permission", permission)
            .put("background", backgroundEligible(getContext()));
    }
    @PluginMethod public void monitoring(PluginCall call) { run(call, this::monitoringState); }
    @PluginMethod public void setMonitoring(PluginCall call) {
        boolean enable = Boolean.TRUE.equals(call.getBoolean("enabled", true));
        if (enable && Boolean.TRUE.equals(call.getBoolean("requestPermission", false))) {
            PermissionState permission = getPermissionState("notifications");
            if (!SchoolNotifications.allowed(getContext()) && (Build.VERSION.SDK_INT < 33
                || permission == PermissionState.DENIED || permission == PermissionState.GRANTED)) {
                if (getActivity() == null) { call.reject("requestFailed"); return; }
                // Only the user's explicit Allow notifications action may open Settings.
                getActivity().runOnUiThread(() -> {
                    try {
                        Intent settings = Build.VERSION.SDK_INT >= 26
                            ? new Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS).putExtra(Settings.EXTRA_APP_PACKAGE, getContext().getPackageName())
                            : new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS).setData(Uri.parse("package:" + getContext().getPackageName()));
                        getActivity().startActivity(settings);
                        saveMonitoring(call);
                    } catch (Exception ignored) { call.reject("requestFailed"); }
                });
                return;
            }
            if (Build.VERSION.SDK_INT >= 33 && permission != PermissionState.GRANTED) {
                requestPermissionForAlias("notifications", call, "schoolPermissionResult"); return;
            }
        }
        saveMonitoring(call);
    }
    @PermissionCallback private void schoolPermissionResult(PluginCall call) { saveMonitoring(call); }
    private void saveMonitoring(PluginCall call) { run(call, () -> {
        boolean enable = Boolean.TRUE.equals(call.getBoolean("enabled", true));
        if (!SchoolNotifications.preferences(getContext()).edit().putBoolean("enabled", enable).commit()) throw new BridgeError("storageFailed");
        if (!enable) SchoolNotifications.clear(getContext());
        schedule(getContext()); return monitoringState();
    }); }
    @PluginMethod public void setSchoolPaused(PluginCall call) { run(call, () -> {
        paused = Boolean.TRUE.equals(call.getBoolean("paused", false)); schedule(getContext()); return new JSObject();
    }); }
    @PluginMethod public void detail(PluginCall call) { run(call, () -> {
        requireOwner(call); String id = call.getString("id", "");
        JSONObject message = request(command("detail").put("id", id)); message.put("unread", false);
        JSONObject next = new JSONObject(archive.toString()); next.getJSONObject("details").put(id, message);
        JSONObject snapshots = next.getJSONObject("snapshots"); Iterator<String> keys = snapshots.keys();
        while (keys.hasNext()) {
            String key = keys.next(); if (!key.startsWith("Messages:")) continue;
            JSONArray items = snapshots.getJSONObject(key).getJSONObject("payload").getJSONObject("Messages").getJSONArray("items");
            for (int i = 0; i < items.length(); i++) if (id.equals(items.getJSONObject(i).optString("id"))) items.getJSONObject(i).put("unread", false);
        }
        storage.write("account:" + owner, next.toString()); archive = next;
        return new JSObject().put("message", message);
    }); }
    @PluginMethod public void language(PluginCall call) { run(call, () -> {
        String language = call.getString("language", "en");
        request(command("language").put("language", language));
        SchoolNotifications.preferences(getContext()).edit().putString("language", language).apply(); return new JSObject();
    }); }
    @PluginMethod public void github(PluginCall call) { open(call, "https://github.com/chillyfrog123/klaso-mobile"); }
    @PluginMethod public void official(PluginCall call) { open(call, "https://synergia.librus.pl/"); }
    private void open(PluginCall call, String url) {
        getActivity().runOnUiThread(() -> {
            try { getActivity().startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(url))); call.resolve(); }
            catch (Exception ignored) { call.reject("requestFailed"); }
        });
    }
}
