package pl.libruslite.mobile;

import android.content.Intent;
import android.net.Uri;
import com.getcapacitor.JSObject;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;
import org.json.JSONObject;
import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;
import java.util.TimeZone;
import java.util.Iterator;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

@CapacitorPlugin(name="Librus")
public class LibrusPlugin extends Plugin {
    static { System.loadLibrary("librus_mobile_core"); }
    private static native String nativeRequest(String request);
    // Process-wide state also covers Activity/plugin recreation. Rust owns one
    // process-wide session, so separate plugin instances must not own separate queues.
    // All account changes, network requests and encrypted writes are ordered on one worker.
    // UI cancellation also uses generation tickets, so old work cannot enter a new account.
    private static final ExecutorService worker=Executors.newSingleThreadExecutor();
    private static SecureStore storage;
    private static String owner="";
    private static boolean session=false,remembered=false,authenticated=false;
    private static JSONObject archive=emptyArchive();
    private static JSONObject emptyArchive() { try { return new JSONObject("{\"snapshots\":{},\"details\":{}}"); } catch(Exception impossible){throw new AssertionError();} }
    interface Work { JSObject run() throws Exception; }
    private static class BridgeError extends Exception { BridgeError(String code){super(code);} }
    private void run(PluginCall call,Work action) {
        worker.execute(()->{try{if(storage==null)storage=new SecureStore(getContext());call.resolve(action.run());}
            catch(BridgeError error){call.reject(error.getMessage());}
            catch(Exception ignored){call.reject("storageFailed");}});
    }
    private JSONObject request(JSONObject input) throws Exception {
        JSONObject result=new JSONObject(nativeRequest(input.toString()));
        if(!result.optBoolean("ok"))throw new BridgeError(result.optString("code","requestFailed"));
        return result.getJSONObject("value");
    }
    private JSONObject command(String op) throws Exception {return new JSONObject().put("op",op);}
    private JSObject state() throws Exception {
        JSObject out=new JSObject();
        out.put("account",owner.isEmpty()?JSONObject.NULL:new JSONObject().put("owner",owner).put("session",session).put("remembered",remembered).put("authenticated",authenticated));
        out.put("archive",archive);out.put("accounts",accounts().names()==null?new org.json.JSONArray():accounts().names());return out;
    }
    private JSONObject accounts() throws Exception {String raw=storage.read("accounts-v1");return raw==null?new JSONObject():new JSONObject(raw);}
    private void activate(String nextOwner, String password) throws Exception {
        String cached=storage.read("account:"+nextOwner);
        request(command("drop"));owner="";session=false;remembered=false;authenticated=false;archive=emptyArchive();
        try {
            request(command("restore").put("login",nextOwner).put("password",password));
            storage.write("active",new JSONObject().put("owner",nextOwner).toString());
            owner=nextOwner;session=true;remembered=true;archive=cached==null?emptyArchive():new JSONObject(cached);
        } catch(Exception error) {request(command("drop"));throw error;}
    }
    @PluginMethod public void switchAccount(PluginCall call) {run(call,()->{
        String next=call.getString("owner","");String password=accounts().optString(next,"");
        if(password.isEmpty())throw new BridgeError("signedOut");activate(next,password);return state();
    });}
    @PluginMethod public void forgetAccount(PluginCall call) {run(call,()->{
        String target=call.getString("owner","");JSONObject saved=accounts();saved.remove(target);storage.write("accounts-v1",saved.toString());
        if(target.equals(owner)){remembered=false;storage.write("active",new JSONObject().put("owner",owner).toString());}
        return new JSObject();
    });}
    private void requireOwner(PluginCall call) throws Exception {
        if(!session||owner.isEmpty()||!owner.equals(call.getString("owner")))throw new BridgeError("signedOut");
    }
    @PluginMethod public void bootstrap(PluginCall call) {run(call,()->{
        if (!owner.isEmpty()) return state();
        String saved=storage.read("active");
        if(saved!=null){JSONObject active=new JSONObject(saved);String name=active.getString("owner");JSONObject list=accounts();
            if(active.optBoolean("remembered")&&active.has("password")){list.put(name,active.getString("password"));storage.write("accounts-v1",list.toString());storage.write("active",new JSONObject().put("owner",name).toString());}
            String password=list.optString(name,"");
            if(!password.isEmpty())activate(name,password);
            else {owner=name;String cached=storage.read("account:"+owner);archive=cached==null?emptyArchive():new JSONObject(cached);}
        }
        return state();
    });}
    @PluginMethod public void login(PluginCall call) {run(call,()->{
        storage.delete("active");request(command("drop"));owner="";session=false;remembered=false;authenticated=false;archive=emptyArchive();
        String login=call.getString("login","").trim(),password=call.getString("password","");
        if(login.isEmpty()||password.isEmpty())throw new BridgeError("requestFailed");
        try { request(command("login").put("login",login).put("password",password)); }
        finally { call.getData().remove("password"); }
        boolean keep=Boolean.TRUE.equals(call.getBoolean("remember",false));
        JSONObject active=new JSONObject().put("owner",login).put("remembered",keep);
        if(keep)active.put("password",password);
        try {
            String cached=storage.read("account:"+login);
            JSONObject nextArchive=cached==null?emptyArchive():new JSONObject(cached);
            JSONObject list=accounts();if(keep)list.put(login,password);else list.remove(login);storage.write("accounts-v1",list.toString());
            storage.write("active",new JSONObject().put("owner",login).toString());
            owner=login;remembered=keep;session=true;authenticated=true;archive=nextArchive;
        } catch(Exception error) {request(command("drop"));throw error;}
        finally {call.getData().remove("password");active.remove("password");}
        return state();
    });}
    @PluginMethod public void logout(PluginCall call) {run(call,()->{
        JSONObject list=accounts();list.remove(owner);storage.write("accounts-v1",list.toString());
        storage.delete("active");owner="";session=false;remembered=false;authenticated=false;archive=emptyArchive();
        request(command("logout"));return new JSObject();
    });}
    @PluginMethod public void load(PluginCall call) {run(call,()->{
        requireOwner(call);
        String section=call.getString("section",""),week=call.getString("week","");
        int page=Math.max(1,call.getInt("page",1));
        JSONObject payload=request(command("load").put("section",section).put("week",week).put("page",page));
        String id=section.equals("Timetable")?section+":"+week:section.equals("Messages")?section+":"+page:section;
        authenticated=true;
        SimpleDateFormat timestamp = new SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'", Locale.ROOT);
        timestamp.setTimeZone(TimeZone.getTimeZone("UTC"));
        JSONObject snapshot=new JSONObject().put("payload",payload).put("updated",timestamp.format(new Date()));
        JSONObject next=new JSONObject(archive.toString());next.getJSONObject("snapshots").put(id,snapshot);
        storage.write("account:"+owner,next.toString());archive=next;
        return JSObject.fromJSONObject(snapshot);
    });}
    @PluginMethod public void detail(PluginCall call) {run(call,()->{
        requireOwner(call);String id=call.getString("id","");
        JSONObject message=request(command("detail").put("id",id));
        // The successful read endpoint is allowed to mark the remote message read.
        message.put("unread",false);
        JSONObject next=new JSONObject(archive.toString());next.getJSONObject("details").put(id,message);
        JSONObject snapshots=next.getJSONObject("snapshots");Iterator<String> keys=snapshots.keys();
        while(keys.hasNext()){String k=keys.next();if(k.startsWith("Messages:")){
            org.json.JSONArray items=snapshots.getJSONObject(k).getJSONObject("payload").getJSONObject("Messages").getJSONArray("items");
            for(int i=0;i<items.length();i++){JSONObject item=items.getJSONObject(i);if(id.equals(item.optString("id")))item.put("unread",false);}
        }}
        storage.write("account:"+owner,next.toString());archive=next;
        return new JSObject().put("message",message);
    });}
    @PluginMethod public void language(PluginCall call) {run(call,()->{request(command("language").put("language",call.getString("language","en")));return new JSObject();});}
    @PluginMethod public void github(PluginCall call) {
        getActivity().runOnUiThread(()->{try{getActivity().startActivity(new Intent(Intent.ACTION_VIEW,Uri.parse("https://github.com/chillyfrog123/klaso-mobile")));call.resolve();}catch(Exception ignored){call.reject("requestFailed");}});
    }
    @PluginMethod public void official(PluginCall call) {
        getActivity().runOnUiThread(()->{try{getActivity().startActivity(new Intent(Intent.ACTION_VIEW,Uri.parse("https://synergia.librus.pl/")));call.resolve();}catch(Exception ignored){call.reject("requestFailed");}});
    }
}
