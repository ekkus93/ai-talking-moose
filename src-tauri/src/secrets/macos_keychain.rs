use crate::secrets::store::SecretBackend;
use security_framework::passwords::{
    delete_generic_password, generic_password, set_generic_password, PasswordOptions,
};
use security_framework_sys::base::errSecItemNotFound;

const KEYCHAIN_SERVICE: &str = "com.talkingmoose.ai";
const GOOGLE_API_KEY_ACCOUNT: &str = "google-api-key";

pub(crate) struct MacOsKeychainBackend;

impl SecretBackend for MacOsKeychainBackend {
    fn read_google_api_key(&self) -> Result<Option<String>, String> {
        let options =
            PasswordOptions::new_generic_password(KEYCHAIN_SERVICE, GOOGLE_API_KEY_ACCOUNT);
        match generic_password(options) {
            Ok(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| "Google API key in macOS Keychain is not valid UTF-8".to_string()),
            Err(error) if error.code() == errSecItemNotFound => Ok(None),
            Err(error) => Err(format!(
                "failed to read Google API key from macOS Keychain: {error}"
            )),
        }
    }

    fn write_google_api_key(&self, key: &str) -> Result<(), String> {
        set_generic_password(KEYCHAIN_SERVICE, GOOGLE_API_KEY_ACCOUNT, key.as_bytes())
            .map_err(|error| format!("failed to store Google API key in macOS Keychain: {error}"))
    }

    fn delete_google_api_key(&self) -> Result<(), String> {
        match delete_generic_password(KEYCHAIN_SERVICE, GOOGLE_API_KEY_ACCOUNT) {
            Ok(()) => Ok(()),
            Err(error) if error.code() == errSecItemNotFound => Ok(()),
            Err(error) => Err(format!(
                "failed to delete Google API key from macOS Keychain: {error}"
            )),
        }
    }
}
