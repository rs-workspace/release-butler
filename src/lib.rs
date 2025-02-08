use octocrab::{models::AppId, Octocrab};

pub mod common;
pub mod config;
pub mod events;
#[cfg(feature = "tests")]
pub mod tests_utils;
pub mod webhook;

pub static DEFAULT_CONFIG_FILE_PATH: &str = "release-butler.toml";
pub static CONFIG_ISSUE_LABEL: &str = "release-butler-config-error";

pub struct State {
    pub webhook_secret: String,
    pub app_username: String,
    pub app_id: AppId,
    pub key: jsonwebtoken::EncodingKey,
    pub gh: Octocrab,
}

impl State {
    pub fn new(
        webhook_secret: String,
        app_username: String,
        app_id: String,
        private_key: String,
    ) -> Self {
        let app_id = AppId(app_id.parse::<u64>().expect("Failed to parse app_id"));
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(private_key.as_bytes())
            .expect("Failed to generate EncodingKey");

        let gh = Octocrab::builder()
            .app(app_id, key.clone())
            .build()
            .expect("Failed to build Octocrab");

        Self {
            webhook_secret,
            app_username: format!("app/{}", app_username),
            app_id,
            key,
            gh,
        }
    }

    #[cfg(feature = "tests")]
    pub fn new_basic(webhook_secret: String) -> Self {
        // This is just a random dummy key, don't worry let it be here
        let dummy_rsa_key = r"-----BEGIN RSA PRIVATE KEY-----
            MIIEpAIBAAKCAQEAgQC0hB9NdFSU74N45z4xq58TtJg1qdBFEdHzexEmoFSDBBe7
            Sh88c3nKcQGtjFPaaD1N9ovFeshd5r49dqrp3djDFaUctMOkuVv2nlIA53JKaUm2
            yaUjmhLplSFuOqMmDv4e+ET+Uk6uDbc2MJQdTKcblsg9wiUYtkszgnvLEe1FxrNa
            b+7yJP4QLq2N3WsKDPNtFcapmVsnTHJkdj/T5Ms7IYediejNx6NjSCZcLmt+hgyc
            e5RjsGGqeWkj1LNPIaG31hWOb90LClQUln0neKvTzpVtC3r4zvfXf5GoZP22JbvM
            Q19bHM0DicMGaXVg1S1JGIVN2tudTSxGFcAyRQIDAQABAoIBABMM1ZuFO9zn+K5+
            DcaoSpF7hl9u8s3G8cw14uzTlY6rrEVYc9H4Vub+n0Sc6NIGOASYuQCll14QZL2E
            bnMtvieCsRxrK5gOJC8zQ3IRzgxftlliB1ozxtQj4tag/zQtj5s7L7ueBKiG8fEY
            kyoNVV5SdyKHI4eeDs4swMiOG2jkGKT0r8bGD2R4XtUIKl6zFpEebYNoV2+97V4q
            Z44JFZiBTmSK9qQXv7Q4eHogFLE4nA2AbXl6KLYFsLrTpBIbnZmFYf+mCOESK4OB
            LokzM286/zFcQvu9hnkaVcirUreaDj+NsS4C4cC0TO4aTVphLFftkJvhCrpePU6s
            1XdEQYECgYEA2BZFOnuJzzXj32jm1kb3NeNLqb0LMOUwKwWdcG6hzn3vs+6AcqgB
            ogjXZCcRHArY4MyKaHvJCh8vOoELk1Cxe2I8cvhMQKWGN9xFNKyzYaHmHdujzvZq
            j4vBVB4stdTseNkd7Y5FqZBtiHqtoyqxXVsphsQWKm2LA5s5nJ+r8JECgYEAmNSi
            RK8Yhx6Clmp9x2+VLWfKZexjyb/t58zyvswVfKg47pOllQgkVStnMm4K9zQY2fqR
            M68t2egAQ9XBvW/3CuVas3XUgWUbZhn9QbjA68/vtHVP/P24X8a1q3RpCY0jH/K8
            N5fX1b7J4Rld/FRxM1M9JSsSpaNbS6NlTNMXQHUCgYEAoD77veAJlcHYKECqFzPv
            dmYGIW1RFESSkQUL+WoB0pkwHtZ7KQwQkfJOkTYriQk+Ro9JASzzLO9tXcx/IhNQ
            WzjBrV0XZ0WZIGnYZLTCHmAqv++3Le8tnSA+EbyC2aF6cDBK8nV0kcfKgtC/XeZ2
            O840IH3gFjzAP79oXQ9IOhECgYEAgZqotWhre4KTKa3LVoq4zlWbXY33HctGnHHA
            Va9KdXlPNns9S0IpVZTGIg0R/YtPm+MSqergDk/hkaU/dD/0F2hi35eIC+dLMe3O
            SKK97/xZggaOO7SKW6Zuv6Srwq7O37QAi4CYR6pRFzRk8KxHh0gKrW92k8MRk/ZP
            3LOSn1UCgYBc7plV8yhXXaGGMPYGMTv1xQiuo/gLbksiEVSBFAmMqiBxExXK7PdS
            H0XhC7y+kquqq+pwgHPeVdiwmqwWCSFyUm0uqUpAh3LH786UCMu7MAQlSfSw6/by
            MCUo4Itp4U2eQPav/61C64G//DFraJsZpWn0RVgvmydpPlntABRthw==
            -----END RSA PRIVATE KEY-----";

        Self {
            webhook_secret,
            app_username: String::new(),
            app_id: AppId(0),
            key: jsonwebtoken::EncodingKey::from_rsa_pem(dummy_rsa_key.as_bytes()).unwrap(),
            gh: Octocrab::builder()
                .base_uri("http://127.0.0.1:1111/dummy")
                .unwrap()
                .build()
                .unwrap(),
        }
    }
}
