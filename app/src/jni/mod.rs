//! JNI Bridge for Android
//!
//! Provides JNI initialization and callbacks for Android integration.

#[cfg(target_os = "android")]
mod android {
    use jni::JavaVM;
    use jni::objects::{JClass, JObject, JString};
    use jni::sys::{jint, jobject, jstring};
    use std::sync::OnceLock;
    use tracing::{info, error, debug};

    static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

    /// Initialize the JNI bridge with the Java VM
    pub fn init_jni_bridge(vm: JavaVM) {
        if JAVA_VM.set(vm).is_ok() {
            info!("JNI bridge initialized successfully");
        } else {
            error!("JNI bridge already initialized");
        }
    }

    /// Get the Java VM instance
    pub fn get_java_vm() -> Option<&'static JavaVM> {
        JAVA_VM.get()
    }

    /// JNI_OnLoad entry point - called when the native library is loaded
    #[no_mangle]
    pub extern "system" fn JNI_OnLoad(vm: JavaVM, _reserved: *mut std::ffi::c_void) -> jint {
        init_jni_bridge(vm);
        // Return JNI version
        jni::sys::JNI_VERSION_1_6
    }

    /// Initialize the foreground service plugin JNI bridge
    #[no_mangle]
    pub extern "system" fn Java_com_tunisiacareerscout_app_TunisiaCareerScoutApplication_initJniBridge(
        env: jni::JNIEnv,
        _class: jni::objects::JClass,
    ) {
        if let Ok(vm) = env.get_java_vm() {
            init_jni_bridge(vm);
            info!("Foreground service plugin JNI bridge initialized");
        } else {
            error!("Failed to get Java VM for foreground service plugin");
        }
    }

    /// JNI callback: Called when the foreground service is ready
    #[no_mangle]
    pub extern "system" fn Java_com_tunisiacareerscout_app_ScanForegroundService_notifyServiceReady(
        env: jni::JNIEnv,
        _class: JClass,
    ) {
        debug!("Foreground service ready callback received");
        // Could emit a Tauri event here to notify the frontend
        // For now, just log
    }

    /// JNI callback: Called when a scan starts
    #[no_mangle]
    pub extern "system" fn Java_com_tunisiacareerscout_app_ScanForegroundService_notifyScanStarted(
        env: jni::JNIEnv,
        _class: JClass,
        source_id: JString,
    ) {
        let source_id: String = env.get_string(&source_id).expect("Failed to get source_id string").into();
        debug!("Scan started for source: {}", source_id);
        // Could emit a Tauri event here to notify the frontend
    }

    /// JNI callback: Called when a scan completes successfully
    #[no_mangle]
    pub extern "system" fn Java_com_tunisiacareerscout_app_ScanForegroundService_notifyScanCompleted(
        env: jni::JNIEnv,
        _class: JClass,
        source_id: JString,
    ) {
        let source_id: String = env.get_string(&source_id).expect("Failed to get source_id string").into();
        debug!("Scan completed for source: {}", source_id);
        // Could emit a Tauri event here to notify the frontend
    }

    /// JNI callback: Called when a scan fails
    #[no_mangle]
    pub extern "system" fn Java_com_tunisiacareerscout_app_ScanForegroundService_notifyScanFailed(
        env: jni::JNIEnv,
        _class: JClass,
        source_id: JString,
        error: JString,
    ) {
        let source_id: String = env.get_string(&source_id).expect("Failed to get source_id string").into();
        let error: String = env.get_string(&error).expect("Failed to get error string").into();
        error!("Scan failed for source {}: {}", source_id, error);
        // Could emit a Tauri event here to notify the frontend
    }

    /// JNI callback: Called when a schedule is requested from the service
    #[no_mangle]
    pub extern "system" fn Java_com_tunisiacareerscout_app_ScanForegroundService_notifyScheduleRequested(
        env: jni::JNIEnv,
        _class: JClass,
        source_id: JString,
        cron_expression: JString,
    ) {
        let source_id: String = env.get_string(&source_id).expect("Failed to get source_id string").into();
        let cron_expression: String = env.get_string(&cron_expression).expect("Failed to get cron_expression string").into();
        debug!("Schedule requested for source {} with cron: {}", source_id, cron_expression);
        // Could emit a Tauri event here to notify the frontend
    }
}

#[cfg(not(target_os = "android"))]
mod android {
    pub fn init_jni_bridge(_vm: ()) {}
    pub fn get_java_vm() -> Option<()> { None }
}

#[cfg(target_os = "android")]
pub use android::{init_jni_bridge, get_java_vm};

#[cfg(not(target_os = "android"))]
pub use android::{init_jni_bridge, get_java_vm};