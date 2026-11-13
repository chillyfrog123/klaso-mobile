package pl.libruslite.mobile;
import android.os.Bundle;
import com.getcapacitor.BridgeActivity;
public class MainActivity extends BridgeActivity {
    @Override public void onCreate(Bundle savedInstanceState) {
        registerPlugin(LibrusPlugin.class);
        super.onCreate(savedInstanceState);
    }
}
