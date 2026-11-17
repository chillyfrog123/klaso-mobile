package pl.libruslite.mobile;

import org.json.JSONArray;
import org.json.JSONObject;
import org.junit.Test;
import java.util.HashSet;
import java.util.Set;
import static org.junit.Assert.*;

public class SchoolInboxTest {
    private JSONObject archive() throws Exception { return new JSONObject("{\"snapshots\":{},\"details\":{}}"); }
    private JSONObject messages(int total, String... ids) throws Exception {
        JSONArray items = new JSONArray();
        for (String id : ids) items.put(new JSONObject().put("id", id));
        return new JSONObject().put("Messages", new JSONObject().put("items", items).put("total", total));
    }
    private SchoolChanges.Delta apply(JSONObject state, int page, JSONObject payload, boolean monitoring) throws Exception {
        String key = "Messages:" + page;
        SchoolChanges.Delta delta = SchoolChanges.apply(state, "Messages", key, payload, true,
            "2026-11-17", "2026-11-17T12:00:00Z", monitoring);
        state.getJSONObject("snapshots").put(key, new JSONObject().put("payload", payload));
        return delta;
    }
    private Set<String> known(String... ids) {
        Set<String> result = new HashSet<>();
        for (String id : ids) result.add(id);
        return result;
    }
    @Test public void firstEverInboxSeedsSilentlyWithoutPagingIntoHistory() throws Exception {
        JSONObject state = archive();
        SchoolChanges.InboxScan scan = new SchoolChanges.InboxScan(SchoolChanges.knownMessages(state));
        JSONObject first = messages(80, "old-a", "old-b");
        assertEquals(0, apply(state, 1, first, false).messages);
        assertFalse(scan.continueAfter(first));
        assertEquals(0, apply(state, 3, messages(80, "historical"), false).messages);
        assertTrue(SchoolChanges.knownMessages(state).contains("historical"));
    }
    @Test public void scannerContinuesThroughAllNewPagesAndStopsAtKnownIds() throws Exception {
        SchoolChanges.InboxScan scan = new SchoolChanges.InboxScan(known("seen"));
        assertTrue(scan.continueAfter(messages(80, "new-a", "new-b")));
        assertTrue(scan.continueAfter(messages(80, "new-c", "new-d")));
        assertFalse(scan.continueAfter(messages(80, "new-e", "seen", "older")));
    }
    @Test public void scannerStopsAtIgnoredPagingEmptyPagesAndAdvertisedEnd() throws Exception {
        SchoolChanges.InboxScan repeating = new SchoolChanges.InboxScan(known("seen"));
        JSONObject first = messages(80, "new-a", "new-b");
        assertTrue(repeating.continueAfter(first));
        assertFalse(repeating.continueAfter(first));
        assertFalse(new SchoolChanges.InboxScan(known()).continueAfter(messages(80)));
        assertFalse(new SchoolChanges.InboxScan(known()).continueAfter(messages(2, "a", "b")));
        SchoolChanges.InboxScan end = new SchoolChanges.InboxScan(known("seen"));
        String[] page = new String[20]; for (int i = 0; i < page.length; i++) page[i] = "new-" + i;
        assertTrue(end.continueAfter(messages(21, page)));
        assertFalse(end.continueAfter(messages(21, "last-new")));
    }
    @Test public void additionalMonitoredPageCountsOnlyBeforeTheKnownBoundary() throws Exception {
        JSONObject state = archive();
        apply(state, 1, messages(80, "seen"), false);
        assertEquals(2, apply(state, 1, messages(80, "new-a", "new-b"), false).messages);
        assertEquals(1, apply(state, 2, messages(80, "new-c", "seen", "older-unseen"), true).messages);
        assertEquals(0, apply(state, 2, messages(80, "new-c", "seen", "older-unseen"), true).messages);
    }
    @Test public void cachedHistoricalBrowsingDoesNotTurnIntoNewNotifications() throws Exception {
        JSONObject state = archive();
        apply(state, 1, messages(80, "seen"), false);
        apply(state, 4, messages(80, "historical"), false);
        assertEquals(0, apply(state, 1, messages(80, "historical", "seen"), false).messages);
        assertEquals(0, apply(state, 2, messages(80, "historical", "uncached-old"), true).messages);
    }
    @Test public void interruptedScanRetainsItsOriginalBoundaryForRetryWithoutDuplicates() throws Exception {
        JSONObject state = archive();
        apply(state, 1, messages(80, "seen"), false);
        SchoolChanges.startInboxScan(state);
        apply(state, 1, messages(80, "new-a", "new-b"), false);
        apply(state, 2, messages(80, "new-c", "new-d"), true);
        // A later network request fails; only committed pages and the original
        // baseline survive a process restart.
        state = new JSONObject(state.toString());
        SchoolChanges.InboxScan retry = new SchoolChanges.InboxScan(SchoolChanges.inboxScanBaseline(state));
        JSONObject first = messages(80, "new-a", "new-b");
        assertEquals(0, apply(state, 1, first, false).messages);
        assertTrue(retry.continueAfter(first));
        JSONObject second = messages(80, "new-c", "new-between", "new-d");
        assertEquals(1, apply(state, 2, second, true).messages);
        assertTrue(retry.continueAfter(second));
        JSONObject last = messages(80, "new-e", "seen", "old-uncached");
        assertEquals(1, apply(state, 3, last, true).messages);
        assertFalse(retry.continueAfter(last));
        assertTrue(SchoolChanges.finishInboxScan(state));
        assertTrue(SchoolChanges.inboxScanBaseline(state).contains("new-e"));
    }
    @Test public void notificationBatchIncludesBothWeeksAndRequiredActionsDoNotRapidRetry() throws Exception {
        SchoolChanges.Delta first = new SchoolChanges.Delta(); first.substitutions = 2; first.messages = 20;
        SchoolChanges.Delta second = new SchoolChanges.Delta(); second.cancellations = 3; second.messages = 5;
        first.add(second);
        assertEquals(2, first.substitutions); assertEquals(3, first.cancellations); assertEquals(25, first.messages);
        assertTrue(SchoolChanges.backgroundComplete(new JSONArray()));
        assertTrue(SchoolChanges.backgroundComplete(new JSONArray().put("requestFailed").put("verification")));
        assertTrue(SchoolChanges.backgroundComplete(new JSONArray().put("signedOut")));
        assertFalse(SchoolChanges.backgroundComplete(new JSONArray().put("requestFailed")));
    }
}
