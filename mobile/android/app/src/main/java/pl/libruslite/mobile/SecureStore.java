package pl.libruslite.mobile;

import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.AtomicFile;
import java.io.File;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;
import java.security.KeyStore;
import java.security.MessageDigest;
import java.util.Arrays;
import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

/** AES-GCM files authenticated to their account-derived filename; key never leaves Android Keystore. */
final class SecureStore {
    private final File root;
    private final SecretKey key;
    SecureStore(android.content.Context context) throws Exception {
        root = new File(context.getNoBackupFilesDir(), "private-v1");
        if (!root.exists() && !root.mkdirs()) throw new java.io.IOException();
        KeyStore keys = KeyStore.getInstance("AndroidKeyStore");
        keys.load(null);
        String alias = "librus-lite-mobile-v1";
        if (!keys.containsAlias(alias)) {
            KeyGenerator generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES,"AndroidKeyStore");
            generator.init(new KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256).setRandomizedEncryptionRequired(true).build());
            generator.generateKey();
        }
        key = (SecretKey) keys.getKey(alias,null);
    }
    private String filename(String name) throws Exception {
        byte[] hash = MessageDigest.getInstance("SHA-256").digest(name.getBytes(StandardCharsets.UTF_8));
        StringBuilder out = new StringBuilder();
        for(byte b:hash) out.append(String.format("%02x",b));
        return out.toString();
    }
    String read(String name) throws Exception {
        String id = filename(name);
        AtomicFile file = new AtomicFile(new File(root,id));
        if (!file.getBaseFile().exists()) return null;
        byte[] encrypted = file.readFully();
        if (encrypted.length < 29 || encrypted[0]!=1) throw new java.io.IOException();
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.DECRYPT_MODE,key,new GCMParameterSpec(128,Arrays.copyOfRange(encrypted,1,13)));
        cipher.updateAAD(id.getBytes(StandardCharsets.UTF_8));
        byte[] plaintext = cipher.doFinal(encrypted,13,encrypted.length-13);
        try { return new String(plaintext,StandardCharsets.UTF_8); } finally { Arrays.fill(plaintext,(byte)0); }
    }
    void write(String name,String value) throws Exception {
        String id = filename(name);
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.ENCRYPT_MODE,key);
        cipher.updateAAD(id.getBytes(StandardCharsets.UTF_8));
        byte[] plaintext=value.getBytes(StandardCharsets.UTF_8);
        byte[] encrypted;
        try { encrypted=cipher.doFinal(plaintext); } finally { Arrays.fill(plaintext,(byte)0); }
        AtomicFile file = new AtomicFile(new File(root,id));
        FileOutputStream stream=null;
        try {
            stream=file.startWrite();stream.write(1);stream.write(cipher.getIV());stream.write(encrypted);file.finishWrite(stream);
        } catch (Exception e) { if(stream!=null)file.failWrite(stream);throw e; }
    }
    void delete(String name) throws Exception {
        File target = new File(root,filename(name));
        new AtomicFile(target).delete();
        if (target.exists()) throw new java.io.IOException();
    }
}
