package pl.libruslite.mobile;

import org.junit.Test;
import static org.junit.Assert.*;

public class UpdatePolicyTest {
    @Test public void comparesNumericStableVersionsAndBlocksDowngrades() {
        assertTrue(UpdatePolicy.newer("v0.1.10", "0.1.9"));
        assertTrue(UpdatePolicy.newer("1.0.0", "0.99.99"));
        assertFalse(UpdatePolicy.newer("v0.1.2", "0.1.2"));
        assertFalse(UpdatePolicy.newer("0.1.1", "0.1.2"));
        for (String invalid : new String[]{"v0.1.3-beta", "0.1", "01.2.3", "1.2.3.4", "1.2.99999999999999999999"}) {
            try { UpdatePolicy.version(invalid); fail(invalid); }
            catch (IllegalArgumentException expected) { /* rejected */ }
        }
    }

    @Test public void selectsOnlyMatchingArm64ApksAndFullSha256Digests() {
        assertTrue(UpdatePolicy.assetName("klaso-mobile-0.1.3-arm64-debug.apk", "0.1.3"));
        assertTrue(UpdatePolicy.assetName("klaso-mobile-0.1.3-arm64-release.apk", "0.1.3"));
        assertFalse(UpdatePolicy.assetName("klaso-mobile-0.1.2-arm64-debug.apk", "0.1.3"));
        assertFalse(UpdatePolicy.assetName("klaso-mobile-0.1.3-x86.apk", "0.1.3"));
        assertTrue(UpdatePolicy.digest("sha256:" + new String(new char[64]).replace('\0', 'a')));
        assertFalse(UpdatePolicy.digest("sha256:abc"));
        assertFalse(UpdatePolicy.digest("null"));
    }

    @Test public void rejectsForeignAssetsAndInsecureRedirects() {
        String name = "klaso-mobile-0.1.3-arm64-debug.apk";
        String url = "https://github.com/chillyfrog123/klaso-mobile/releases/download/v0.1.3/" + name;
        assertTrue(UpdatePolicy.releaseUrl(url, "v0.1.3", name));
        assertFalse(UpdatePolicy.releaseUrl(url.replace("klaso-mobile/releases", "other/releases"), "v0.1.3", name));
        assertFalse(UpdatePolicy.releaseUrl(url + "?redirect=bad", "v0.1.3", name));
        assertFalse(UpdatePolicy.releaseUrl(url.replace("https:", "http:"), "v0.1.3", name));
        assertTrue(UpdatePolicy.transportUrl("https://release-assets.githubusercontent.com/github-production-release-asset/test?token=x"));
        for (String invalid : new String[]{"https://evil.example/update.apk", "http://github.com/a", "https://github.com.evil.example/a", "https://github.com@evil.example/a", "https://api.github.com/repos/other/repo/releases/latest", "https://objects.githubusercontent.com:8443/a"}) {
            assertFalse(invalid, UpdatePolicy.transportUrl(invalid));
        }
    }
}
