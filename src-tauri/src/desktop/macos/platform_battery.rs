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

