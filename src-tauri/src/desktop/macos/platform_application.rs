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

