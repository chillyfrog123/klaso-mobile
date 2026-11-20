package pl.libruslite.mobile;

import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import com.getcapacitor.Plugin;
import com.getcapacitor.PluginCall;
import com.getcapacitor.PluginMethod;
import com.getcapacitor.annotation.CapacitorPlugin;

/** Local, user-requested actions on the text captured by the in-app menu. */
@CapacitorPlugin(name = "TextActions")
public class TextActionsPlugin extends Plugin {
    @PluginMethod
    public void copy(PluginCall call) {
        String text = call.getString("text");
        String label = call.getString("label");
        if (text == null || text.trim().isEmpty() || label == null || label.trim().isEmpty()) {
            call.reject("invalidTextAction");
            return;
        }
        getActivity().runOnUiThread(() -> {
            try {
                ClipboardManager clipboard = (ClipboardManager) getContext().getSystemService(Context.CLIPBOARD_SERVICE);
                if (clipboard == null) {
                    call.reject("copyFailed");
                    return;
                }
                clipboard.setPrimaryClip(ClipData.newPlainText(label, text));
                call.resolve();
            } catch (RuntimeException ignored) {
                call.reject("copyFailed");
            }
        });
    }

    @PluginMethod
    public void share(PluginCall call) {
        String text = call.getString("text");
        String title = call.getString("title");
        if (text == null || text.trim().isEmpty() || title == null || title.trim().isEmpty()) {
            call.reject("invalidTextAction");
            return;
        }
        getActivity().runOnUiThread(() -> {
            try {
                Intent send = new Intent(Intent.ACTION_SEND);
                send.setType("text/plain");
                send.putExtra(Intent.EXTRA_TEXT, text);
                getActivity().startActivity(Intent.createChooser(send, title));
                call.resolve();
            } catch (RuntimeException ignored) {
                call.reject("shareFailed");
            }
        });
    }
}
