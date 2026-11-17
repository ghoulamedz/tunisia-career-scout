package com.tunisiacareerscout.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.util.Log
import androidx.annotation.RequiresApi
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import com.tunisiacareerscout.app.R
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import org.json.JSONObject

/**
 * Foreground service for background job scanning on Android.
 * 
 * This service runs continuously in the background to perform periodic
 * scans of job sources. It uses a foreground service with type dataSync
 * (Android 14+) to ensure it's not killed by the system.
 * 
 * The service communicates with the Rust scheduler via Tauri events/JNI
 * to trigger scans and report results.
 */
class ScanForegroundService : Service() {

    companion object {
        private const val TAG = "ScanForegroundService"
        private const val CHANNEL_ID = "scan_channel"
        private const val NOTIFICATION_ID = 1001
        private const val ACTION_START_SCAN = "com.tunisiacareerscout.app.START_SCAN"
        private const val ACTION_STOP_SCAN = "com.tunisiacareerscout.app.STOP_SCAN"
        private const val ACTION_SCHEDULE_SCAN = "com.tunisiacareerscout.app.SCHEDULE_SCAN"
        private const val ACTION_UPDATE_NOTIFICATION = "com.tunisiacareerscout.app.UPDATE_NOTIFICATION"
        
        // Extra keys for intent communication
        private const val EXTRA_SOURCE_ID = "source_id"
        private const val EXTRA_CRON_EXPRESSION = "cron_expression"
        private const val EXTRA_SCAN_STATUS = "scan_status"
        private const val EXTRA_SCAN_MESSAGE = "scan_message"
        
        @Volatile
        private var instance: ScanForegroundService? = null
        
        private var coroutineScope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
        private var scanJob: Job? = null
        private var wakeLock: android.os.PowerManager.WakeLock? = null

        fun ensureStarted(context: Context) {
            if (instance == null) {
                val intent = Intent(context, ScanForegroundService::class.java)
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    context.startForegroundService(intent)
                } else {
                    context.startService(intent)
                }
            }
        }

        fun ensureStopped(context: Context) {
            val intent = Intent(context, ScanForegroundService::class.java)
            context.stopService(intent)
        }

        fun getInstance(): ScanForegroundService? = instance

        fun triggerScan(sourceId: String) {
            instance?.triggerScanInternal(sourceId)
        }

        fun scheduleScan(sourceId: String, cronExpression: String) {
            instance?.scheduleScanInternal(sourceId, cronExpression)
        }

        fun updateNotification(status: String, message: String) {
            instance?.updateNotificationInternal(status, message)
        }
    }

    private lateinit var notificationManager: NotificationManager
    private lateinit var powerManager: PowerManager
    private var currentScanSource: String? = null
    private var isScanning = false

    override fun onCreate() {
        super.onCreate()
        Log.d(TAG, "Service created")
        instance = this
        
        notificationManager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        powerManager = getSystemService(Context.POWER_SERVICE) as PowerManager
        
        createNotificationChannel()
        acquireWakeLock()
    }

    @RequiresApi(Build.VERSION_CODES.O)
    private fun createNotificationChannel() {
        val channel = NotificationChannel(
            CHANNEL_ID,
            getString(R.string.notification_channel_name),
            NotificationManager.IMPORTANCE_LOW
        ).apply {
            description = getString(R.string.notification_channel_description)
            enableVibration(false)
            setShowBadge(false)
            lockscreenVisibility = Notification.VISIBILITY_SECRET
        }
        notificationManager.createNotificationChannel(channel)
    }

    private fun acquireWakeLock() {
        wakeLock = powerManager.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "TunisiaCareerScout::ScanWakeLock"
        ).apply {
            isReferenceCounted = false
            acquire()
        }
    }

    private fun releaseWakeLock() {
        wakeLock?.let {
            if (it.isHeld) {
                it.release()
            }
            wakeLock = null
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action
        Log.d(TAG, "onStartCommand: action=$action")
        
        when (action) {
            ACTION_START_SCAN -> {
                val sourceId = intent.getStringExtra(EXTRA_SOURCE_ID)
                sourceId?.let { triggerScanInternal(it) }
            }
            ACTION_STOP_SCAN -> stopScanInternal()
            ACTION_SCHEDULE_SCAN -> {
                val sourceId = intent.getStringExtra(EXTRA_SOURCE_ID)
                val cron = intent.getStringExtra(EXTRA_CRON_EXPRESSION)
                sourceId?.let { scheduleScanInternal(it, cron ?: "") }
            }
            ACTION_UPDATE_NOTIFICATION -> {
                val status = intent.getStringExtra(EXTRA_SCAN_STATUS) ?: "idle"
                val message = intent.getStringExtra(EXTRA_SCAN_MESSAGE) ?: ""
                updateNotificationInternal(status, message)
            }
            null -> {
                // Service started without explicit action - start foreground
                startForegroundService()
            }
        }
        
        return START_STICKY
    }

    private fun startForegroundService() {
        val notification = buildNotification(
            getString(R.string.notification_title),
            getString(R.string.notification_idle),
            false
        )
        
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
        
        // Notify Rust side that service is ready
        notifyServiceReady()
    }

    private fun buildNotification(title: String, text: String, isScanning: Boolean): Notification {
        val intent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
        }
        val pendingIntent = PendingIntent.getActivity(
            this, 0, intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle(title)
            .setContentText(text)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setVisibility(NotificationCompat.VISIBILITY_SECRET)
            .setProgress(0, 0, isScanning)
            .build()
    }

    private fun triggerScanInternal(sourceId: String) {
        if (isScanning) {
            Log.w(TAG, "Scan already in progress, ignoring request for $sourceId")
            return
        }
        
        isScanning = true
        currentScanSource = sourceId
        
        updateNotificationInternal("scanning", getString(R.string.notification_scanning, sourceId))
        
        // Notify Rust scheduler to start scan
        notifyScanStarted(sourceId)
        
        // The actual scan is performed by the Rust side via JNI/Tauri
        // This service just manages the foreground lifecycle
        scanJob = coroutineScope.launch {
            // Simulate scan monitoring - in reality, Rust handles the scan
            // We just keep the service alive and update notification
            while (isScanning) {
                try {
                    kotlinx.coroutines.delay(5000)
                    // Check if scan is still running via JNI callback
                    // For now, we rely on Rust to call stopScanInternal
                } catch (e: Exception) {
                    Log.e(TAG, "Error in scan monitoring", e)
                }
            }
        }
    }

    private fun stopScanInternal() {
        if (!isScanning) return
        
        isScanning = false
        val sourceId = currentScanSource
        currentScanSource = null
        
        scanJob?.cancel()
        scanJob = null
        
        updateNotificationInternal("idle", getString(R.string.notification_idle))
        
        if (sourceId != null) {
            notifyScanCompleted(sourceId)
        }
    }

    private fun scheduleScanInternal(sourceId: String, cronExpression: String) {
        Log.d(TAG, "Scheduling scan for $sourceId with cron: $cronExpression")
        // Forward to Rust scheduler via JNI
        notifyScheduleRequested(sourceId, cronExpression)
    }

    private fun updateNotificationInternal(status: String, message: String) {
        val title = when (status) {
            "scanning" -> getString(R.string.notification_title)
            "completed" -> "Scan Completed"
            "error" -> "Scan Error"
            else -> getString(R.string.notification_title)
        }
        
        val notification = buildNotification(title, message, status == "scanning")
        notificationManager.notify(NOTIFICATION_ID, notification)
    }

    // JNI callbacks to communicate with Rust
    external fun notifyServiceReady()
    external fun notifyScanStarted(sourceId: String)
    external fun notifyScanCompleted(sourceId: String)
    external fun notifyScanFailed(sourceId: String, error: String)
    external fun notifyScheduleRequested(sourceId: String, cronExpression: String)

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        Log.d(TAG, "Service destroyed")
        isScanning = false
        currentScanSource = null
        scanJob?.cancel()
        releaseWakeLock()
        coroutineScope.cancel()
        instance = null
        super.onDestroy()
    }
}