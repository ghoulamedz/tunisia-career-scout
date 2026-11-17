package com.tunisiacareerscout.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.PowerManager
import android.util.Log

/**
 * Broadcast receiver to handle battery optimization and Doze mode changes.
 * Ensures the foreground service continues to run even in power-saving modes.
 */
class BatteryOptimizationReceiver : BroadcastReceiver() {

    companion object {
        private const val TAG = "BatteryOptReceiver"
    }

    override fun onReceive(context: Context, intent: Intent) {
        val action = intent.action
        Log.d(TAG, "Received battery/doze action: $action")
        
        when (action) {
            PowerManager.ACTION_POWER_SAVE_MODE_CHANGED -> {
                val powerManager = context.getSystemService(Context.POWER_SERVICE) as PowerManager
                val isPowerSaveMode = powerManager.isPowerSaveMode
                Log.d(TAG, "Power save mode changed: $isPowerSaveMode")
                
                if (!isPowerSaveMode) {
                    // Power save mode disabled - ensure service is running
                    ScanForegroundService.Companion.ensureStarted(context)
                }
            }
            
            "android.os.action.DEVICE_IDLE_MODE_CHANGED" -> {
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                    val powerManager = context.getSystemService(Context.POWER_SERVICE) as PowerManager
                    val isDeviceIdle = powerManager.isDeviceIdleMode
                    Log.d(TAG, "Device idle mode changed: $isDeviceIdle")
                    
                    if (!isDeviceIdle) {
                        // Device exited idle mode - ensure service is running
                        ScanForegroundService.Companion.ensureStarted(context)
                    }
                }
            }
        }
    }
}