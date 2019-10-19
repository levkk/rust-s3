use std::env;
use dirs;
use ini::Ini;
use error::{err, S3Result};
use std::collections::HashMap;

/// AWS access credentials: access key, secret key, and optional token.
///
/// # Example
///
/// Loads from the standard AWS credentials file with the given profile name,
/// defaults to "default".
///
/// ```no_run
/// # // Do not execute this as it would cause unit tests to attempt to access
/// # // real user credentials.
/// use s3::credentials::Credentials;
///
/// // Load credentials from `[default]` profile
/// let credentials = Credentials::default();
///
/// // Also loads credentials from `[default]` profile
/// let credentials = Credentials::new(None, None, None, None);
///
/// // Load credentials from `[my-profile]` profile
/// let credentials = Credentials::new(None, None, None, Some("my-profile".into()));
/// ```
///
/// Credentials may also be initialized directly or by the following environment variables:
///
///   - `AWS_ACCESS_KEY_ID`,
///   - `AWS_SECRET_ACCESS_KEY`
///   - `AWS_SESSION_TOKEN`
///
/// The order of preference is arguments, then environment, and finally AWS
/// credentials file.
///
/// ```
/// use s3::credentials::Credentials;
///
/// // Load credentials directly
/// let access_key = String::from("AKIAIOSFODNN7EXAMPLE");
/// let secret_key = String::from("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
/// let credentials = Credentials::new(Some(access_key), Some(secret_key), None, None);
///
/// // Load credentials from the environment
/// use std::env;
/// env::set_var("AWS_ACCESS_KEY_ID", "AKIAIOSFODNN7EXAMPLE");
/// env::set_var("AWS_SECRET_ACCESS_KEY", "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
/// let credentials = Credentials::new(None, None, None, None);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Credentials {
    /// AWS public access key.
    pub access_key: String,
    /// AWS secret key.
    pub secret_key: String,
    /// Temporary token issued by AWS service.
    pub token: Option<String>,
    _private: (),
}

impl Credentials {
    /// Initialize Credentials directly with key ID, secret key, and optional
    /// token.
    pub fn new(access_key: Option<String>, secret_key: Option<String>, token: Option<String>, profile: Option<String>) -> Credentials {
        let credentials = match access_key {
            Some(key) => match secret_key {
                Some(secret) => {
                    match token {
                        Some(t) => {
                            Some(Credentials {
                                access_key: key,
                                secret_key: secret,
                                token: Some(t),
                                _private: (),
                            })
                        }
                        None => {
                            Some(Credentials {
                                access_key: key,
                                secret_key: secret,
                                token: None,
                                _private: (),
                            })
                        }
                    }
                }
                None => None
            }
            None => None
        };
        match credentials {
            Some(c) => c,
            None => match Credentials::from_env() {
                Ok(c) => c,
                Err(_) => match Credentials::from_profile(profile) {
                    Ok(c) => c,
                    Err(_) => match Credentials::from_instance_metadata() {
                        Ok(c) => c,
                        Err(e) => panic!("No credentials provided as arguments, in the environment or in the profile file. \n {}", e)
                    }
                }
            }
        }
    }

    fn from_env() -> S3Result<Credentials> {
        let access_key = env::var("AWS_ACCESS_KEY_ID")?;
        let secret_key = env::var("AWS_SECRET_ACCESS_KEY")?;
        let token = match env::var("AWS_SESSION_TOKEN") {
            Ok(x) => Some(x),
            Err(_) => None
        };
        Ok(Credentials { access_key, secret_key, token, _private: () })
    }

    fn from_instance_metadata() -> S3Result<Credentials> {
        let resp: HashMap<String, String> = reqwest::get("http://169.254.169.254/latest/meta-data/iam/info")?
            .json()?;
        let credentials = if let Some(arn) = resp.get("InstanceProfileArn") {
            if let Some(role) = arn.split('/').last() {
                let resp: HashMap<String, String> = reqwest::get(&format!("http://169.254.169.254/latest/meta-data/iam/security-credentials/{}", role))?
                    .json()?;
                let access_key = resp.get("AccessKeyId").unwrap().clone();
                let secret_key = resp.get("SecretAccessKey").unwrap().clone();
                let token = Some(resp.get("Token").unwrap().clone());
                Some(Credentials { access_key, secret_key, token, _private: ()})
            } else {
                None
            }
        } else {
            None
        };

        Ok(credentials.unwrap())
    }

    fn from_profile(section: Option<String>) -> S3Result<Credentials> {
        let home_dir = match dirs::home_dir() {
            Some(path) => Ok(path),
            None => Err(err("Invalid home dir")),
        };
        let profile = format!("{}/.aws/credentials", home_dir?.display());
        let conf = Ini::load_from_file(&profile)?;
        let section = match section {
            Some(s) => s,
            None => String::from("default")
        };
        let data1 = match conf.section(Some(section.clone())) {
            Some(d) => Ok(d),
            None => Err(err("Missing aws section"))
        };
        let data2 = match conf.section(Some(section.clone())) {
            Some(d) => Ok(d),
            None => Err(err("Missing aws section"))
        };
        let access_key = match data1?.get("aws_access_key_id") {
            Some(x) => Ok(x.to_owned()),
            None => Err(err("Missing aws section"))
        };
        let secret_key = match data2?.get("aws_secret_access_key") {
            Some(x) => Ok(x.to_owned()),
            None => Err(err("Missing aws section"))
        };
        Ok(Credentials { access_key: access_key?.to_owned(), secret_key: secret_key?.to_owned(), token: None, _private: () })
    }
}

impl Default for Credentials {
    fn default() -> Self {
        Credentials::new(None, None, None, None)
    }
}