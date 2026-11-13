package pl.libruslite.mobile;

import static org.junit.Assert.*;
import android.content.Context;
import android.util.AtomicFile;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import org.json.JSONObject;
import org.junit.Test;
import org.junit.runner.RunWith;
import java.io.File;
import java.io.FileOutputStream;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;

/** Runs only on an Android device, using explicit fictional keys and no network requests. */
@RunWith(AndroidJUnit4.class)
public class NativeBoundaryTest {
    private Context context() {
        return InstrumentationRegistry.getInstrumentation().getTargetContext();
    }
    private File file(String name) throws Exception {
        byte[] digest = MessageDigest.getInstance("SHA-256").digest(name.getBytes(StandardCharsets.UTF_8));
        StringBuilder hash = new StringBuilder();
        for (byte b : digest) hash.append(String.format("%02x", b));
        return new File(new File(context().getNoBackupFilesDir(), "private-v1"), hash.toString());
    }
    @Test public void secureSnapshotsPersistAndRejectCrossAccountSubstitution() throws Exception {
        String a = "test-fictional-account-a", b = "test-fictional-account-b";
        String fictional = "{\"message\":\"FICTIONAL_ONLY_314159\",\"updated\":\"2026-11-09\"}";
        SecureStore store = new SecureStore(context());
        try {
            store.write(a, fictional);
            byte[] ciphertext = new AtomicFile(file(a)).readFully();
            assertFalse(new String(ciphertext, StandardCharsets.UTF_8).contains("FICTIONAL_ONLY_314159"));
            assertEquals(fictional, new SecureStore(context()).read(a));
            store.write(b, "{}");
            try (FileOutputStream out = new FileOutputStream(file(b))) { out.write(ciphertext); }
            assertThrows(Exception.class, () -> store.read(b));
            assertEquals(fictional, store.read(a));
            store.delete(a);
            assertNull(store.read(a));
        } finally {
            store.delete(a);
            store.delete(b);
        }
    }
    @Test public void jniDemoUsesSharedRustCalculationsWithoutLoggingIn() throws Exception {
        Method request = LibrusPlugin.class.getDeclaredMethod("nativeRequest", String.class);
        request.setAccessible(true);
        String response = (String) request.invoke(null, "{\"op\":\"demo\",\"week\":\"2026-11-09\"}");
        JSONObject result = new JSONObject(response);
        assertTrue(result.getBoolean("ok"));
        double average = result.getJSONObject("value").getJSONObject("Grades").getJSONObject("Grades")
            .getJSONArray("summaries").getJSONObject(0).getJSONArray("overall").getDouble(0);
        assertEquals(5.0625, average, 0.0001);
    }
}
