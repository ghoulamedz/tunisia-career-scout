//! Android Foreground Service Plugin
//!
//! Tauri plugin for managing the Android foreground service for background scanning.

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

#[cfg(target_os = "android")]
mod android {
    use super::*;
    use jni::objects::{JClass, JObject, JString, JValue};
    use jni::sys::{jboolean, jint, jlong};
    use jni::JNIEnv;
    use std::sync::OnceLock;
    use tauri::plugin::PluginApi;

    static JAVA_VM: OnceLock<jni::JavaVM> = OnceLock::new();

    pub fn init_jni(vm: jni::JavaVM) {
        JAVA_VM.set(vm).ok();
    }

    fn with_env<F, R>(f: F) -> Result<R, String>
    where
        F: FnOnce(&mut JNIEnv) -> Result<R, String>,
    {
        let vm = JAVA_VM.get().ok_or("JavaVM not initialized")?;
        let mut env = vm.attach_current_thread_permanently().map_err(|e| e.to_string())?;
        f(&mut env)
    }

    fn call_static_method(method: &str, args: &[JValue]) -> Result<(), String> {
        with_env(|env| {
            let class = env
                .find_class("com/tunisiacareerscout/app/plugin/ForegroundServicePlugin")
                .map_err(|e| e.to_string())?;
            env.call_static_method_void(class, method, args)
                .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    fn call_static_method_bool(method: &str, args: &[JValue]) -> Result<bool, String> {
        with_env(|env| {
            let class = env
                .find_class("com/tunisiacareerscout/app/plugin/ForegroundServicePlugin")
                .map_err(|e| e.to_string())?;
            let result = env
                .call_static_method(class, method, args, "Z")
                .map_err(|e| e.to_string())?;
            Ok(result.z().map_err(|e| e.to_string())?)
        })
    }

    pub fn start_service() -> Result<(), String> {
        call_static_method("startService", &[])
    }

    pub fn stop_service() -> Result<(), String> {
        call_static_method("stopService", &[])
    }

    pub fn trigger_scan(source_id: &str) -> Result<(), String> {
        with_env(|env| {
            let class = env
                .find_class("com/tunisiacareerscout/app/plugin/ForegroundServicePlugin")
                .map_err(|e| e.to_string())?;
            let source_id_str = env.new_string(source_id).map_err(|e| e.to_string())?;
            env.call_static_method_void(
                class,
                "triggerScan",
                &[JValue::Object(&source_id_str)],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    pub fn schedule_scan(source_id: &str, cron_expression: &str) -> Result<(), String> {
        with_env(|env| {
            let class = env
                .find_class("com/tunisiacareerscout/app/plugin/ForegroundServicePlugin")
                .map_err(|e| e.to_string())?;
            let source_id_str = env.new_string(source_id).map_err(|e| e.to_string())?;
            let cron_str = env.new_string(cron_expression).map_err(|e| e.to_string())?;
            env.call_static_method_void(
                class,
                "scheduleScan",
                &[JValue::Object(&source_id_str), JValue::Object(&cron_str)],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    pub fn update_notification(status: &str, message: &str) -> Result<(), String> {
        with_env(|env| {
            let class = env
                .find_class("com/tunisiacareerscout/app/plugin/ForegroundServicePlugin")
                .map_err(|e| e.to_string())?;
            let status_str = env.new_string(status).map_err(|e| e.to_string())?;
            let message_str = env.new_string(message).map_err(|e| e.to_string())?;
            env.call_static_method_void(
                class,
                "updateNotification",
                &[JValue::Object(&status_str), JValue::Object(&message_str)],
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })
    }

    pub fn request_battery_exemption() -> Result<(), String> {
        call_static_method("requestBatteryExemption", &[])
    }

    pub fn check_battery_exemption() -> Result<bool, String> {
        call_static_method_bool("checkBatteryExemption", &[])
    }

    pub fn is_service_running() -> Result<bool, String> {
        call_static_method_bool("isServiceRunning", &[])
    }
}

#[cfg(not(target_os = "android"))]
mod android {
    pub fn start_service() -> Result<(), String> {
        Ok(())
    }
    pub fn stop_service() -> Result<(), String> {
        Ok(())
    }
    pub fn trigger_scan(_source_id: &str) -> Result<(), String> {
        Ok(())
    }
    pub fn schedule_scan(_source_id: &str, _cron_expression: &str) -> Result<(), String> {
        Ok(())
    }
    pub fn update_notification(_status: &str, _message: &str) -> Result<(), String> {
        Ok(())
    }
    pub fn request_battery_exemption() -> Result<(), String> {
        Ok(())
    }
    pub fn check_battery_exemption() -> Result<bool, String> {
        Ok(true)
    }
    pub fn is_service_running() -> Result<bool, String> {
        Ok(false)
    }
    
    pub fn init_jni(_vm: jni::JavaVM) {}
}

/// Commands exposed to the frontend/Rust
#[tauri::command]
async fn start_foreground_service() -> Result<(), String> {
    android::start_service()
}

#[tauri::command]
async fn stop_foreground_service() -> Result<(), String> {
    android::stop_service()
}

#[tauri::command]
async fn trigger_background_scan(source_id: String) -> Result<(), String> {
    android::trigger_scan(&source_id)
}

#[tauri::command]
async fn schedule_background_scan(source_id: String, cron_expression: String) -> Result<(), String> {
    android::schedule_scan(&source_id, &cron_expression)
}

#[tauri::command]
async fn update_service_notification(status: String, message: String) -> Result<(), String> {
    android::update_notification(&status, &message)
}

#[tauri::command]
async fn request_battery_optimization_exemption() -> Result<(), String> {
    android::request_battery_exemption()
}

#[tauri::command]
async fn check_battery_optimization_exemption() -> Result<bool, String> {
    android::check_battery_exemption()
}

#[tauri::command]
async fn is_foreground_service_running() -> Result<bool, String> {
    android::is_service_running()
}

/// Initialize JNI for Android
#[cfg(target_os = "android")]
#[tauri::command]
async fn init_jni_bridge() -> Result<(), String> {
    // This will be called from the Android side to initialize the JNI bridge
    Ok(())
}

/// Plugin builder
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("android-foreground-service")
        .invoke_handler(tauri::generate_handler![
            start_foreground_service,
            stop_foreground_service,
            trigger_background_scan,
            schedule_background_scan,
            update_service_notification,
            request_battery_optimization_exemption,
            check_battery_optimization_exemption,
            is_foreground_service_running,
            #[cfg(target_os = "android")]
            init_jni_bridge,
        ])
        .build()
}

// Re-export for JNI initialization
#[cfg(target_os = "android")]
pub use android::init_jni;