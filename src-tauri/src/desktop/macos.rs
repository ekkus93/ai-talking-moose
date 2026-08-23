use crate::desktop::observation::{
    ActiveApplicationObservation, BatteryObservation, IdleObservation, ObserverErrorCode,
    ObserverResult, PowerEvent,
};
use tokio::sync::mpsc::UnboundedSender;

fn normalize_idle_duration(seconds: f64) -> Result<IdleObservation, ObserverErrorCode> {
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(ObserverErrorCode::InvalidValue);
    }
    Ok(IdleObservation {
        seconds: seconds.floor().min(u64::MAX as f64) as u64,
    })
}

fn normalize_battery_state(
    current: i32,
    maximum: i32,
    is_charging: bool,
) -> Result<BatteryObservation, ObserverErrorCode> {
    if current < 0 || maximum <= 0 {
        return Err(ObserverErrorCode::InvalidValue);
    }
    let percentage = (f64::from(current) / f64::from(maximum) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8;
    Ok(BatteryObservation {
        level_percent: percentage,
        is_charging,
    })
}

pub struct SystemDesktopMonitor;

impl SystemDesktopMonitor {
    pub fn get_active_application(allowed: bool) -> ObserverResult<ActiveApplicationObservation> {
        if !allowed {
            return ObserverResult::Denied;
        }
        platform::active_application()
    }

    pub fn get_battery_state() -> ObserverResult<BatteryObservation> {
        platform::battery_state()
    }

    pub fn get_idle_time() -> ObserverResult<IdleObservation> {
        platform::idle_time()
    }

    pub fn get_window_title(allowed: bool) -> ObserverResult<String> {
        if !allowed {
            ObserverResult::Denied
        } else {
            // V1 intentionally does not request Accessibility/Screen Recording-style
            // access to inspect another application's window title.
            ObserverResult::Unsupported
        }
    }

    pub fn start_power_events(
        sender: UnboundedSender<PowerEvent>,
    ) -> ObserverResult<SystemPowerObserver> {
        platform::start_power_events(sender)
    }
}

pub struct SystemPowerObserver {
    #[cfg(target_os = "macos")]
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    #[cfg(target_os = "macos")]
    thread: Option<std::thread::JoinHandle<()>>,
}

impl SystemPowerObserver {
    pub fn stop(self) {
        #[cfg(target_os = "macos")]
        {
            let mut observer = self;
            observer.stop_inner();
        }
    }

    fn stop_inner(&mut self) {
        #[cfg(target_os = "macos")]
        {
            use std::sync::atomic::Ordering;
            self.running.store(false, Ordering::SeqCst);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

impl Drop for SystemPowerObserver {
    fn drop(&mut self) {
        self.stop_inner();
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::*;

    pub(super) fn active_application() -> ObserverResult<ActiveApplicationObservation> {
        ObserverResult::Unsupported
    }

    pub(super) fn battery_state() -> ObserverResult<BatteryObservation> {
        ObserverResult::Unsupported
    }

    pub(super) fn idle_time() -> ObserverResult<IdleObservation> {
        ObserverResult::Unsupported
    }

    pub(super) fn start_power_events(
        _sender: UnboundedSender<PowerEvent>,
    ) -> ObserverResult<SystemPowerObserver> {
        ObserverResult::Unsupported
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::ffi::{c_char, c_void, CStr, CString};
    use std::ptr;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    type CFTypeRef = *const c_void;
    type CFArrayRef = *const c_void;
    type CFDictionaryRef = *const c_void;
    type CFRunLoopRef = *mut c_void;
    type CFRunLoopSourceRef = *const c_void;
    type CFStringRef = *const c_void;
    type IONotificationPortRef = *mut c_void;
    type IoObject = u32;
    type IoConnect = u32;

    const CG_EVENT_SOURCE_COMBINED_SESSION_STATE: i32 = 0;
    const CG_ANY_INPUT_EVENT_TYPE: u32 = u32::MAX;
    const CF_NUMBER_SINT32_TYPE: i32 = 3;
    const CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
    const IO_MESSAGE_CAN_SYSTEM_SLEEP: u32 = 0xe000_0270;
    const IO_MESSAGE_SYSTEM_WILL_SLEEP: u32 = 0xe000_0280;
    const IO_MESSAGE_SYSTEM_HAS_POWERED_ON: u32 = 0xe000_0300;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        #[link_name = "CGEventSourceSecondsSinceLastEventType"]
        fn cg_event_source_seconds_since_last_event_type(state_id: i32, event_type: u32) -> f64;
    }

    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

    #[link(name = "objc")]
    extern "C" {
        #[link_name = "objc_getClass"]
        fn objc_get_class(name: *const c_char) -> *mut c_void;
        #[link_name = "sel_registerName"]
        fn sel_register_name(name: *const c_char) -> *mut c_void;
        #[link_name = "objc_msgSend"]
        fn objc_msg_send(receiver: *mut c_void, selector: *mut c_void) -> *mut c_void;
        #[link_name = "objc_autoreleasePoolPush"]
        fn objc_autorelease_pool_push() -> *mut c_void;
        #[link_name = "objc_autoreleasePoolPop"]
        fn objc_autorelease_pool_pop(context: *mut c_void);
    }

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        #[link_name = "IOPSCopyPowerSourcesInfo"]
        fn iops_copy_power_sources_info() -> CFTypeRef;
        #[link_name = "IOPSCopyPowerSourcesList"]
        fn iops_copy_power_sources_list(blob: CFTypeRef) -> CFArrayRef;
        #[link_name = "IOPSGetPowerSourceDescription"]
        fn iops_get_power_source_description(
            blob: CFTypeRef,
            power_source: CFTypeRef,
        ) -> CFDictionaryRef;

        #[link_name = "IORegisterForSystemPower"]
        fn io_register_for_system_power(
            refcon: *mut c_void,
            notification_port: *mut IONotificationPortRef,
            callback: unsafe extern "C" fn(*mut c_void, IoObject, u32, *mut c_void),
            notifier: *mut IoObject,
        ) -> IoConnect;
        #[link_name = "IONotificationPortGetRunLoopSource"]
        fn io_notification_port_get_run_loop_source(
            notification_port: IONotificationPortRef,
        ) -> CFRunLoopSourceRef;
        #[link_name = "IONotificationPortDestroy"]
        fn io_notification_port_destroy(notification_port: IONotificationPortRef);
        #[link_name = "IODeregisterForSystemPower"]
        fn io_deregister_for_system_power(notifier: *mut IoObject) -> i32;
        #[link_name = "IOAllowPowerChange"]
        fn io_allow_power_change(root_port: IoConnect, notification_id: isize) -> i32;
        #[link_name = "IOServiceClose"]
        fn io_service_close(connect: IoConnect) -> i32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        #[link_name = "CFRelease"]
        fn cf_release(value: CFTypeRef);
        #[link_name = "CFStringCreateWithCString"]
        fn cf_string_create_with_c_string(
            allocator: CFTypeRef,
            value: *const c_char,
            encoding: u32,
        ) -> CFStringRef;
        #[link_name = "CFArrayGetCount"]
        fn cf_array_get_count(array: CFArrayRef) -> isize;
        #[link_name = "CFArrayGetValueAtIndex"]
        fn cf_array_get_value_at_index(array: CFArrayRef, index: isize) -> CFTypeRef;
        #[link_name = "CFDictionaryGetValue"]
        fn cf_dictionary_get_value(dictionary: CFDictionaryRef, key: CFTypeRef) -> CFTypeRef;
        #[link_name = "CFGetTypeID"]
        fn cf_get_type_id(value: CFTypeRef) -> usize;
        #[link_name = "CFNumberGetTypeID"]
        fn cf_number_get_type_id() -> usize;
        #[link_name = "CFBooleanGetTypeID"]
        fn cf_boolean_get_type_id() -> usize;
        #[link_name = "CFNumberGetValue"]
        fn cf_number_get_value(number: CFTypeRef, number_type: i32, value: *mut c_void) -> u8;
        #[link_name = "CFBooleanGetValue"]
        fn cf_boolean_get_value(value: CFTypeRef) -> u8;
        #[link_name = "CFEqual"]
        fn cf_equal(left: CFTypeRef, right: CFTypeRef) -> u8;
        #[link_name = "CFRunLoopGetCurrent"]
        fn cf_run_loop_get_current() -> CFRunLoopRef;
        #[link_name = "CFRunLoopAddSource"]
        fn cf_run_loop_add_source(
            run_loop: CFRunLoopRef,
            source: CFRunLoopSourceRef,
            mode: CFStringRef,
        );
        #[link_name = "CFRunLoopRunInMode"]
        fn cf_run_loop_run_in_mode(
            mode: CFStringRef,
            seconds: f64,
            return_after_source_handled: u8,
        ) -> i32;
        #[link_name = "kCFRunLoopDefaultMode"]
        static cf_run_loop_default_mode: CFStringRef;
    }

    struct CfOwned(CFTypeRef);

    impl Drop for CfOwned {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: CfOwned is constructed only from CoreFoundation Create/Copy
                // functions and therefore owns exactly one +1 reference.
                unsafe { cf_release(self.0) };
            }
        }
    }

    fn cf_string(value: &str) -> Option<CfOwned> {
        let value = CString::new(value).ok()?;
        // SAFETY: null allocator requests the default allocator; CString is valid UTF-8 bytes.
        let string = unsafe {
            cf_string_create_with_c_string(ptr::null(), value.as_ptr(), CF_STRING_ENCODING_UTF8)
        };
        (!string.is_null()).then_some(CfOwned(string))
    }

    struct AutoreleasePool(*mut c_void);

    impl AutoreleasePool {
        fn new() -> Self {
            // SAFETY: libobjc returns an opaque pool token valid until the matching pop.
            Self(unsafe { objc_autorelease_pool_push() })
        }
    }

    impl Drop for AutoreleasePool {
        fn drop(&mut self) {
            // SAFETY: the token was produced by objc_autorelease_pool_push in this scope.
            unsafe { objc_autorelease_pool_pop(self.0) };
        }
    }

    pub(super) fn idle_time() -> ObserverResult<IdleObservation> {
        // SAFETY: this CoreGraphics query is read-only and uses documented enum values.
        let seconds = unsafe {
            cg_event_source_seconds_since_last_event_type(
                CG_EVENT_SOURCE_COMBINED_SESSION_STATE,
                CG_ANY_INPUT_EVENT_TYPE,
            )
        };
        match normalize_idle_duration(seconds) {
            Ok(observation) => ObserverResult::Available(observation),
            Err(code) => ObserverResult::Error(code),
        }
    }

    unsafe fn send_object_message(receiver: *mut c_void, selector: &CStr) -> *mut c_void {
        if receiver.is_null() {
            return ptr::null_mut();
        }
        // SAFETY: selectors used by this module are zero-argument object-returning AppKit/
        // Foundation methods and the receiver type is checked by the call sequence.
        unsafe { objc_msg_send(receiver, sel_register_name(selector.as_ptr())) }
    }

    pub(super) fn active_application() -> ObserverResult<ActiveApplicationObservation> {
        let _pool = AutoreleasePool::new();
        let workspace_class = CString::new("NSWorkspace").expect("static class name");
        let shared_workspace = CString::new("sharedWorkspace").expect("static selector");
        let frontmost_application = CString::new("frontmostApplication").expect("static selector");
        let localized_name = CString::new("localizedName").expect("static selector");
        let utf8_string = CString::new("UTF8String").expect("static selector");

        // SAFETY: objc_get_class and the message sequence use stable AppKit/Foundation APIs.
        let class = unsafe { objc_get_class(workspace_class.as_ptr()) };
        if class.is_null() {
            return ObserverResult::Error(ObserverErrorCode::PlatformApiFailure);
        }
        // SAFETY: NSWorkspace.sharedWorkspace returns an NSWorkspace object.
        let workspace = unsafe { send_object_message(class, &shared_workspace) };
        // SAFETY: frontmostApplication returns an optional NSRunningApplication.
        let application = unsafe { send_object_message(workspace, &frontmost_application) };
        if application.is_null() {
            return ObserverResult::Unavailable(ObserverErrorCode::NoFrontmostApplication);
        }
        // SAFETY: localizedName returns an optional NSString.
        let name = unsafe { send_object_message(application, &localized_name) };
        if name.is_null() {
            return ObserverResult::Unavailable(ObserverErrorCode::NoFrontmostApplication);
        }
        // SAFETY: UTF8String returns a NUL-terminated pointer valid for the NSString lifetime.
        let utf8 = unsafe { send_object_message(name, &utf8_string) }.cast::<c_char>();
        if utf8.is_null() {
            return ObserverResult::Error(ObserverErrorCode::PlatformApiFailure);
        }
        // SAFETY: UTF8String is guaranteed to be NUL terminated while `name` is alive.
        let value = unsafe { CStr::from_ptr(utf8) }.to_string_lossy();
        let bounded: String = value.trim().chars().take(128).collect();
        if bounded.is_empty() {
            ObserverResult::Unavailable(ObserverErrorCode::NoFrontmostApplication)
        } else {
            ObserverResult::Available(ActiveApplicationObservation { name: bounded })
        }
    }

    unsafe fn read_cf_i32(dictionary: CFDictionaryRef, key: CFTypeRef) -> Option<i32> {
        // SAFETY: the dictionary is owned by the power-sources info blob and key is an
        // exported IOKit CFString constant.
        let value = unsafe { cf_dictionary_get_value(dictionary, key) };
        if value.is_null() || unsafe { cf_get_type_id(value) } != unsafe { cf_number_get_type_id() }
        {
            return None;
        }
        let mut result = 0_i32;
        // SAFETY: result points to writable i32 storage matching kCFNumberSInt32Type.
        let success = unsafe {
            cf_number_get_value(
                value,
                CF_NUMBER_SINT32_TYPE,
                (&mut result as *mut i32).cast::<c_void>(),
            )
        };
        (success != 0).then_some(result)
    }

    unsafe fn read_cf_bool(dictionary: CFDictionaryRef, key: CFTypeRef) -> Option<bool> {
        // SAFETY: dictionary/key lifetime and ownership are supplied by IOPowerSources.
        let value = unsafe { cf_dictionary_get_value(dictionary, key) };
        if value.is_null()
            || unsafe { cf_get_type_id(value) } != unsafe { cf_boolean_get_type_id() }
        {
            return None;
        }
        // SAFETY: value was type-checked as CFBoolean.
        Some(unsafe { cf_boolean_get_value(value) } != 0)
    }

    pub(super) fn battery_state() -> ObserverResult<BatteryObservation> {
        // SAFETY: Copy APIs return owned CoreFoundation objects released by CfOwned.
        let info = CfOwned(unsafe { iops_copy_power_sources_info() });
        if info.0.is_null() {
            return ObserverResult::Error(ObserverErrorCode::PlatformApiFailure);
        }
        // SAFETY: info is a valid IOPowerSources info blob.
        let list = CfOwned(unsafe { iops_copy_power_sources_list(info.0) }.cast::<c_void>());
        if list.0.is_null() {
            return ObserverResult::Error(ObserverErrorCode::PlatformApiFailure);
        }

        let Some(current_capacity_key) = cf_string("Current Capacity") else {
            return ObserverResult::Error(ObserverErrorCode::PlatformApiFailure);
        };
        let Some(max_capacity_key) = cf_string("Max Capacity") else {
            return ObserverResult::Error(ObserverErrorCode::PlatformApiFailure);
        };
        let Some(is_charging_key) = cf_string("Is Charging") else {
            return ObserverResult::Error(ObserverErrorCode::PlatformApiFailure);
        };
        let Some(type_key) = cf_string("Type") else {
            return ObserverResult::Error(ObserverErrorCode::PlatformApiFailure);
        };
        let Some(internal_battery_type) = cf_string("InternalBattery") else {
            return ObserverResult::Error(ObserverErrorCode::PlatformApiFailure);
        };

        // SAFETY: list is a valid CFArray returned by iops_copy_power_sources_list.
        let count = unsafe { cf_array_get_count(list.0) };
        for index in 0..count {
            // SAFETY: index is within the array's reported bounds.
            let source = unsafe { cf_array_get_value_at_index(list.0, index) };
            // SAFETY: source belongs to list and info remains alive for the description lifetime.
            let description = unsafe { iops_get_power_source_description(info.0, source) };
            if description.is_null() {
                continue;
            }
            // SAFETY: description is a valid CFDictionary and exported keys/types are stable.
            let source_type = unsafe { cf_dictionary_get_value(description, type_key.0) };
            if source_type.is_null()
                || unsafe { cf_equal(source_type, internal_battery_type.0) } == 0
            {
                continue;
            }

            // SAFETY: keys are expected to contain numeric/boolean values for internal batteries.
            let current = unsafe { read_cf_i32(description, current_capacity_key.0) };
            // SAFETY: same as above.
            let maximum = unsafe { read_cf_i32(description, max_capacity_key.0) };
            // SAFETY: same as above.
            let is_charging = unsafe { read_cf_bool(description, is_charging_key.0) };
            let (Some(current), Some(maximum), Some(is_charging)) = (current, maximum, is_charging)
            else {
                return ObserverResult::Error(ObserverErrorCode::InvalidValue);
            };
            return match normalize_battery_state(current, maximum, is_charging) {
                Ok(observation) => ObserverResult::Available(observation),
                Err(code) => ObserverResult::Error(code),
            };
        }

        ObserverResult::Unavailable(ObserverErrorCode::NoPowerSource)
    }

    struct PowerCallbackContext {
        sender: UnboundedSender<PowerEvent>,
        root_port: AtomicU32,
    }

    unsafe extern "C" fn power_callback(
        refcon: *mut c_void,
        _service: IoObject,
        message_type: u32,
        message_argument: *mut c_void,
    ) {
        if refcon.is_null() {
            return;
        }
        // SAFETY: refcon points to the boxed PowerCallbackContext retained by the observer thread.
        let context = unsafe { &*(refcon.cast::<PowerCallbackContext>()) };
        match message_type {
            IO_MESSAGE_CAN_SYSTEM_SLEEP => {
                let root_port = context.root_port.load(Ordering::SeqCst);
                if root_port != 0 {
                    // SAFETY: message_argument is the power-notification token documented by IOKit.
                    unsafe { io_allow_power_change(root_port, message_argument as isize) };
                }
            }
            IO_MESSAGE_SYSTEM_WILL_SLEEP => {
                let _ = context.sender.send(PowerEvent::Sleep);
                let root_port = context.root_port.load(Ordering::SeqCst);
                if root_port != 0 {
                    // SAFETY: the system sleep notification must be acknowledged with its token.
                    unsafe { io_allow_power_change(root_port, message_argument as isize) };
                }
            }
            IO_MESSAGE_SYSTEM_HAS_POWERED_ON => {
                let _ = context.sender.send(PowerEvent::Wake);
            }
            _ => {}
        }
    }

    fn cleanup_power_registration(
        root_port: IoConnect,
        notification_port: IONotificationPortRef,
        notifier: &mut IoObject,
    ) {
        // SAFETY: these handles were returned by io_register_for_system_power and are released once.
        unsafe {
            if *notifier != 0 {
                let _ = io_deregister_for_system_power(notifier);
            }
            if !notification_port.is_null() {
                io_notification_port_destroy(notification_port);
            }
            if root_port != 0 {
                let _ = io_service_close(root_port);
            }
        }
    }

    pub(super) fn start_power_events(
        sender: UnboundedSender<PowerEvent>,
    ) -> ObserverResult<SystemPowerObserver> {
        let running = Arc::new(AtomicBool::new(true));
        let thread_running = running.clone();
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let observer_thread = thread::spawn(move || {
            let context = Box::new(PowerCallbackContext {
                sender,
                root_port: AtomicU32::new(0),
            });
            let context_ptr = Box::into_raw(context);
            let mut notification_port: IONotificationPortRef = ptr::null_mut();
            let mut notifier = 0_u32;
            // SAFETY: context_ptr remains allocated for the complete registration lifetime.
            let root_port = unsafe {
                io_register_for_system_power(
                    context_ptr.cast::<c_void>(),
                    &mut notification_port,
                    power_callback,
                    &mut notifier,
                )
            };
            if root_port == 0 || notification_port.is_null() {
                cleanup_power_registration(root_port, notification_port, &mut notifier);
                // SAFETY: registration failed, so no run-loop callback retains context_ptr.
                unsafe { drop(Box::from_raw(context_ptr)) };
                let _ = ready_tx.send(Err(ObserverErrorCode::RegistrationFailed));
                return;
            }
            // SAFETY: context_ptr remains valid until after run-loop processing stops.
            unsafe { (*context_ptr).root_port.store(root_port, Ordering::SeqCst) };
            // SAFETY: notification_port is valid after successful registration.
            let source = unsafe { io_notification_port_get_run_loop_source(notification_port) };
            if source.is_null() {
                cleanup_power_registration(root_port, notification_port, &mut notifier);
                // SAFETY: no run-loop source was installed, so callbacks cannot race cleanup.
                unsafe { drop(Box::from_raw(context_ptr)) };
                let _ = ready_tx.send(Err(ObserverErrorCode::RegistrationFailed));
                return;
            }
            // SAFETY: this dedicated thread owns its current run loop for the observer lifetime.
            let run_loop = unsafe { cf_run_loop_get_current() };
            // SAFETY: source and default mode are valid CoreFoundation objects.
            unsafe { cf_run_loop_add_source(run_loop, source, cf_run_loop_default_mode) };
            let _ = ready_tx.send(Ok(()));

            while thread_running.load(Ordering::SeqCst) {
                // SAFETY: run the dedicated observer run loop in bounded slices so stop can join.
                unsafe { cf_run_loop_run_in_mode(cf_run_loop_default_mode, 0.25, 1) };
                thread::sleep(Duration::from_millis(10));
            }

            cleanup_power_registration(root_port, notification_port, &mut notifier);
            // SAFETY: the run loop has stopped and registration is torn down before freeing context.
            unsafe { drop(Box::from_raw(context_ptr)) };
        });

        match ready_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(())) => ObserverResult::Available(SystemPowerObserver {
                running,
                thread: Some(observer_thread),
            }),
            Ok(Err(code)) => {
                let _ = observer_thread.join();
                ObserverResult::Error(code)
            }
            Err(_) => {
                running.store(false, Ordering::SeqCst);
                drop(observer_thread);
                ObserverResult::Error(ObserverErrorCode::RegistrationFailed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::observation::{ObserverKind, ObserverStatus};

    #[test]
    fn idle_conversion_rejects_invalid_values_and_floors_seconds() {
        assert_eq!(
            normalize_idle_duration(f64::NAN),
            Err(ObserverErrorCode::InvalidValue)
        );
        assert_eq!(
            normalize_idle_duration(-1.0),
            Err(ObserverErrorCode::InvalidValue)
        );
        assert_eq!(normalize_idle_duration(12.9).unwrap().seconds, 12);
    }

    #[test]
    fn battery_conversion_is_bounded_and_rejects_invalid_capacity() {
        assert_eq!(
            normalize_battery_state(25, 100, false).unwrap(),
            BatteryObservation {
                level_percent: 25,
                is_charging: false
            }
        );
        assert_eq!(
            normalize_battery_state(110, 100, true)
                .unwrap()
                .level_percent,
            100
        );
        assert_eq!(
            normalize_battery_state(1, 0, false),
            Err(ObserverErrorCode::InvalidValue)
        );
    }

    #[test]
    fn opt_out_denies_active_app_before_platform_observation() {
        let result = SystemDesktopMonitor::get_active_application(false);
        assert_eq!(
            result.diagnostic(ObserverKind::ActiveApplication).status,
            ObserverStatus::Denied
        );
    }

    #[test]
    fn window_titles_are_always_unsupported_in_v1_even_if_legacy_setting_is_true() {
        let result = SystemDesktopMonitor::get_window_title(true);
        assert_eq!(
            result.diagnostic(ObserverKind::WindowTitle).status,
            ObserverStatus::Unsupported
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn unsupported_platform_never_fabricates_observer_values() {
        assert!(SystemDesktopMonitor::get_idle_time()
            .into_available()
            .is_none());
        assert!(SystemDesktopMonitor::get_battery_state()
            .into_available()
            .is_none());
        assert!(SystemDesktopMonitor::get_active_application(true)
            .into_available()
            .is_none());
    }
}
