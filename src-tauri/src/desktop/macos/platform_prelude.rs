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

