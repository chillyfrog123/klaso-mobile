package pl.libruslite.mobile;

import org.json.JSONArray;
import org.json.JSONObject;
import org.junit.Test;
import static org.junit.Assert.*;

public class SchoolChangesTest {
    private static final String NOW = "2026-11-17T12:00:00.000Z";
    private JSONObject archive() throws Exception { return new JSONObject("{\"snapshots\":{},\"details\":{}}"); }
    private JSONObject grade(String id) throws Exception {
        return new JSONObject().put("id",id).put("subject","Math / A").put("value","5")
            .put("category","Test").put("date","2026-11-17").put("teacher","Teacher").put("semester","1");
    }
    private JSONObject grades(JSONObject... rows) throws Exception {
        JSONArray items=new JSONArray(); for(JSONObject row:rows)items.put(row);
        return new JSONObject().put("Grades",new JSONObject().put("items",items));
    }
    private SchoolChanges.Delta apply(JSONObject archive, String section, String key, JSONObject payload, boolean foreground) throws Exception {
        SchoolChanges.Delta delta=SchoolChanges.apply(archive,section,key,payload,foreground,"2026-11-17",NOW);
        archive.getJSONObject("snapshots").put(key,new JSONObject().put("payload",payload).put("updated",NOW));
        return delta;
    }
    private SchoolChanges.Delta applyGrades(JSONObject archive,boolean foreground,JSONObject... rows) throws Exception {
        return apply(archive,"Grades","Grades",grades(rows),foreground);
    }
    @Test public void firstFetchSeedsSilentlyAndRefreshDoesNotClearNewGrades() throws Exception {
        JSONObject state=archive(); SchoolChanges.beginLogin(state,NOW);
        assertEquals(0,applyGrades(state,true,grade("1")).grades);
        assertTrue(state.getJSONObject("activity").getBoolean("baselineReady"));
        assertEquals(1,applyGrades(state,true,grade("1"),grade("2")).grades);
        assertEquals("id:2",state.getJSONObject("activity").getJSONArray("newGradeIds").getString(0));
        assertEquals(0,applyGrades(state,true,grade("1"),grade("2")).grades);
        assertEquals(1,state.getJSONObject("activity").getJSONArray("newGradeIds").length());
    }
    @Test public void backgroundGradesSurviveTheNextLoginBaseline() throws Exception {
        JSONObject state=archive();SchoolChanges.beginLogin(state,NOW);applyGrades(state,true,grade("1"));
        assertEquals(1,applyGrades(state,false,grade("1"),grade("2")).grades);
        SchoolChanges.beginLogin(state,"2026-11-18T09:00:00Z");
        assertEquals("id:2",state.getJSONObject("activity").getJSONArray("newGradeIds").getString(0));
        applyGrades(state,true,grade("1"),grade("2"));SchoolChanges.beginLogin(state,"2026-11-19T09:00:00Z");
        assertEquals(0,state.getJSONObject("activity").getJSONArray("newGradeIds").length());
    }
    @Test public void upgradingCachedGradesToServerIdsDoesNotFloodNotifications() throws Exception {
        JSONObject state=archive();applyGrades(state,true,grade(""),grade(""));SchoolChanges.beginLogin(state,NOW);
        assertEquals(0,applyGrades(state,true,grade("1"),grade("2")).grades);
        assertEquals(0,state.getJSONObject("activity").getJSONArray("newGradeIds").length());
        assertEquals(1,applyGrades(state,true,grade("1"),grade("2"),grade("3")).grades);
    }
    @Test public void fallbackKeysCountStableIdRowsTooAndMatchJavascriptEscaping() throws Exception {
        JSONArray rows=new JSONArray().put(grade("1")).put(grade("")).put(grade(""));
        java.util.List<String> keys=new java.util.ArrayList<>(SchoolChanges.gradeKeys(rows).keySet());
        assertEquals("id:1",keys.get(0));
        assertEquals("[\"Math / A\",\"5\",\"Test\",\"2026-11-17\",\"Teacher\",\"1\"]#1",keys.get(1));
        assertTrue(keys.get(2).endsWith("#2"));
    }
    @Test public void separateAccountsHaveSeparateGradeHistory() throws Exception {
        JSONObject a=archive(),b=archive();SchoolChanges.beginLogin(a,NOW);SchoolChanges.beginLogin(b,NOW);
        applyGrades(a,true,grade("1"));applyGrades(a,true,grade("1"),grade("2"));
        assertEquals(0,applyGrades(b,true,grade("2")).grades);
        assertEquals(0,b.getJSONObject("activity").getJSONArray("newGradeIds").length());
        assertEquals(1,a.getJSONObject("activity").getJSONArray("newGradeIds").length());
    }
    private JSONObject messages(String... ids) throws Exception {
        JSONArray rows=new JSONArray();for(String id:ids)rows.put(new JSONObject().put("id",id).put("unread",false));
        return new JSONObject().put("Messages",new JSONObject().put("items",rows).put("total",rows.length()));
    }
    @Test public void messageReadStateDoesNotAffectNewMessageDetection() throws Exception {
        JSONObject state=archive();assertEquals(0,apply(state,"Messages","Messages:1",messages("a"),true).messages);
        JSONObject incoming=messages("a","b");incoming.getJSONObject("Messages").getJSONArray("items").getJSONObject(0).put("unread",true);
        assertEquals(1,apply(state,"Messages","Messages:1",incoming,true).messages);
        assertEquals(0,apply(state,"Messages","Messages:1",messages("a","b"),false).messages);
    }
    private JSONObject lesson(String date,boolean canceled,boolean substitution,String room) throws Exception {
        return new JSONObject().put("date",date).put("number","1").put("subject","Math").put("teacher","Teacher")
            .put("from","08:00").put("to","08:45").put("room",room).put("note","")
            .put("canceled",canceled).put("substitution",substitution);
    }
    private JSONObject timetable(JSONObject... lessons) throws Exception {
        JSONArray rows=new JSONArray();for(JSONObject lesson:lessons)rows.put(lesson);return new JSONObject().put("Timetable",rows);
    }
    @Test public void substitutionsAndCancellationsNotifyOnceAndIgnorePastChanges() throws Exception {
        JSONObject state=archive();String key="Timetable:2026-11-16";
        apply(state,"Timetable",key,timetable(lesson("2026-11-18",false,false,"101")),true);
        JSONObject changed=timetable(lesson("2026-11-18",false,true,"102"),lesson("2026-11-19",true,false,"101"),lesson("2026-11-16",true,false,"101"));
        SchoolChanges.Delta delta=apply(state,"Timetable",key,changed,false);
        assertEquals(1,delta.substitutions);assertEquals(1,delta.cancellations);
        delta=apply(state,"Timetable",key,changed,true);assertEquals(0,delta.substitutions+delta.cancellations);
        delta=apply(state,"Timetable",key,timetable(lesson("2026-11-18",false,true,"103")),true);
        assertEquals(1,delta.substitutions);
    }
    @Test public void firstTimetableSnapshotDoesNotNotifyForEveryExistingChange() throws Exception {
        assertEquals(0,apply(archive(),"Timetable","Timetable:2026-11-16",timetable(lesson("2026-11-18",true,false,"101")),true).cancellations);
    }
}
