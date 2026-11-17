package pl.libruslite.mobile;

import android.content.Context;
import androidx.annotation.NonNull;
import androidx.work.Worker;
import androidx.work.WorkerParameters;

/** All network work still runs on LibrusPlugin's single account/session executor. */
public final class SchoolRefreshWorker extends Worker {
    public SchoolRefreshWorker(@NonNull Context context, @NonNull WorkerParameters parameters) { super(context, parameters); }
    @NonNull @Override public Result doWork() {
        try {
            boolean complete = LibrusPlugin.backgroundSync(getApplicationContext(), this::isStopped).get();
            return complete ? Result.success() : Result.retry();
        } catch (Exception ignored) { return isStopped() ? Result.success() : Result.retry(); }
    }
}
