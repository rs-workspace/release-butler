use hmac::{Hmac, Mac};
use octocrab::{params::repos::Reference, Octocrab};
use serde::Serialize;
use sha2::Sha256;
use tracing::error;

pub type HmacSha256 = Hmac<Sha256>;

pub fn generate_hmac_sha256_hex(body: &[u8], key: &[u8]) -> Option<String> {
    let mut hasher = HmacSha256::new_from_slice(key).expect("Failed to create Hasher");
    hasher.update(body);

    let mut enc_buf = [0u8; 256];
    let Ok(hex) = base16ct::lower::encode_str(&hasher.finalize().into_bytes(), &mut enc_buf) else {
        return None;
    };
    Some(hex.to_owned())
}

pub struct UpdateFiles<'a> {
    gh: &'a Octocrab,
    files: Vec<File>,
    ref_: Reference,
    commit_msg: String,
}

pub struct File {
    pub name: String,
    pub new_content: String,
}

impl<'a> UpdateFiles<'a> {
    pub fn new(gh: &'a Octocrab, files: Vec<File>, ref_: Reference, commit_msg: String) -> Self {
        Self {
            gh,
            files,
            ref_,
            commit_msg,
        }
    }

    pub async fn execute(self, owner: &str, repo: &str, base_commit_sha: &str) {
        #[derive(Serialize, Debug)]
        struct BlobsTree {
            path: String,
            mode: String,
            r#type: String,
            sha: serde_json::Value,
        }

        let mut blobs = Vec::new();

        for file in self.files {
            // Create the blob
            match self
                .gh
                .post::<serde_json::Value, serde_json::Value>(
                    format!("/repos/{}/{}/git/blobs", owner, repo),
                    Some(&serde_json::json!({
                        "content": file.new_content
                    })),
                )
                .await
            {
                Ok(mut res) => {
                    blobs.push(BlobsTree {
                        path: file.name,
                        mode: String::from("100644"),
                        r#type: String::from("blob"),
                        sha: res["sha"].take(),
                    });
                }
                Err(err) => {
                    error!(
                        "Failed to upload blob of file {} in repo {}/{}. Error: {}",
                        file.name, owner, repo, err
                    );
                    continue;
                }
            };
        }

        if blobs.is_empty() {
            return;
        }

        // Create a tree
        let Ok(tree_res) = self
            .gh
            .post::<serde_json::Value, serde_json::Value>(
                format!("/repos/{}/{}/git/trees", owner, repo),
                Some(&serde_json::json!({
                    "base_tree": base_commit_sha,
                    "tree": blobs
                })),
            )
            .await
        else {
            error!("Failed to create tree. blobs: {:?}", blobs);
            return;
        };
        let tree_sha = tree_res["sha"].as_str().unwrap_or_default();

        // Create a commit
        let commit_res = match self
            .gh
            .post::<serde_json::Value, serde_json::Value>(
                format!("/repos/{}/{}/git/commits", owner, repo),
                Some(&serde_json::json!({
                    "message": self.commit_msg,
                    "tree": tree_sha,
                    "parents": [base_commit_sha]
                })),
            )
            .await
        {
            Ok(res) => res,
            Err(err) => {
                error!("Failed to create commit! Error: {}", err);
                return;
            }
        };
        let commit_sha = commit_res["sha"].as_str().unwrap_or_default();

        // Check if the branch/reference already exists
        let repos = self.gh.repos(owner, repo);
        if repos.get_ref(&self.ref_).await.is_err() {
            // Create a branch/reference
            if let Err(err) = repos.create_ref(&self.ref_, commit_sha).await {
                error!("Failed to create the reference. Error: {}", err);
            }
        } else if let Err(err) = self
            .gh
            .patch::<serde_json::Value, String, serde_json::Value>(
                format!("/repos/{}/{}/git/refs/{}", owner, repo, self.ref_.ref_url()),
                Some(&serde_json::json!({
                    "sha": commit_sha,
                    "force": true
                })),
            )
            .await
        {
            error!(
                "URL: {}\nBody:{}",
                format!("/repos/{}/{}/git/refs/{}", owner, repo, self.ref_.ref_url()),
                serde_json::json!({
                    "sha": commit_sha,
                    "force": true
                })
            );
            error!("Failed to force update the ref. Error: {}", err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex() {
        let key = "abc123".as_bytes();
        let body = "Sample Payload".as_bytes();

        let expected =
            String::from("4a91576675ad4b18544e6108b9eaf06c4b5f799cf6ca9bde7ea83c04ec6eff7f"); // Generated from https://www.devglan.com/online-tools/hmac-sha256-online
        let actual = generate_hmac_sha256_hex(body, key).unwrap_or_default();

        assert_eq!(expected, actual)
    }

    #[cfg(feature = "tests")]
    #[test]
    fn test_json_signature() {
        let expected =
            String::from("4ed99f2f66b2328f8af4f8b56874e818033949dc87734b8ac5480c62829fa11a"); // Generated form https://www.devglan.com/online-tools/hmac-sha256-online
        let actual = generate_hmac_sha256_hex(
            crate::tests_utils::payload_template::GITHUB_PUSH,
            crate::tests_utils::DEFAULT_HMAC_KEY.as_bytes(),
        )
        .unwrap_or_default();

        assert_eq!(expected, actual);

        let expected_header_like = format!("sha256={}", expected);
        assert_eq!(
            expected_header_like,
            *crate::tests_utils::payload_template::GITHUB_PUSH_HEX
        )
    }
}
