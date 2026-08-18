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
    pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
        let host = cpal::default_host();
        let default_device_name = host.default_input_device().and_then(|d| d.name().ok());

        let mut list = Vec::new();
        if let Ok(devices) = host.input_devices() {
            for device in devices {
                if let Ok(name) = device.name() {
                    let is_default = default_device_name.as_deref() == Some(&name);
                    list.push(AudioDeviceInfo {
                        id: name.clone(),
                        name: name.clone(),
                        is_default,
                    });
                }
            }
        }

        if list.is_empty() {
            list.push(AudioDeviceInfo {
                id: "default_input".to_string(),
                name: "Default Microphone (Mock)".to_string(),
                is_default: true,
            });
        }

        list
    }

    pub fn list_output_devices() -> Vec<AudioDeviceInfo> {
        let host = cpal::default_host();
        let default_device_name = host.default_output_device().and_then(|d| d.name().ok());

        let mut list = Vec::new();
        if let Ok(devices) = host.output_devices() {
            for device in devices {
                if let Ok(name) = device.name() {
                    let is_default = default_device_name.as_deref() == Some(&name);
                    list.push(AudioDeviceInfo {
                        id: name.clone(),
                        name: name.clone(),
                        is_default,
                    });
                }
            }
        }

        if list.is_empty() {
            list.push(AudioDeviceInfo {
                id: "default_output".to_string(),
                name: "Default Speakers (Mock)".to_string(),
                is_default: true,
            });
        }

        list
    }
}
