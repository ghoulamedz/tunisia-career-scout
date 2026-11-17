package com.tunisiacareerscout.app.plugin

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings
import android.util.Log
import com.tauri.Plugin
import com.tauri.PluginHandle
import com.tauri.JavaCallback
import com.tauri.TauriPlugin
import com.tunisiacareerscout.app.ScanForegroundService
import org.json.JSONObject

/**
 * Tauri plugin for Android Foreground Service management.
 * Provides JavaScript/Rust interface to control the background scanning service.
 */
class ForegroundServicePlugin : TauriPlugin() {

    companion object {
        private const val TAG = "ForegroundServicePlugin"
        private const val CMD_START_SERVICE = "startService"
        private const val CMD_STOP_SERVICE = "stopService"
        private const val CMD_TRIGGER_SCAN = "triggerScan"
        private const val CMD_SCHEDULE_SCAN = "scheduleScan"
        private const val CMD_UPDATE_NOTIFICATION = "updateNotification"
        private const val CMD_REQUEST_BATTERY_EXEMPTION = "requestBatteryExemption"
        private const val CMD_CHECK_BATTERY_EXEMPTION = "checkBatteryExemption"
        private const val CMD_IS_SERVICE_RUNNING = "isServiceRunning"
    }

    private var pluginHandle: PluginHandle? = null
    private var activity: Activity? = null

    override fun load(pluginHandle: PluginHandle) {
        this.pluginHandle = pluginHandle
        this.activity = pluginHandle.getActivity()
        Log.d(TAG, "ForegroundServicePlugin loaded")
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        // Handle any intents if needed
    }

    @Plugin(command = CMD_START_SERVICE)
    fun startService(invoke: JavaCallback) {
        val context = activity?.applicationContext
        context?.let {
            ScanForegroundService.Companion.ensureStarted(it)
            invoke.resolve(JSONObject().put("success", true))
        } ?: invoke.reject("Activity context not available")
    }

    @Plugin(command = CMD_STOP_SERVICE)
    fun stopService(invoke: JavaCallback) {
        val context = activity?.applicationContext
        context?.let {
            ScanForegroundService.Companion.ensureStopped(it)
            invoke.resolve(JSONObject().put("success", true))
        } ?: invoke.reject("Activity context not available")
    }

    @Plugin(command = CMD_TRIGGER_SCAN)
    fun triggerScan(invoke: JavaCallback, sourceId: String) {
        ScanForegroundService.Companion.triggerScan(sourceId)
        invoke.resolve(JSONObject().put("success", true).put("sourceId", sourceId))
    }

    @Plugin(command = CMD_SCHEDULE_SCAN)
    fun scheduleScan(invoke: JavaCallback, sourceId: String, cronExpression: String) {
        ScanForegroundService.Companion.scheduleScan(sourceId, cronExpression)
        invoke.resolve(JSONObject().put("success", true).put("sourceId", sourceId))
    }

    @Plugin(command = CMD_UPDATE_NOTIFICATION)
    fun updateNotification(invoke: JavaCallback, status: String, message: String) {
        ScanForegroundService.Companion.updateNotification(status, message)
        invoke.resolve(JSONObject().put("success", true))
    }

    @Plugin(command = CMD_REQUEST_BATTERY_EXEMPTION)
    fun requestBatteryExemption(invoke: JavaCallback) {
        val context = activity?.applicationContext
        context?.let {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                val powerManager = it.getSystemService(Context.POWER_SERVICE) as PowerManager
                val packageName = it.packageName
                
                if (!powerManager.isIgnoringBatteryOptimizations(packageName)) {
                    val intent = Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS)
                    intent.data = Uri.parse("package:$packageName")
                    intent.flags = Intent.FLAG_ACTIVITY_NEW_TASK
                    
                    if (intent.resolveActivity(it.packageManager) != null) {
                        activity?.startActivity(intent)
                        invoke.resolve(JSONObject().put("success", true).put("requested", true))
                    } else {
                        invoke.resolve(JSONObject().put("success", true).put("requested", false).put("reason", "No activity to handle request"))
                    }
                } else {
                    invoke.resolve(JSONObject().put("success", true).put("requested", false).put("reason", "Already exempt"))
                }
            } else {
                invoke.resolve(JSONObject().put("success", true).put("requested", false).put("reason", "Not required on this Android version"))
            }
        } ?: invoke.reject("Activity context not available")
    }

    @Plugin(command = CMD_CHECK_BATTERY_EXEMPTION)
    fun checkBatteryExemption(invoke: JavaCallback) {
        val context = activity?.applicationContext
        context?.let {
            var isExempt = false
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                val powerManager = it.getSystemService(Context.POWER_SERVICE) as PowerManager
                isExempt = powerManager.isIgnoringBatteryOptimizations(it.packageName)
            } else {
                // Pre-Marshmallow doesn't have battery optimization
                isExempt = true
            }
            invoke.resolve(JSONObject().put("success", true).put("isExempt", isExempt))
        } ?: invoke.reject("Activity context not available")
    }

    @Plugin(command = CMD_IS_SERVICE_RUNNING)
    fun isServiceRunning(invoke: JavaCallback) {
        val isRunning = ScanForegroundService.Companion.getInstance() != null
        invoke.resolve(JSONObject().put("success", true).put("isRunning", isRunning))
    }
}