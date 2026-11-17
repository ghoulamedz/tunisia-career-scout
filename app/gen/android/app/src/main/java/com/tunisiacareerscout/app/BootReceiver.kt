package com.tunisiacareerscout.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

/**
 * Broadcast receiver to restart the foreground service on device boot.
 * This ensures background scanning continues after device restart.
 */
class BootReceiver : BroadcastReceiver() {

    companion object {
        private const val TAG = "BootReceiver"
    }

    override fun onReceive(context: Context, intent: Intent) {
        val action = intent.action
        Log.d(TAG, "Received boot action: $action")
        
        when (action) {
            Intent.ACTION_BOOT_COMPLETED,
            Intent.ACTION_QUICKBOOT_POWERON,
            "com.htc.intent.action.QUICKBOOT_POWERON" -> {
                // Start the foreground service
                ScanForegroundService.Companion.ensureStarted(context)
                
                // Also request battery optimization exemption if needed
                requestBatteryOptimizationExemption(context)
            }
        }
    }

    private fun requestBatteryOptimizationExemption(context: Context) {
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.M) {
            val powerManager = context.getSystemService(Context.POWER_SERVICE) as android.os.PowerManager
            val packageName = context.packageName
            
            if (!powerManager.isIgnoringBatteryOptimizations(packageName)) {
                Log.d(TAG, "App is not exempt from battery optimizations")
                // Note: We cannot automatically request exemption - user must grant it
                // The app should prompt the user to grant this permission
            }
        }
    }
}