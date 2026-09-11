pub mod legalize;

pub fn version() -> String {
     let version = option_env!("LEGALIZEGIT_HASH").unwrap_or(&"no hash");

     version.to_string()
}