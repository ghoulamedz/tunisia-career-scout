package com.tunisiacareerscout.app

import android.app.Application
import android.content.Context
import android.util.Log
import com.tunisiacareerscout.app.plugin.ForegroundServicePlugin
import com.tauri.TauriApplication

/**
 * Custom Application class for JNI initialization.
 * This initializes the JNI bridge for the foreground service plugin.
 */
class TunisiaCareerScoutApplication : TauriApplication() {

    companion object {
        private const val TAG = "TunisiaCareerScoutApp"
        
        // Load native library
        init {
            System.loadLibrary("tunisia_career_scout")
        }
    }

    override fun onCreate() {
        super.onCreate()
        Log.d(TAG, "Application created")
        
        // Initialize JNI bridge for foreground service plugin
        initJniBridge()
    }

    /**
     * Initialize JNI bridge for the foreground service plugin.
     * Called from the Rust side via JNI_OnLoad or explicitly.
     */
    external fun initJniBridge()

    override fun attachBaseContext(base: Context) {
        super.attachBaseContext(base)
        // MultiDex support if needed
    }
}