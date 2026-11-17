package pl.libruslite.mobile;

import org.json.JSONArray;
import org.json.JSONObject;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.Calendar;
import java.text.SimpleDateFormat;
import java.util.Locale;
import java.util.TimeZone;

/** Pure change tracking. The caller commits this together with the successful snapshot. */
final class SchoolChanges {
    static final class Delta {
        int grades, messages, substitutions, cancellations;
        void add(Delta other) {
            grades += other.grades; messages += other.messages;
            substitutions += other.substitutions; cancellations += other.cancellations;
        }
    }

    /** A null set means there has never been an inbox baseline. Cached browsing
     * pages are already known to the user and must not later become new alerts. */
    static Set<String> knownMessages(JSONObject archive) throws Exception {
        JSONObject meta = archive.optJSONObject("schoolWatch");
        JSONArray seen = meta == null ? null : meta.optJSONArray("seenMessages");
        boolean initialized = seen != null;
        Set<String> result = new HashSet<>(list(seen));
        JSONObject snapshots = archive.optJSONObject("snapshots");
        if (snapshots != null) {
            Iterator<String> pages = snapshots.keys();
            while (pages.hasNext()) {
                String page = pages.next();
                if (!page.startsWith("Messages:")) continue;
                JSONObject snapshot = snapshots.optJSONObject(page);
                JSONObject payload = snapshot == null ? null : snapshot.optJSONObject("payload");
                JSONObject messages = payload == null ? null : payload.optJSONObject("Messages");
                if (messages == null) continue;
                initialized = true;
                JSONArray rows = messages.optJSONArray("items");
                for (int i = 0; rows != null && i < rows.length(); i++) {
                    String id = rows.getJSONObject(i).optString("id");
                    if (!id.isEmpty()) result.add(id);
                }
            }
        }
        result.remove("");
        return initialized ? result : null;
    }
    static Set<String> inboxScanBaseline(JSONObject archive) throws Exception {
        JSONObject meta = archive.optJSONObject("schoolWatch");
        JSONArray pending = meta == null ? null : meta.optJSONArray("inboxScanBaseline");
        return pending == null ? knownMessages(archive) : new HashSet<>(list(pending));
    }
    static void startInboxScan(JSONObject archive) throws Exception {
        JSONObject meta = metadata(archive);
        if (meta.has("inboxScanBaseline")) return;
        Set<String> baseline = knownMessages(archive);
        if (baseline != null) meta.put("inboxScanBaseline", new JSONArray(baseline));
    }
    static boolean finishInboxScan(JSONObject archive) {
        JSONObject meta = archive.optJSONObject("schoolWatch");
        if (meta == null || !meta.has("inboxScanBaseline")) return false;
        meta.remove("inboxScanBaseline"); return true;
    }

    /** Page beyond the first only to bridge an entirely new first page back to
     * the existing baseline. Some gateways ignore page, so repeated IDs stop it. */
    static final class InboxScan {
        private final Set<String> known;
        private final Set<String> visited = new HashSet<>();
        private int consumed, pages;
        InboxScan(Set<String> baseline) { known = baseline == null ? null : new HashSet<>(baseline); }
        boolean continueAfter(JSONObject payload) throws Exception {
            JSONObject messages = payload.getJSONObject("Messages");
            JSONArray rows = messages.getJSONArray("items");
            int total = messages.optInt("total", rows.length());
            pages++; consumed += rows.length();
            if (known == null || rows.length() == 0 || total <= 20 || consumed >= total || pages >= 100) return false;
            for (int i = 0; i < rows.length(); i++) {
                String id = rows.getJSONObject(i).optString("id");
                if (id.isEmpty() || known.contains(id) || !visited.add(id)) return false;
            }
            return true;
        }
    }

    /** Required-action errors wait for the next normal period, not WorkManager's
     * short transport-error retry backoff. Foreground login can resolve them. */
    static boolean backgroundComplete(JSONArray errors) {
        if (errors.length() == 0) return true;
        for (int i = 0; i < errors.length(); i++)
            if ("verification".equals(errors.optString(i)) || "signedOut".equals(errors.optString(i))) return true;
        return false;
    }

    private static JSONObject metadata(JSONObject archive) throws Exception {
        JSONObject meta = archive.optJSONObject("schoolWatch");
        if (meta == null) { meta = new JSONObject(); archive.put("schoolWatch", meta); }
        return meta;
    }
    private static JSONArray gradeItems(JSONObject archive) {
        JSONObject snapshots = archive.optJSONObject("snapshots");
        JSONObject snapshot = snapshots == null ? null : snapshots.optJSONObject("Grades");
        JSONObject payload = snapshot == null ? null : snapshot.optJSONObject("payload");
        JSONObject grades = payload == null ? null : payload.optJSONObject("Grades");
        return grades == null ? null : grades.optJSONArray("items");
    }
    private static List<String> list(JSONArray values) {
        List<String> result = new ArrayList<>();
        if (values != null) for (int i = 0; i < values.length(); i++) result.add(values.optString(i));
        return result;
    }
    /** Values exactly match the TypeScript gradeKeys helper, including duplicate occurrence indices. */
    static LinkedHashMap<String, String> gradeKeys(JSONArray items) throws Exception {
        LinkedHashMap<String, String> result = new LinkedHashMap<>();
        Map<String, Integer> occurrences = new HashMap<>();
        for (int i = 0; items != null && i < items.length(); i++) {
            JSONObject grade = items.getJSONObject(i);
            StringBuilder fields = new StringBuilder("[");
            for (String name : new String[]{"subject", "value", "category", "date", "teacher", "semester"}) {
                if (fields.length() > 1) fields.append(',');
                fields.append(jsonString(grade.optString(name, "")));
            }
            String fallback = fields.append(']').toString();
            int occurrence = occurrences.getOrDefault(fallback, 0);
            occurrences.put(fallback, occurrence + 1);
            fallback += "#" + occurrence;
            String id = grade.optString("id", "");
            result.put(id.isEmpty() ? fallback : "id:" + id, fallback);
        }
        return result;
    }
    // Android's JSON writer escapes slashes and some printable Unicode differently from
    // JSON.stringify; use the ECMAScript representation for the cross-language fallback ID.
    private static String jsonString(String value) {
        StringBuilder result = new StringBuilder("\"");
        for (int i = 0; i < value.length(); i++) {
            char c = value.charAt(i);
            switch (c) {
                case '"': result.append("\\\""); break;
                case '\\': result.append("\\\\"); break;
                case '\b': result.append("\\b"); break;
                case '\f': result.append("\\f"); break;
                case '\n': result.append("\\n"); break;
                case '\r': result.append("\\r"); break;
                case '\t': result.append("\\t"); break;
                default:
                    if (c < 32) result.append(String.format(Locale.ROOT, "\\u%04x", (int)c));
                    else result.append(c);
            }
        }
        return result.append('"').toString();
    }
    private static JSONArray keys(JSONArray items) throws Exception {
        return new JSONArray(gradeKeys(items).keySet());
    }
    /** Upgrade older cached grades that did not include Librus IDs without flagging every old grade. */
    private static JSONArray migrate(JSONArray old, LinkedHashMap<String, String> current) {
        if (old == null) return null;
        Set<String> prior = new HashSet<>(list(old));
        for (Map.Entry<String, String> entry : current.entrySet())
            if (prior.remove(entry.getValue())) prior.add(entry.getKey());
        return new JSONArray(prior);
    }
    static void beginLogin(JSONObject archive, String timestamp) throws Exception {
        JSONObject meta = metadata(archive);
        JSONArray checkpoint = meta.optJSONArray("foregroundGrades");
        if (checkpoint == null && gradeItems(archive) != null) checkpoint = keys(gradeItems(archive));
        if (checkpoint != null) meta.put("baseline", new JSONArray(checkpoint.toString()));
        else meta.remove("baseline");
        meta.put("since", timestamp);
        activity(archive, meta, gradeItems(archive), timestamp);
    }
    private static void activity(JSONObject archive, JSONObject meta, JSONArray items, String timestamp) throws Exception {
        JSONArray baseline = meta.optJSONArray("baseline");
        Set<String> known = new HashSet<>(list(baseline));
        JSONArray added = new JSONArray();
        if (baseline != null)
            for (String key : gradeKeys(items).keySet()) if (!known.contains(key)) added.put(key);
        archive.put("activity", new JSONObject().put("newGradeIds", added)
            .put("baselineReady", baseline != null).put("since", meta.optString("since", timestamp))
            .put("updated", timestamp));
    }
    static Delta apply(JSONObject archive, String section, String cacheKey, JSONObject payload,
                       boolean foreground, String today, String timestamp) throws Exception {
        return apply(archive, section, cacheKey, payload, foreground, today, timestamp, false);
    }
    static Delta apply(JSONObject archive, String section, String cacheKey, JSONObject payload,
                       boolean foreground, String today, String timestamp, boolean monitoredInboxPage) throws Exception {
        Delta delta = new Delta();
        JSONObject meta = metadata(archive);
        if (section.equals("Grades")) {
            JSONArray items = payload.getJSONObject("Grades").getJSONArray("items");
            LinkedHashMap<String, String> current = gradeKeys(items);
            JSONArray seen = migrate(meta.optJSONArray("seenGrades"), current);
            if (seen == null && gradeItems(archive) != null) seen = migrate(keys(gradeItems(archive)), current);
            Set<String> all = new HashSet<>(list(seen));
            for (String key : current.keySet()) { if (seen != null && !all.contains(key)) delta.grades++; all.add(key); }
            meta.put("seenGrades", new JSONArray(all));
            JSONArray baseline = migrate(meta.optJSONArray("baseline"), current);
            if (baseline == null && foreground) baseline = new JSONArray(current.keySet());
            if (baseline != null) meta.put("baseline", baseline);
            if (foreground) meta.put("foregroundGrades", new JSONArray(current.keySet()));
            activity(archive, meta, items, timestamp);
        } else if (section.equals("Messages") && (cacheKey.equals("Messages:1") || monitoredInboxPage)) {
            Set<String> seen = knownMessages(archive);
            Set<String> boundary = monitoredInboxPage ? inboxScanBaseline(archive) : null;
            Set<String> all = seen == null ? new HashSet<>() : new HashSet<>(seen);
            JSONArray rows = payload.getJSONObject("Messages").getJSONArray("items");
            for (int i = 0; i < rows.length(); i++) {
                String id = rows.getJSONObject(i).optString("id");
                if (id.isEmpty()) continue;
                if (boundary != null && boundary.contains(id)) break;
                if (seen != null && !all.contains(id)) delta.messages++;
                all.add(id);
            }
            meta.put("seenMessages", new JSONArray(all));
        } else if (section.equals("Timetable")) {
            JSONObject weeks = meta.optJSONObject("weeks");
            if (weeks == null) { weeks = new JSONObject(); meta.put("weeks", weeks); }
            JSONObject prior = weeks.optJSONObject(cacheKey);
            if (prior == null) {
                JSONObject cached = archive.getJSONObject("snapshots").optJSONObject(cacheKey);
                if (cached != null) prior = timetableKeys(cached.getJSONObject("payload").getJSONArray("Timetable"));
            }
            JSONObject current = timetableKeys(payload.getJSONArray("Timetable"));
            Iterator<String> events = current.keys();
            JSONObject seen = prior == null ? new JSONObject() : new JSONObject(prior.toString());
            while (events.hasNext()) {
                String event = events.next();
                JSONObject row = current.getJSONObject(event);
                if (prior != null && !prior.has(event) && row.optString("date").compareTo(today) >= 0) {
                    if (row.optBoolean("canceled")) delta.cancellations++;
                    else if (row.optBoolean("substitution")) delta.substitutions++;
                }
                seen.put(event, row);
            }
            weeks.put(cacheKey, seen);
            // Retain only current/future monitoring weeks; dates are ISO and therefore sortable.
            List<String> expired = new ArrayList<>();
            Iterator<String> names = weeks.keys();
            SimpleDateFormat format = new SimpleDateFormat("yyyy-MM-dd", Locale.ROOT);
            format.setTimeZone(TimeZone.getTimeZone("UTC"));
            Calendar cutoff = Calendar.getInstance(TimeZone.getTimeZone("UTC"), Locale.ROOT);
            cutoff.setTime(format.parse(today)); cutoff.add(Calendar.DAY_OF_MONTH, -7);
            String oldest = format.format(cutoff.getTime());
            while (names.hasNext()) { String name = names.next(); if (name.substring("Timetable:".length()).compareTo(oldest) < 0) expired.add(name); }
            for (String name : expired) weeks.remove(name);
        }
        return delta;
    }
    private static JSONObject timetableKeys(JSONArray rows) throws Exception {
        JSONObject keys = new JSONObject();
        for (int i = 0; i < rows.length(); i++) {
            JSONObject row = rows.getJSONObject(i);
            if (!row.optBoolean("canceled") && !row.optBoolean("substitution")) continue;
            JSONArray fields = new JSONArray();
            for (String name : new String[]{"date", "number", "subject", "teacher", "room", "from", "to", "note"}) fields.put(row.optString(name));
            fields.put(row.optBoolean("canceled")).put(row.optBoolean("substitution"));
            keys.put(fields.toString(), row);
        }
        return keys;
    }
}
