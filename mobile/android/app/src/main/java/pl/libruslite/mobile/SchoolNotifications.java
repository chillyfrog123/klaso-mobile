package pl.libruslite.mobile;

import android.Manifest;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.os.Build;
import androidx.core.app.NotificationCompat;
import androidx.core.app.NotificationManagerCompat;
import androidx.core.content.ContextCompat;
import androidx.work.Constraints;
import androidx.work.ExistingPeriodicWorkPolicy;
import androidx.work.NetworkType;
import androidx.work.PeriodicWorkRequest;
import androidx.work.WorkManager;
import java.util.concurrent.TimeUnit;

/** Non-sensitive preferences and generic, private lock-screen notifications. */
final class SchoolNotifications {
    private static final String WORK = "school-monitor-v1";
    static final String EXTRA_SECTION = "school_section", EXTRA_OWNER = "school_owner";
    static SharedPreferences preferences(Context context) {
        return context.getSharedPreferences("school-monitor-v1", Context.MODE_PRIVATE);
    }
    static boolean enabled(Context context) { return preferences(context).getBoolean("enabled", true); }
    static boolean allowed(Context context) {
        return (Build.VERSION.SDK_INT < 33 || ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED)
            && NotificationManagerCompat.from(context).areNotificationsEnabled();
    }
    static void schedule(Context context, boolean active) {
        WorkManager manager = WorkManager.getInstance(context);
        if (!active) { manager.cancelUniqueWork(WORK); return; }
        PeriodicWorkRequest request = new PeriodicWorkRequest.Builder(SchoolRefreshWorker.class, 15, TimeUnit.MINUTES)
            .setConstraints(new Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
            .setInitialDelay(15, TimeUnit.MINUTES).build();
        manager.enqueueUniquePeriodicWork(WORK, ExistingPeriodicWorkPolicy.KEEP, request);
    }
    static void clear(Context context) {
        for (int id : new int[]{3101, 3102, 3103}) NotificationManagerCompat.from(context).cancel(id);
    }
    static void show(Context context, String owner, SchoolChanges.Delta delta) {
        if (!enabled(context) || !allowed(context)) return;
        boolean polish = preferences(context).getString("language", "en").equals("pl");
        send(context, owner, "Grades", 3101, delta.grades, polish ? "Nowe oceny" : "New grades", polish);
        send(context, owner, "Messages", 3102, delta.messages, polish ? "Nowe wiadomości" : "New messages", polish);
        int changes = delta.substitutions + delta.cancellations;
        if (changes > 0) {
            String title = polish ? "Zmiany w planie lekcji" : "Timetable changes";
            String body = polish ? "Zastępstwa: " + delta.substitutions + ". Odwołania: " + delta.cancellations + "."
                : "Substitutions: " + delta.substitutions + ". Cancellations: " + delta.cancellations + ".";
            post(context, owner, "Timetable", 3103, title, body, polish);
        }
    }
    private static void send(Context context, String owner, String section, int id, int count, String title, boolean polish) {
        if (count > 0) post(context, owner, section, id, title,
            (polish ? "Liczba nowych wpisów: " : "New entries: ") + count, polish);
    }
    private static void post(Context context, String owner, String section, int id, String title, String body, boolean polish) {
        String channelId = "school-" + section.toLowerCase(java.util.Locale.ROOT);
        if (Build.VERSION.SDK_INT >= 26) {
            NotificationChannel channel = new NotificationChannel(channelId, title, NotificationManager.IMPORTANCE_DEFAULT);
            channel.setLockscreenVisibility(android.app.Notification.VISIBILITY_PRIVATE);
            context.getSystemService(NotificationManager.class).createNotificationChannel(channel);
        }
        Intent intent = new Intent(context, MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP | Intent.FLAG_ACTIVITY_SINGLE_TOP)
            .putExtra(EXTRA_SECTION, section).putExtra(EXTRA_OWNER, owner);
        PendingIntent content = PendingIntent.getActivity(context, id, intent, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
        android.app.Notification publicVersion = new NotificationCompat.Builder(context, channelId)
            .setSmallIcon(R.drawable.ic_school_notification).setContentTitle("Klaso Mobile")
            .setContentText(polish ? "Nowe informacje ze szkoły" : "New school information").build();
        android.app.Notification notification = new NotificationCompat.Builder(context, channelId)
            .setSmallIcon(R.drawable.ic_school_notification).setContentTitle(title).setContentText(body)
            .setContentIntent(content).setAutoCancel(true).setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
            .setPublicVersion(publicVersion).setCategory(NotificationCompat.CATEGORY_SOCIAL).build();
        try { NotificationManagerCompat.from(context).notify(id, notification); }
        catch (SecurityException ignored) { /* Permission may be revoked between the check and post. */ }
    }
}
