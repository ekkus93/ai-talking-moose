include!("macos/common.rs");
include!("macos/non_macos.rs");

#[cfg(target_os = "macos")]
mod platform {
    include!("macos/platform_prelude.rs");
    include!("macos/platform_application.rs");
    include!("macos/platform_battery.rs");
    include!("macos/platform_power.rs");
}

include!("macos/tests.rs");
