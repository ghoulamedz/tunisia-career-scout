package com.tunisiacareerscout.app

import android.os.Bundle
import com.tauri.TauriActivity

class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // Initialize foreground service on app launch
        ScanForegroundService.Companion.ensureStarted(this)
    }
}