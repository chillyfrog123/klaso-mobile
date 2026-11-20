package pl.libruslite.mobile;

import android.os.Bundle;
import android.view.ActionMode;
import android.webkit.WebView;
import com.getcapacitor.BridgeActivity;

public class MainActivity extends BridgeActivity {
    private ActionMode selectionMode;
    private Runnable hideSelectionToolbar;
    private long selectionGeneration;

    @Override public void onCreate(Bundle savedInstanceState) {
        registerPlugin(LibrusPlugin.class);
        registerPlugin(AppUpdaterPlugin.class);
        registerPlugin(TextActionsPlugin.class);
        super.onCreate(savedInstanceState);
    }

    @Override public void onActionModeStarted(ActionMode mode) {
        super.onActionModeStarted(mode);
        if (mode.getType() != ActionMode.TYPE_FLOATING || bridge == null) return;

        stopHidingSelectionToolbar();
        selectionMode = mode;
        selectionGeneration++;
        // Keep Android's selection handles while React offers its own menu trigger.
        // Inputs keep their ordinary toolbar when React does not handle this event.
        mode.hide(1500);
        WebView webView = bridge.getWebView();
        webView.evaluateJavascript(
            "(() => { const event = new Event('klaso-text-selection', { cancelable: true });"
                + " document.dispatchEvent(event); return event.defaultPrevented; })()",
            handled -> {
                if (selectionMode != mode) return;
                if (!"true".equals(handled)) {
                    selectionMode = null;
                    mode.hide(0);
                    return;
                }
                // ActionMode.hide has a short platform limit. Chromium can also
                // request a shorter hide when the user moves a selection handle.
                hideSelectionToolbar = new Runnable() {
                    @Override public void run() {
                        if (selectionMode != mode) return;
                        mode.hide(1500);
                        webView.postDelayed(this, 250);
                    }
                };
                hideSelectionToolbar.run();
            }
        );
    }

    @Override public void onActionModeFinished(ActionMode mode) {
        if (selectionMode == mode) {
            selectionMode = null;
            stopHidingSelectionToolbar();
            long generation = ++selectionGeneration;
            WebView webView = bridge.getWebView();
            webView.post(() -> {
                if (selectionMode != null || selectionGeneration != generation) return;
                // A later selection may already exist when this script executes.
                webView.evaluateJavascript(
                    "if (!window.getSelection() || window.getSelection().isCollapsed)"
                        + " document.dispatchEvent(new Event('klaso-text-selection-ended'));",
                    null
                );
            });
        }
        super.onActionModeFinished(mode);
    }

    private void stopHidingSelectionToolbar() {
        if (hideSelectionToolbar != null && bridge != null) {
            bridge.getWebView().removeCallbacks(hideSelectionToolbar);
        }
        hideSelectionToolbar = null;
    }

    @Override public void onPause() {
        if (selectionMode != null) selectionMode.finish();
        stopHidingSelectionToolbar();
        super.onPause();
    }

    @Override public void onDestroy() {
        selectionMode = null;
        selectionGeneration++;
        stopHidingSelectionToolbar();
        super.onDestroy();
    }
}
