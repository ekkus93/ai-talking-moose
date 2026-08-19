use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub struct AudioDeviceManager;

impl AudioDeviceManager {
    pub fn list_input_devices() -> Result<Vec<AudioDeviceInfo>, String> {
        let host = cpal::default_host();
        let default_device_name = host
            .default_input_device()
            .and_then(|device| device.name().ok());
        let devices = host.input_devices().map_err(|error_value| {
            format!("failed to enumerate audio input devices: {error_value}")
        })?;

        let mut list = Vec::new();
        for device in devices {
            if let Ok(name) = device.name() {
                let is_default = default_device_name.as_deref() == Some(&name);
                list.push(AudioDeviceInfo {
                    id: name.clone(),
                    name,
                    is_default,
                });
            }
        }
        Ok(list)
    }

    pub fn list_output_devices() -> Result<Vec<AudioDeviceInfo>, String> {
        let host = cpal::default_host();
        let default_device_name = host
            .default_output_device()
            .and_then(|device| device.name().ok());
        let devices = host.output_devices().map_err(|error_value| {
            format!("failed to enumerate audio output devices: {error_value}")
        })?;

        let mut list = Vec::new();
        for device in devices {
            if let Ok(name) = device.name() {
                let is_default = default_device_name.as_deref() == Some(&name);
                list.push(AudioDeviceInfo {
                    id: name.clone(),
                    name,
                    is_default,
                });
            }
        }
        Ok(list)
    }
}
